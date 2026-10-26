//! Hides a short text watermark in an image and reads it back, either in the least significant bits or in a resilient mode that survives JPEG compression and resizing.
//!
//! The crate works on decoded images and enables no codecs of [`image`], which it re-exports as part of its API. To load and save files, depend on `image` 0.25 yourself with the formats you need, such as `png` or `jpeg`.
//!
//! ```
//! use invisible_watermark::image::{DynamicImage, RgbImage};
//!
//! let image = DynamicImage::ImageRgb8(RgbImage::new(64, 64));
//! let marked = invisible_watermark::embed(image, b"hello")?;
//! assert_eq!(invisible_watermark::extract(&marked)?, b"hello");
//! # Ok::<(), invisible_watermark::Error>(())
//! ```
//!
//! With a password the payload is encrypted and its bits are scattered across the image:
//!
//! ```
//! use invisible_watermark::image::{DynamicImage, RgbImage};
//!
//! let image = DynamicImage::ImageRgb8(RgbImage::new(64, 64));
//! let marked = invisible_watermark::embed_with_password(image, b"hello", "correct horse")?;
//! assert_eq!(invisible_watermark::extract_with_password(&marked, "correct horse")?, b"hello");
//! # Ok::<(), invisible_watermark::Error>(())
//! ```
//!
//! The resilient mode holds up to 32 bytes of text and needs at least 512 pixels on the short side:
//!
//! ```
//! use invisible_watermark::image::{DynamicImage, Rgb, RgbImage};
//!
//! let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(512, 512, Rgb([128, 128, 128])));
//! let marked = invisible_watermark::embed_resilient(image, "hello")?;
//! assert_eq!(invisible_watermark::extract_resilient(&marked)?, "hello");
//! # Ok::<(), invisible_watermark::Error>(())
//! ```
//!
//! [`read_watermark`] reads either kind, as the `iwm` command-line tool does. Pass it the EXIF orientation from the decoder: the LSB formats are read in stored pixel order and the resilient format after orientation.
//!
//! ```
//! use invisible_watermark::Watermark;
//! use invisible_watermark::image::metadata::Orientation;
//! use invisible_watermark::image::{DynamicImage, Rgb, RgbImage};
//!
//! let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(512, 512, Rgb([128, 128, 128])));
//! let marked = invisible_watermark::embed_resilient(image, "hello")?;
//! let watermark = invisible_watermark::read_watermark(&marked, Orientation::NoTransforms, None)?;
//! assert_eq!(watermark, Watermark::Resilient("hello".to_owned()));
//! # Ok::<(), invisible_watermark::Error>(())
//! ```
//!
//! Formats v1, v2 and v3 are stable: every 1.x release reads them, and known-answer tests guard the bytes they produce. A later LSB format gets a new version byte after the magic, and a later resilient format a new mode byte inside the v3 record, so older releases report it as [`Error::UnsupportedVersion`] instead of misreading it. A resilient format with a different layout reads as [`Error::NoWatermark`]. The API follows semantic versioning, and [`image`] is part of it: a breaking release of `image`, such as 0.26, means a new major version of this crate. The minimum supported Rust version may rise in a minor release.

#![warn(missing_docs)]

/// Re-exported so callers build images with the same version of `image`.
pub use image;

use std::borrow::Cow;
use std::collections::HashMap;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::{ChaCha20, Nonce};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use image::DynamicImage;
use image::metadata::Orientation;

mod resilient;

pub use resilient::{
    RESILIENT_MAX_TEXT, RESILIENT_MIN_SIDE, embed_resilient, embed_resilient_with_password,
    extract_resilient, extract_resilient_with_password,
};

const MAGIC: [u8; 4] = *b"IWMK";
const V1: u8 = 1;
const V2: u8 = 2;
const HEADER_LEN: usize = 9;
const CRC_LEN: usize = 4;
const OVERHEAD: usize = HEADER_LEN + CRC_LEN;
const SALT_LEN: usize = 16;
const PREFIX_LEN: usize = 5 + SALT_LEN;
const PREFIX_BITS: usize = PREFIX_LEN * 8;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const PASSWORD_OVERHEAD: usize = PREFIX_LEN + 4 + NONCE_LEN + TAG_LEN;

/// Why embedding or extraction failed.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The image carries no mark in the format being read: no LSB magic bytes, or no resilient
    /// record with a valid outer checksum. A damaged magic or checksum looks the same.
    #[error("no watermark found")]
    NoWatermark,
    /// The LSB format version or the resilient mode byte is one this library does not know.
    #[error("unsupported watermark format version {0:#04x}")]
    UnsupportedVersion(u8),
    /// The stored length does not fit, a checksum does not match, or a resilient record has
    /// nonzero padding or text that is not UTF-8.
    #[error("damaged watermark")]
    Damaged,
    /// The payload does not fit in the image.
    #[error("payload is {len} bytes but the image holds at most {capacity}")]
    PayloadTooLarge {
        /// Size of the rejected payload in bytes.
        len: usize,
        /// Largest payload the image can hold in bytes.
        capacity: usize,
    },
    /// The text is longer than the resilient mode holds.
    #[error("text is {len} bytes but the resilient mode holds at most {max}", max = RESILIENT_MAX_TEXT)]
    TextTooLong {
        /// Size of the rejected text in bytes.
        len: usize,
    },
    /// The image is too small to embed a resilient mark.
    #[error("image is {short_side} pixels on the short side but the resilient mode needs at least {min}", min = RESILIENT_MIN_SIDE)]
    ImageTooSmall {
        /// The shorter of the image's width and height.
        short_side: u32,
    },
    /// The watermark is encrypted, so reading it needs a password.
    #[error("watermark is encrypted, a password is required")]
    PasswordRequired,
    /// A password was given but the watermark is not encrypted.
    #[error("watermark is not encrypted")]
    NotEncrypted,
    /// The encrypted watermark did not open: the password is wrong or the watermark is damaged.
    #[error("wrong password or damaged watermark")]
    WrongPasswordOrDamaged,
    /// The password is empty.
    #[error("password is empty")]
    EmptyPassword,
    /// The system random source failed while generating a salt or nonce.
    #[error("system random source failed")]
    RandomSource,
    /// Argon2 could not derive the keys, which in practice means it ran out of memory.
    #[error("cannot derive keys from the password")]
    KeyDerivation,
}

fn available_bytes(width: u32, height: u32) -> u128 {
    u128::from(width) * u128::from(height) * 3 / 8
}

fn sample_count(width: u32, height: u32) -> usize {
    width as usize * height as usize * 3
}

/// Whether a record with a payload of `len` bytes and `overhead` bytes around it fits in the image and in the length field.
fn fits(len: usize, overhead: usize, width: u32, height: u32) -> bool {
    let len = len as u128;
    len <= u128::from(u32::MAX) && len + overhead as u128 <= available_bytes(width, height)
}

fn payload_capacity(overhead: usize, width: u32, height: u32) -> usize {
    let bytes = available_bytes(width, height)
        .saturating_sub(overhead as u128)
        .min(u128::from(u32::MAX));
    usize::try_from(bytes).unwrap_or(usize::MAX)
}

/// Payload bytes an image of this size can hold. Images too small for the 13 bytes of header
/// and checksum also report 0, although not even an empty payload fits.
pub fn capacity(width: u32, height: u32) -> usize {
    payload_capacity(OVERHEAD, width, height)
}

/// Payload bytes an image of this size can hold with a password, after 65 bytes of overhead.
pub fn capacity_with_password(width: u32, height: u32) -> usize {
    payload_capacity(PASSWORD_OVERHEAD, width, height)
}

/// Embeds `payload` and returns an 8-bit copy of the image: RGBA if it has alpha, RGB otherwise.
/// Alpha and the bits after the record are left unchanged.
pub fn embed(image: DynamicImage, payload: &[u8]) -> Result<DynamicImage, Error> {
    let (width, height) = (image.width(), image.height());
    let too_large = || Error::PayloadTooLarge {
        len: payload.len(),
        capacity: capacity(width, height),
    };
    if !fits(payload.len(), OVERHEAD, width, height) {
        return Err(too_large());
    }
    let len = u32::try_from(payload.len()).map_err(|_| too_large())?;

    let mut record = Vec::with_capacity(payload.len() + OVERHEAD);
    record.extend_from_slice(&MAGIC);
    record.push(V1);
    record.extend_from_slice(&len.to_be_bytes());
    record.extend_from_slice(payload);
    record.extend_from_slice(&crc32fast::hash(&record).to_be_bytes());

    Ok(with_samples(image, |samples, channels| {
        write_bits(samples, channels, 0..sample_count(width, height), &record);
    }))
}

/// Encrypts `payload` with `password` and scatters it across the image. Returns an 8-bit copy like [`embed`].
pub fn embed_with_password(
    image: DynamicImage,
    payload: &[u8],
    password: &str,
) -> Result<DynamicImage, Error> {
    let (salt, nonce) = salt_and_nonce()?;
    embed_v2(image, payload, password, &salt, &nonce)
}

fn salt_and_nonce() -> Result<([u8; SALT_LEN], [u8; NONCE_LEN]), Error> {
    let (mut salt, mut nonce) = ([0; SALT_LEN], [0; NONCE_LEN]);
    getrandom::fill(&mut salt)
        .and_then(|()| getrandom::fill(&mut nonce))
        .map_err(|_| Error::RandomSource)?;
    Ok((salt, nonce))
}

fn embed_v2(
    image: DynamicImage,
    payload: &[u8],
    password: &str,
    salt: &[u8; SALT_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Result<DynamicImage, Error> {
    if password.is_empty() {
        return Err(Error::EmptyPassword);
    }
    let (width, height) = (image.width(), image.height());
    let too_large = || Error::PayloadTooLarge {
        len: payload.len(),
        capacity: capacity_with_password(width, height),
    };
    if !fits(payload.len(), PASSWORD_OVERHEAD, width, height) {
        return Err(too_large());
    }
    let len = u32::try_from(payload.len()).map_err(|_| too_large())?;

    let (cipher_key, position_key) = derive_keys(password, salt)?;
    let record = build_v2(payload, len, &cipher_key, salt, nonce).map_err(|_| too_large())?;
    let (prefix, body) = record.split_at(PREFIX_LEN);

    let count = sample_count(width, height);
    let scattered = draw_positions(&position_key, count, body.len() * 8);
    // The ChaCha20 keystream ends after 256 GiB, which only images of tens of gigapixels reach.
    if scattered.len() < body.len() * 8 {
        return Err(too_large());
    }
    Ok(with_samples(image, |samples, channels| {
        write_bits(samples, channels, 0..PREFIX_BITS, prefix);
        write_bits(samples, channels, scattered, body);
    }))
}

fn argon2id(password: &str, salt: &[u8], output: &mut [u8]) -> Result<(), Error> {
    let params = Params::new(19_456, 2, 1, Some(output.len())).map_err(|_| Error::KeyDerivation)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, output)
        .map_err(|_| Error::KeyDerivation)
}

fn derive_keys(password: &str, salt: &[u8]) -> Result<([u8; 32], [u8; 32]), Error> {
    let mut keys = [0u8; 64];
    argon2id(password, salt, &mut keys)?;
    let (mut cipher_key, mut position_key) = ([0; 32], [0; 32]);
    cipher_key.copy_from_slice(&keys[..32]);
    position_key.copy_from_slice(&keys[32..]);
    Ok((cipher_key, position_key))
}

fn build_v2(
    payload: &[u8],
    len: u32,
    cipher_key: &[u8; 32],
    salt: &[u8; SALT_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Result<Vec<u8>, chacha20poly1305::Error> {
    let mut record = Vec::with_capacity(PASSWORD_OVERHEAD + payload.len());
    record.extend_from_slice(&MAGIC);
    record.push(V2);
    record.extend_from_slice(salt);
    record.extend_from_slice(&len.to_be_bytes());
    let ciphertext = XChaCha20Poly1305::new(&Key::from(*cipher_key)).encrypt(
        &XNonce::from(*nonce),
        Payload {
            msg: payload,
            aad: &record,
        },
    )?;
    record.extend_from_slice(nonce);
    record.extend_from_slice(&ciphertext);
    Ok(record)
}

/// Draws the first `count` v2 body positions from `168..samples`, at most all of them.
fn draw_positions(key: &[u8; 32], samples: usize, count: usize) -> Vec<usize> {
    let mut positions = shuffle(key, samples.saturating_sub(PREFIX_BITS), count);
    for position in &mut positions {
        *position += PREFIX_BITS;
    }
    positions
}

/// The first `count` entries of a keyed Fisher-Yates shuffle of `0..len`, fewer if the keystream runs out.
fn shuffle(key: &[u8; 32], len: usize, count: usize) -> Vec<usize> {
    let count = count.min(len);
    let mut keystream = ChaCha20::new(&Key::from(*key), &Nonce::from([0; 12]));
    let mut displaced = HashMap::new();
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        let range = (len - i) as u64;
        let limit = (1u128 << 64) / u128::from(range) * u128::from(range);
        let r = loop {
            let mut word = [0; 8];
            if keystream.try_apply_keystream(&mut word).is_err() {
                return entries;
            }
            let value = u64::from_le_bytes(word);
            if u128::from(value) < limit {
                break (value % range) as usize;
            }
        };
        let at_i = displaced.remove(&i).unwrap_or(i);
        let at_j = if r == 0 {
            at_i
        } else {
            displaced.insert(i + r, at_i).unwrap_or(i + r)
        };
        entries.push(at_j);
    }
    entries
}

/// Reads the payload of an image watermarked without a password.
pub fn extract(image: &DynamicImage) -> Result<Vec<u8>, Error> {
    read(image, None)
}

/// Reads and decrypts the payload of an image watermarked with `password`.
pub fn extract_with_password(image: &DynamicImage, password: &str) -> Result<Vec<u8>, Error> {
    if password.is_empty() {
        return Err(Error::EmptyPassword);
    }
    read(image, Some(password))
}

/// A watermark found by [`read_watermark`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Watermark {
    /// The payload of an LSB watermark, format v1 or v2.
    Lsb(Vec<u8>),
    /// The text of a resilient mark, format v3.
    Resilient(String),
}

/// Reads the LSB formats in stored pixel order and, if that fails, the resilient format after
/// applying `orientation`. Returns the LSB error when the resilient read finds no watermark.
pub fn read_watermark(
    image: &DynamicImage,
    orientation: Orientation,
    password: Option<&str>,
) -> Result<Watermark, Error> {
    let lsb = match password {
        Some(password) => extract_with_password(image, password),
        None => extract(image),
    };
    let lsb_error = match lsb {
        Ok(payload) => return Ok(Watermark::Lsb(payload)),
        Err(err) => err,
    };
    // Each arm allocates at most one image, so a rotation never holds a clone and its result.
    let image = match orientation {
        Orientation::NoTransforms => Cow::Borrowed(image),
        Orientation::Rotate90 => Cow::Owned(image.rotate90()),
        Orientation::Rotate270 => Cow::Owned(image.rotate270()),
        Orientation::Rotate90FlipH => {
            let mut rotated = image.rotate90();
            rotated.apply_orientation(Orientation::FlipHorizontal);
            Cow::Owned(rotated)
        }
        Orientation::Rotate270FlipH => {
            let mut rotated = image.rotate270();
            rotated.apply_orientation(Orientation::FlipHorizontal);
            Cow::Owned(rotated)
        }
        other => {
            let mut copy = image.clone();
            copy.apply_orientation(other);
            Cow::Owned(copy)
        }
    };
    let resilient = match password {
        Some(password) => extract_resilient_with_password(&image, password),
        None => extract_resilient(&image),
    };
    match resilient {
        Ok(text) => Ok(Watermark::Resilient(text)),
        Err(Error::NoWatermark) => Err(lsb_error),
        Err(err) => Err(err),
    }
}

fn read(image: &DynamicImage, password: Option<&str>) -> Result<Vec<u8>, Error> {
    let (width, height) = (image.width(), image.height());
    let (samples, channels) = color_buffer(image);
    let start = read_bytes(&samples, channels, 0..sample_count(width, height), 5);
    let Some((magic, &[version])) = start.split_first_chunk::<4>() else {
        return Err(Error::NoWatermark);
    };
    if *magic != MAGIC {
        return Err(Error::NoWatermark);
    }
    match (version, password) {
        (V1, None) => read_v1(&samples, channels, width, height),
        (V1, Some(_)) => Err(Error::NotEncrypted),
        (V2, None) => Err(Error::PasswordRequired),
        (V2, Some(password)) => parse_v2(&samples, channels, width, height, password),
        (version, _) => Err(Error::UnsupportedVersion(version)),
    }
}

fn read_v1(samples: &[u8], channels: usize, width: u32, height: u32) -> Result<Vec<u8>, Error> {
    let count = sample_count(width, height);
    let header = read_bytes(samples, channels, 0..count, HEADER_LEN);
    let Some(&[.., a, b, c, d]) = header.as_slice().first_chunk::<HEADER_LEN>() else {
        return Err(Error::NoWatermark);
    };
    let len = usize::try_from(u32::from_be_bytes([a, b, c, d])).map_err(|_| Error::Damaged)?;
    if !fits(len, OVERHEAD, width, height) {
        return Err(Error::Damaged);
    }

    let mut record = read_bytes(samples, channels, 0..count, HEADER_LEN + len + CRC_LEN);
    let Some((body, crc)) = record.split_last_chunk::<CRC_LEN>() else {
        return Err(Error::Damaged);
    };
    if crc32fast::hash(body) != u32::from_be_bytes(*crc) {
        return Err(Error::Damaged);
    }
    record.truncate(record.len() - CRC_LEN);
    record.drain(..HEADER_LEN);
    Ok(record)
}

/// Reads and decrypts a v2 record; every failure after the prefix is `WrongPasswordOrDamaged`.
fn parse_v2(
    samples: &[u8],
    channels: usize,
    width: u32,
    height: u32,
    password: &str,
) -> Result<Vec<u8>, Error> {
    let count = sample_count(width, height);
    let prefix = read_bytes(samples, channels, 0..count, PREFIX_LEN);
    if prefix.len() < PREFIX_LEN {
        return Err(Error::Damaged);
    }
    let (cipher_key, position_key) = derive_keys(password, &prefix[5..])?;

    let length = read_bytes(
        samples,
        channels,
        draw_positions(&position_key, count, 32),
        4,
    );
    let length: [u8; 4] = length
        .try_into()
        .map_err(|_| Error::WrongPasswordOrDamaged)?;
    let len =
        usize::try_from(u32::from_be_bytes(length)).map_err(|_| Error::WrongPasswordOrDamaged)?;
    if !fits(len, PASSWORD_OVERHEAD, width, height) {
        return Err(Error::WrongPasswordOrDamaged);
    }

    let body_len = 4 + NONCE_LEN + len + TAG_LEN;
    let positions = draw_positions(&position_key, count, body_len * 8);
    let body = read_bytes(samples, channels, positions, body_len);
    let Some((nonce, ciphertext)) = body
        .get(4..)
        .and_then(|rest| rest.split_first_chunk::<NONCE_LEN>())
    else {
        return Err(Error::WrongPasswordOrDamaged);
    };
    let aad = [&prefix[..], &length[..]].concat();
    XChaCha20Poly1305::new(&Key::from(cipher_key))
        .decrypt(
            &XNonce::from(*nonce),
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| Error::WrongPasswordOrDamaged)
}

fn with_samples(image: DynamicImage, write: impl FnOnce(&mut [u8], usize)) -> DynamicImage {
    if image.color().has_alpha() {
        let mut buffer = image.into_rgba8();
        write(&mut buffer, 4);
        DynamicImage::ImageRgba8(buffer)
    } else {
        let mut buffer = image.into_rgb8();
        write(&mut buffer, 3);
        DynamicImage::ImageRgb8(buffer)
    }
}

fn color_buffer(image: &DynamicImage) -> (Cow<'_, [u8]>, usize) {
    match image {
        DynamicImage::ImageRgb8(buffer) => (Cow::Borrowed(&**buffer), 3),
        DynamicImage::ImageRgba8(buffer) => (Cow::Borrowed(&**buffer), 4),
        other => (Cow::Owned(other.to_rgb8().into_raw()), 3),
    }
}

fn offset(sample: usize, channels: usize) -> usize {
    sample / 3 * channels + sample % 3
}

fn write_bits(
    samples: &mut [u8],
    channels: usize,
    positions: impl IntoIterator<Item = usize>,
    bytes: &[u8],
) {
    let bits = bytes
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
    for (position, bit) in positions.into_iter().zip(bits) {
        let sample = &mut samples[offset(position, channels)];
        *sample = (*sample & !1) | bit;
    }
}

fn read_bytes(
    samples: &[u8],
    channels: usize,
    positions: impl IntoIterator<Item = usize>,
    len: usize,
) -> Vec<u8> {
    let mut bits = positions
        .into_iter()
        .map(|position| samples[offset(position, channels)] & 1);
    std::iter::from_fn(move || (0..8).try_fold(0u8, |byte, _| Some((byte << 1) | bits.next()?)))
        .take(len)
        .collect()
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    const SALT: [u8; SALT_LEN] = [7; SALT_LEN];
    const NONCE: [u8; NONCE_LEN] = [9; NONCE_LEN];
    const PASSWORD: &str = "correct horse battery staple";

    fn image() -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(32, 32, |x, y| {
            Rgb([
                (x * 37 + y * 11) as u8,
                (x * 13 + y * 29) as u8,
                (x * 7 + y * 3) as u8,
            ])
        }))
    }

    fn sealed(payload: &[u8]) -> DynamicImage {
        embed_v2(image(), payload, PASSWORD, &SALT, &NONCE).unwrap()
    }

    fn body_position(bit: usize) -> usize {
        let (_, position_key) = derive_keys(PASSWORD, &SALT).unwrap();
        draw_positions(&position_key, 32 * 32 * 3, bit + 1)[bit]
    }

    fn assert_flip_detected(sample: usize) {
        let mut buffer = sealed(b"hello").into_rgb8();
        let raw: &mut [u8] = &mut buffer;
        raw[sample] ^= 1;
        assert_eq!(
            extract_with_password(&DynamicImage::ImageRgb8(buffer), PASSWORD),
            Err(Error::WrongPasswordOrDamaged)
        );
    }

    #[test]
    fn draw_positions_covers_every_candidate_once() {
        let mut positions = draw_positions(&[1; 32], 300, 300 - PREFIX_BITS);
        positions.sort_unstable();
        assert_eq!(positions, (PREFIX_BITS..300).collect::<Vec<_>>());
    }

    #[test]
    fn flipping_a_salt_bit_fails_authentication() {
        assert_flip_detected(40);
    }

    #[test]
    fn flipping_a_length_bit_fails_authentication() {
        assert_flip_detected(body_position(31));
    }

    #[test]
    fn flipping_a_nonce_bit_fails_authentication() {
        assert_flip_detected(body_position(32));
    }

    #[test]
    fn flipping_a_ciphertext_bit_fails_authentication() {
        assert_flip_detected(body_position(32 + NONCE_LEN * 8));
    }

    #[test]
    fn known_answer_output_is_stable() {
        let marked = sealed(b"Copyright 2026 Example Studio");
        assert_eq!(crc32fast::hash(marked.as_bytes()), 0x0ADF9F13);
    }
}
