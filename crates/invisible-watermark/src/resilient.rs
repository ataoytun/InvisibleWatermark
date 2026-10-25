use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::{ChaCha20, Nonce};
use chacha20poly1305::Key;
use image::DynamicImage;

use super::{Error, MAGIC, argon2id, color_buffer, shuffle, with_samples};

/// Shortest side in pixels an image needs for [`embed_resilient`].
pub const RESILIENT_MIN_SIDE: u32 = 512;
/// Longest text in bytes the resilient mode holds.
pub const RESILIENT_MAX_TEXT: usize = 32;

const SIZE: usize = 256;
const SIDE_BLOCKS: usize = SIZE / 8;
const BLOCKS: usize = SIDE_BLOCKS * SIDE_BLOCKS;
const STEP: f32 = 16.0;
const COEFFICIENTS: [(usize, usize); 4] = [(1, 1), (1, 2), (2, 1), (2, 2)];
const RECORD_LEN: usize = 46;
const STEPS: usize = RECORD_LEN * 8 + 6;
const CODED_LEN: usize = 2 * STEPS;
const MODE_PLAIN: u8 = 0x31;
const MODE_ENCRYPTED: u8 = 0x32;
const SALT_LEN: usize = 4;
const LAYOUT_KEY: [u8; 32] = *b"iwm v3 public layout key\0\0\0\0\0\0\0\0";
const GENERATOR_0: u32 = 0o171;
const GENERATOR_1: u32 = 0o133;

/// Embeds `text` so that it survives JPEG compression and resizing. Returns an 8-bit copy like [`embed`](crate::embed), alpha unchanged.
pub fn embed_resilient(image: DynamicImage, text: &str) -> Result<DynamicImage, Error> {
    embed(image, text, None)
}

/// Like [`embed_resilient`], with the text encrypted under `password`.
pub fn embed_resilient_with_password(
    image: DynamicImage,
    text: &str,
    password: &str,
) -> Result<DynamicImage, Error> {
    let mut salt = [0; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|_| Error::RandomSource)?;
    embed(image, text, Some((password, salt)))
}

/// Reads the text of a resilient mark made without a password. Pass the image after EXIF orientation, as it was embedded.
pub fn extract_resilient(image: &DynamicImage) -> Result<String, Error> {
    extract(image, None)
}

/// Reads and decrypts the text of a resilient mark made with `password`.
pub fn extract_resilient_with_password(
    image: &DynamicImage,
    password: &str,
) -> Result<String, Error> {
    if password.is_empty() {
        return Err(Error::EmptyPassword);
    }
    extract(image, Some(password))
}

fn embed(
    image: DynamicImage,
    text: &str,
    password: Option<(&str, [u8; SALT_LEN])>,
) -> Result<DynamicImage, Error> {
    if matches!(password, Some(("", _))) {
        return Err(Error::EmptyPassword);
    }
    if text.len() > RESILIENT_MAX_TEXT {
        return Err(Error::TextTooLong { len: text.len() });
    }
    let (width, height) = (image.width(), image.height());
    let short_side = width.min(height);
    if short_side < RESILIENT_MIN_SIDE {
        return Err(Error::ImageTooSmall { short_side });
    }
    let record = build_record(text, password)?;
    let bits = block_bits(&encode(&record));
    Ok(with_samples(image, |samples, channels| {
        mark(samples, channels, width as usize, height as usize, &bits);
    }))
}

fn build_record(
    text: &str,
    password: Option<(&str, [u8; SALT_LEN])>,
) -> Result<[u8; RECORD_LEN], Error> {
    let mut record = [0; RECORD_LEN];
    record[0] = if password.is_some() {
        MODE_ENCRYPTED
    } else {
        MODE_PLAIN
    };
    if let Some((_, salt)) = password {
        record[1..5].copy_from_slice(&salt);
    }
    record[5] = text.len() as u8;
    record[6..6 + text.len()].copy_from_slice(text.as_bytes());
    let inner = crc32fast::hash(&record[..38]);
    record[38..42].copy_from_slice(&inner.to_be_bytes());
    if let Some((password, salt)) = password {
        apply_cipher(&cipher_key(password, salt)?, &mut record[5..42]);
    }
    let outer = crc32fast::hash(&record[..42]);
    record[42..].copy_from_slice(&outer.to_be_bytes());
    Ok(record)
}

fn cipher_key(password: &str, salt: [u8; SALT_LEN]) -> Result<[u8; 32], Error> {
    let mut argon_salt = [0; 8];
    argon_salt[..4].copy_from_slice(&MAGIC);
    argon_salt[4..].copy_from_slice(&salt);
    let mut key = [0; 32];
    argon2id(password, &argon_salt, &mut key)?;
    Ok(key)
}

fn apply_cipher(key: &[u8; 32], bytes: &mut [u8]) {
    ChaCha20::new(&Key::from(*key), &Nonce::from([0; 12])).apply_keystream(bytes);
}

fn parse_record(mut record: [u8; RECORD_LEN], password: Option<&str>) -> Result<String, Error> {
    let outer = u32::from_be_bytes([record[42], record[43], record[44], record[45]]);
    if outer != crc32fast::hash(&record[..42]) {
        return Err(Error::NoWatermark);
    }
    let encrypted = match (record[0], password) {
        (MODE_PLAIN, Some(_)) => return Err(Error::NotEncrypted),
        (MODE_PLAIN, None) => false,
        (MODE_ENCRYPTED, None) => return Err(Error::PasswordRequired),
        (MODE_ENCRYPTED, Some(password)) => {
            let salt = [record[1], record[2], record[3], record[4]];
            apply_cipher(&cipher_key(password, salt)?, &mut record[5..42]);
            true
        }
        (mode, _) => return Err(Error::UnsupportedVersion(mode)),
    };
    let damaged = || {
        if encrypted {
            Error::WrongPasswordOrDamaged
        } else {
            Error::Damaged
        }
    };
    let len = usize::from(record[5]);
    if len > RESILIENT_MAX_TEXT {
        return Err(damaged());
    }
    let (text, padding) = record[6..38].split_at(len);
    if padding.iter().any(|&byte| byte != 0) {
        return Err(damaged());
    }
    let inner = u32::from_be_bytes([record[38], record[39], record[40], record[41]]);
    if inner != crc32fast::hash(&record[..38]) {
        return Err(damaged());
    }
    String::from_utf8(text.to_vec()).map_err(|_| damaged())
}

fn parity(value: u32) -> u8 {
    (value.count_ones() & 1) as u8
}

fn encode(record: &[u8; RECORD_LEN]) -> [u8; CODED_LEN] {
    let bits = record
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1))
        .chain([0; 6]);
    let mut coded = [0; CODED_LEN];
    let mut state = 0;
    for (step, bit) in bits.enumerate() {
        let register = (u32::from(bit) << 6) | state;
        coded[2 * step] = parity(register & GENERATOR_0);
        coded[2 * step + 1] = parity(register & GENERATOR_1);
        state = register >> 1;
    }
    coded
}

/// Soft-decision Viterbi decoder. A positive soft value votes for a coded 0, a negative one for a 1.
fn viterbi(soft: &[f32; CODED_LEN]) -> [u8; RECORD_LEN] {
    let mut metric = [f32::NEG_INFINITY; 64];
    metric[0] = 0.0;
    let mut from_odd = [0u64; STEPS];
    for (step, decisions) in from_odd.iter_mut().enumerate() {
        let (s0, s1) = (soft[2 * step], soft[2 * step + 1]);
        let mut next = [f32::NEG_INFINITY; 64];
        for (state, best) in next.iter_mut().enumerate() {
            let bit = (state >> 5) as u32;
            for lsb in 0..2 {
                let previous = ((state << 1) & 63) | lsb;
                let register = (bit << 6) | previous as u32;
                let vote0 = if parity(register & GENERATOR_0) == 0 {
                    s0
                } else {
                    -s0
                };
                let vote1 = if parity(register & GENERATOR_1) == 0 {
                    s1
                } else {
                    -s1
                };
                let candidate = metric[previous] + vote0 + vote1;
                if candidate > *best {
                    *best = candidate;
                    if lsb == 1 {
                        *decisions |= 1 << state;
                    }
                }
            }
        }
        metric = next;
    }
    let mut bits = [0u8; STEPS];
    let mut state = 0;
    for step in (0..STEPS).rev() {
        bits[step] = (state >> 5) as u8;
        state = ((state << 1) & 63) | ((from_odd[step] >> state) & 1) as usize;
    }
    let mut record = [0; RECORD_LEN];
    for (index, &bit) in bits.iter().take(RECORD_LEN * 8).enumerate() {
        record[index / 8] |= bit << (7 - index % 8);
    }
    record
}

fn layout() -> Vec<usize> {
    shuffle(&LAYOUT_KEY, BLOCKS, BLOCKS)
}

fn block_bits(coded: &[u8; CODED_LEN]) -> [u8; BLOCKS] {
    let mut bits = [0; BLOCKS];
    for (j, &block) in layout().iter().enumerate() {
        bits[block] = coded[j % CODED_LEN];
    }
    bits
}

fn dither() -> Vec<f32> {
    let mut nonce = [0; 12];
    nonce[0] = 1;
    let mut bytes = vec![0; 4 * BLOCKS * 4];
    ChaCha20::new(&Key::from(LAYOUT_KEY), &Nonce::from(nonce)).apply_keystream(&mut bytes);
    let (words, _) = bytes.as_chunks::<4>();
    words
        .iter()
        .map(|&word| (u32::from_le_bytes(word) >> 8) as f32 / 16_777_216.0)
        .collect()
}

fn dct_matrix() -> [[f32; 8]; 8] {
    let mut matrix = [[0.0; 8]; 8];
    for (k, row) in matrix.iter_mut().enumerate() {
        let scale = if k == 0 { (1.0f64 / 8.0).sqrt() } else { 0.5 };
        for (i, value) in row.iter_mut().enumerate() {
            let angle = std::f64::consts::PI * ((2 * i + 1) * k) as f64 / 16.0;
            *value = (scale * angle.cos()) as f32;
        }
    }
    matrix
}

/// Orthonormal 2D DCT-II of a row-major 8x8 block: afterwards `block[row * 8 + col]` is the vertical frequency `row`, horizontal `col`.
fn forward(block: &mut [f32; 64], matrix: &[[f32; 8]; 8]) {
    let mut rows = [0.0; 64];
    for r in 0..8 {
        for k in 0..8 {
            rows[r * 8 + k] = (0..8).map(|i| matrix[k][i] * block[r * 8 + i]).sum();
        }
    }
    for k in 0..8 {
        for col in 0..8 {
            block[k * 8 + col] = (0..8).map(|r| matrix[k][r] * rows[r * 8 + col]).sum();
        }
    }
}

fn inverse(block: &mut [f32; 64], matrix: &[[f32; 8]; 8]) {
    let mut rows = [0.0; 64];
    for r in 0..8 {
        for col in 0..8 {
            rows[r * 8 + col] = (0..8).map(|k| matrix[k][r] * block[k * 8 + col]).sum();
        }
    }
    for r in 0..8 {
        for i in 0..8 {
            block[r * 8 + i] = (0..8).map(|k| matrix[k][i] * rows[r * 8 + k]).sum();
        }
    }
}

fn luma(samples: &[u8], channels: usize) -> Vec<f32> {
    samples
        .chunks_exact(channels)
        .map(|pixel| {
            0.299 * f32::from(pixel[0]) + 0.587 * f32::from(pixel[1]) + 0.114 * f32::from(pixel[2])
        })
        .collect()
}

/// The source samples that one output sample of the triangle kernel reads, and their weight sum.
struct Window {
    first: i64,
    last: i64,
    center: f64,
    support: f64,
    sum: f64,
}

impl Window {
    fn new(src: usize, dst: usize, i: usize) -> Window {
        let scale = src as f64 / dst as f64;
        let support = scale.max(1.0);
        let center = (i as f64 + 0.5) * scale - 0.5;
        let mut window = Window {
            first: (center - support).ceil() as i64,
            last: (center + support).floor() as i64,
            center,
            support,
            sum: 0.0,
        };
        for j in window.first..=window.last {
            let weight = window.tent(j);
            if weight > 0.0 {
                window.sum += weight;
            }
        }
        window
    }

    fn tent(&self, j: i64) -> f64 {
        1.0 - ((j as f64 - self.center) / self.support).abs()
    }

    /// The normalized weight of source sample `j`, before clamping to the edge, if the kernel reaches it.
    fn weight(&self, j: i64) -> Option<f32> {
        let weight = self.tent(j);
        (weight > 0.0).then(|| weight as f32 / self.sum as f32)
    }
}

struct Taps {
    offsets: Vec<usize>,
    index: Vec<usize>,
    weight: Vec<f32>,
}

fn taps(src: usize, dst: usize) -> Taps {
    let mut taps = Taps {
        offsets: vec![0],
        index: Vec::new(),
        weight: Vec::new(),
    };
    for i in 0..dst {
        let window = Window::new(src, dst, i);
        for j in window.first..=window.last {
            if let Some(weight) = window.weight(j) {
                taps.index.push(j.clamp(0, src as i64 - 1) as usize);
                taps.weight.push(weight);
            }
        }
        taps.offsets.push(taps.weight.len());
    }
    taps
}

/// Separable triangle-kernel resampling, widened when downscaling. Source rows are resampled
/// one at a time and added to the output rows that read them, so memory stays near the output size.
fn resample(
    src: &[f32],
    src_width: usize,
    src_height: usize,
    dst_width: usize,
    dst_height: usize,
) -> Vec<f32> {
    let horizontal = taps(src_width, dst_width);
    let windows: Vec<Window> = (0..dst_height)
        .map(|y| Window::new(src_height, dst_height, y))
        .collect();
    let last_row = src_height as i64 - 1;
    let mut out = vec![0.0; dst_width * dst_height];
    let mut row = vec![0.0; dst_width];
    let mut pending = 0;
    for (i, source) in src.chunks_exact(src_width).enumerate() {
        for (x, value) in row.iter_mut().enumerate() {
            let range = horizontal.offsets[x]..horizontal.offsets[x + 1];
            let indices = &horizontal.index[range.clone()];
            *value = indices
                .iter()
                .zip(&horizontal.weight[range])
                .map(|(&k, &w)| w * source[k])
                .sum();
        }
        let i = i as i64;
        while windows
            .get(pending)
            .is_some_and(|window| window.last.min(last_row) < i)
        {
            pending += 1;
        }
        for (window, dst) in windows
            .iter()
            .zip(out.chunks_exact_mut(dst_width))
            .skip(pending)
        {
            if window.first.max(0) > i {
                break;
            }
            let from = if i == 0 { window.first } else { i };
            let to = if i == last_row { window.last } else { i };
            for j in from..=to {
                if let Some(weight) = window.weight(j) {
                    for (value, &sample) in dst.iter_mut().zip(&row) {
                        *value += weight * sample;
                    }
                }
            }
        }
    }
    out
}

fn mark(samples: &mut [u8], channels: usize, width: usize, height: usize, bits: &[u8; BLOCKS]) {
    let plane = resample(&luma(samples, channels), width, height, SIZE, SIZE);
    let mut change = plane.clone();
    embed_bits(&mut change, bits);
    for (changed, original) in change.iter_mut().zip(&plane) {
        *changed -= original;
    }
    let upsampled = resample(&change, SIZE, SIZE, width, height);
    for (pixel, delta) in samples.chunks_exact_mut(channels).zip(upsampled) {
        for sample in &mut pixel[..3] {
            *sample = (f32::from(*sample) + delta).round().clamp(0.0, 255.0) as u8;
        }
    }
}

fn load_block(plane: &[f32], block: usize) -> [f32; 64] {
    let (x0, y0) = (block % SIDE_BLOCKS * 8, block / SIDE_BLOCKS * 8);
    let mut values = [0.0; 64];
    for (r, row) in values.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        row.copy_from_slice(&plane[(y0 + r) * SIZE + x0..][..8]);
    }
    values
}

fn store_block(plane: &mut [f32], block: usize, values: &[f32; 64]) {
    let (x0, y0) = (block % SIDE_BLOCKS * 8, block / SIDE_BLOCKS * 8);
    for (r, row) in values.as_chunks::<8>().0.iter().enumerate() {
        plane[(y0 + r) * SIZE + x0..][..8].copy_from_slice(row);
    }
}

fn embed_bits(plane: &mut [f32], bits: &[u8; BLOCKS]) {
    let matrix = dct_matrix();
    let dither = dither();
    for (block, &bit) in bits.iter().enumerate() {
        let mut values = load_block(plane, block);
        forward(&mut values, &matrix);
        let half = f32::from(bit) * 0.5;
        for (k, &(row, col)) in COEFFICIENTS.iter().enumerate() {
            let u = dither[4 * block + k];
            let v = values[row * 8 + col];
            values[row * 8 + col] = ((v / STEP - u - half).round() + u + half) * STEP;
        }
        inverse(&mut values, &matrix);
        store_block(plane, block, &values);
    }
}

fn block_soft(plane: &[f32]) -> Vec<f32> {
    let matrix = dct_matrix();
    let dither = dither();
    (0..BLOCKS)
        .map(|block| {
            let mut values = load_block(plane, block);
            forward(&mut values, &matrix);
            let sum: f32 = COEFFICIENTS
                .iter()
                .enumerate()
                .map(|(k, &(row, col))| {
                    let x = values[row * 8 + col] / STEP - dither[4 * block + k];
                    let r = x - x.floor();
                    1.0 - 4.0 * r.min(1.0 - r)
                })
                .sum();
            sum / COEFFICIENTS.len() as f32
        })
        .collect()
}

fn extract(image: &DynamicImage, password: Option<&str>) -> Result<String, Error> {
    let (width, height) = (image.width() as usize, image.height() as usize);
    if width == 0 || height == 0 {
        return Err(Error::NoWatermark);
    }
    let (samples, channels) = color_buffer(image);
    let plane = resample(&luma(&samples, channels), width, height, SIZE, SIZE);
    let block_soft = block_soft(&plane);
    let mut soft = [0.0; CODED_LEN];
    for (j, &block) in layout().iter().enumerate() {
        soft[j % CODED_LEN] += block_soft[block];
    }
    parse_record(viterbi(&soft), password)
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    /// A smooth color gradient with seeded noise, so the image behaves like a photo.
    fn photo(width: u32, height: u32) -> RgbImage {
        let mut state = 0x2545_F491_u32;
        RgbImage::from_fn(width, height, |x, y| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = (state % 17) as f32 - 8.0;
            let base = 40.0 + 140.0 * x as f32 / width as f32 + 30.0 * y as f32 / height as f32;
            let channel = |offset: f32| (base + offset + noise).round() as u8;
            Rgb([channel(20.0), channel(0.0), channel(-25.0)])
        })
    }

    #[test]
    fn dct_round_trip_recovers_block() {
        let matrix = dct_matrix();
        let original: [f32; 64] = std::array::from_fn(|i| ((i * 7919) % 256) as f32);
        let mut block = original;
        forward(&mut block, &matrix);
        inverse(&mut block, &matrix);
        for (a, b) in block.iter().zip(&original) {
            assert!((a - b).abs() < 1e-3);
        }
    }

    #[test]
    fn resample_to_same_size_is_identity() {
        let src: Vec<f32> = (0..40 * 30).map(|i| ((i * 7919) % 256) as f32).collect();
        let out = resample(&src, 40, 30, 40, 30);
        for (a, b) in out.iter().zip(&src) {
            assert!((a - b).abs() < 1e-4);
        }
    }

    #[test]
    fn resample_keeps_constant_plane_constant() {
        let src = vec![123.5f32; 64 * 48];
        for (width, height) in [(32, 24), (48, 36), (128, 96), (512, 512), (20, 100)] {
            let out = resample(&src, 64, 48, width, height);
            assert_eq!(out.len(), width * height);
            assert!(
                out.iter().all(|v| (v - 123.5).abs() < 1e-3),
                "{width}x{height}"
            );
        }
    }

    #[test]
    fn viterbi_decodes_noiseless_codeword() {
        let record = build_record("Example Studio", None).unwrap();
        let soft = encode(&record).map(|bit| if bit == 0 { 1.0 } else { -1.0 });
        assert_eq!(viterbi(&soft), record);
    }

    #[test]
    fn known_answer_output_is_stable() {
        let image = DynamicImage::ImageRgb8(photo(768, 512));
        let marked = embed(image, "Example Studio 2026", None).unwrap();
        assert_eq!(crc32fast::hash(marked.as_bytes()), 0x617A_B12F);
    }

    #[test]
    fn known_answer_encrypted_output_is_stable() {
        let image = DynamicImage::ImageRgb8(photo(768, 512));
        let password = Some(("correct horse battery staple", [7, 7, 7, 7]));
        let marked = embed(image, "Example Studio 2026", password).unwrap();
        assert_eq!(crc32fast::hash(marked.as_bytes()), 0xA243_66F2);
    }

    fn with_outer_checksum(mut record: [u8; RECORD_LEN]) -> [u8; RECORD_LEN] {
        let outer = crc32fast::hash(&record[..42]);
        record[42..].copy_from_slice(&outer.to_be_bytes());
        record
    }

    fn with_checksums(mut record: [u8; RECORD_LEN]) -> [u8; RECORD_LEN] {
        let inner = crc32fast::hash(&record[..38]);
        record[38..42].copy_from_slice(&inner.to_be_bytes());
        with_outer_checksum(record)
    }

    #[test]
    fn parse_record_reports_unknown_mode_as_unsupported_version() {
        let mut record = build_record("hi", None).unwrap();
        record[0] = 0x33;
        assert_eq!(
            parse_record(with_checksums(record), None),
            Err(Error::UnsupportedVersion(0x33))
        );
    }

    #[test]
    fn parse_record_reports_damage_behind_a_valid_outer_checksum() {
        let record = build_record("hi", None).unwrap();
        let edited = |index: usize, value: u8| {
            let mut edited = record;
            edited[index] = value;
            edited
        };
        let cases = [
            with_checksums(edited(5, 33)),
            with_checksums(edited(8, b'x')),
            with_outer_checksum(edited(38, record[38] ^ 1)),
            with_checksums(edited(6, 0xFF)),
        ];
        for case in cases {
            assert_eq!(parse_record(case, None), Err(Error::Damaged));
        }
    }

    #[test]
    fn extract_reads_a_one_pixel_wide_image_of_two_million_rows() {
        let image = DynamicImage::ImageRgb8(RgbImage::new(1, 2_000_000));
        assert_eq!(extract(&image, None), Err(Error::NoWatermark));
    }
}
