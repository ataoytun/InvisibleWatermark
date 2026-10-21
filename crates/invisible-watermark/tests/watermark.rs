use invisible_watermark::image::codecs::jpeg::JpegEncoder;
use invisible_watermark::image::{
    ColorType, DynamicImage, GrayImage, ImageBuffer, ImageFormat, Luma, Rgb, RgbImage, Rgba,
    RgbaImage,
};
use invisible_watermark::{
    Error, capacity, capacity_with_password, embed, embed_with_password, extract,
    extract_with_password,
};

const PASSWORD: &str = "correct horse battery staple";
const MAGIC_BIT: usize = 0;
const VERSION_LOW_BIT: usize = 39;
const PAYLOAD_BIT: usize = 72;
const HI_RECORD: [u8; 15] = [
    b'I', b'W', b'M', b'K', 1, 0, 0, 0, 2, b'h', b'i', 0xFA, 0xD9, 0xD6, 0xD5,
];

/// Image whose low bits vary, so a changed bit is not hidden by equal values.
fn rgb(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(width, height, |x, y| {
        Rgb([
            (x * 37 + y * 11) as u8,
            (x * 13 + y * 29) as u8,
            (x * 7 + y * 3) as u8,
        ])
    }))
}

fn rgba(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgba8(RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            (x * 37 + y * 11) as u8,
            (x * 13 + y * 29) as u8,
            (x * 7 + y * 3) as u8,
            (x * 5 + y * 17) as u8,
        ])
    }))
}

/// Flips the low bit of the given samples; in an RGB buffer the sample index is the record bit index.
fn flip_bits(image: DynamicImage, samples: &[usize]) -> DynamicImage {
    let mut buffer = image.into_rgb8();
    let raw: &mut [u8] = &mut buffer;
    for &sample in samples {
        raw[sample] ^= 1;
    }
    DynamicImage::ImageRgb8(buffer)
}

/// Packs the low bit of each sample into bytes, most significant bit first.
fn low_bits(samples: &[u8], len: usize) -> Vec<u8> {
    samples
        .chunks(8)
        .take(len)
        .map(|bits| {
            bits.iter()
                .fold(0, |byte, sample| (byte << 1) | (sample & 1))
        })
        .collect()
}

/// Replaces the low bits of an RGB image with `stream`, most significant bit first.
fn with_low_bits(image: DynamicImage, stream: &[u8]) -> DynamicImage {
    let mut buffer = image.into_rgb8();
    let bits = stream
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
    for (sample, bit) in buffer.iter_mut().zip(bits) {
        *sample = (*sample & !1) | bit;
    }
    DynamicImage::ImageRgb8(buffer)
}

fn round_trip(text: &str) {
    let marked = embed(rgb(64, 64), text.as_bytes()).unwrap();
    assert_eq!(extract(&marked).unwrap(), text.as_bytes());
}

#[test]
fn round_trips_ascii_text() {
    round_trip("Copyright 2026 Example Studio");
}

#[test]
fn round_trips_mixed_scripts() {
    round_trip("naïve café, Ελλάδα, 東京, Москва");
}

#[test]
fn round_trips_emoji() {
    round_trip("🦀 ok 👍🏽");
}

#[test]
fn payload_that_fills_capacity_round_trips() {
    assert_eq!(capacity(10, 10), 24);
    let payload = [0xA5; 24];
    let marked = embed(rgb(10, 10), &payload).unwrap();
    assert_eq!(extract(&marked).unwrap(), payload);
}

#[test]
fn payload_one_byte_over_capacity_is_rejected() {
    let result = embed(rgb(10, 10), &[0; 25]);
    assert_eq!(
        result.unwrap_err(),
        Error::PayloadTooLarge {
            len: 25,
            capacity: 24
        }
    );
}

#[test]
fn empty_payload_is_rejected_when_header_does_not_fit() {
    let result = embed(rgb(5, 5), b"");
    assert_eq!(
        result.unwrap_err(),
        Error::PayloadTooLarge {
            len: 0,
            capacity: 0
        }
    );
}

#[test]
fn capacity_is_capped_at_the_length_field() {
    assert_eq!(capacity(u32::MAX, u32::MAX), u32::MAX as usize);
}

#[test]
fn length_field_stores_all_four_bytes() {
    let marked = embed(rgb(420, 420), &vec![0; 0x010203])
        .unwrap()
        .into_rgb8();
    assert_eq!(low_bits(&marked, 9)[5..], [0, 1, 2, 3]);
}

#[test]
fn embed_writes_the_v1_record_into_the_low_bits() {
    let marked = embed(rgb(8, 8), b"hi").unwrap().into_rgb8();
    assert_eq!(low_bits(&marked, HI_RECORD.len()), HI_RECORD);
}

#[test]
fn extract_reads_a_hand_written_v1_record() {
    let image = with_low_bits(rgb(8, 8), &HI_RECORD);
    assert_eq!(extract(&image).unwrap(), b"hi");
}

#[test]
fn extract_reports_no_watermark_in_plain_image() {
    assert_eq!(extract(&rgb(64, 64)), Err(Error::NoWatermark));
}

#[test]
fn extract_reports_no_watermark_when_image_is_smaller_than_header() {
    assert_eq!(extract(&rgb(3, 3)), Err(Error::NoWatermark));
}

#[test]
fn extract_reports_no_watermark_after_magic_bit_flip() {
    let marked = flip_bits(embed(rgb(64, 64), b"hello").unwrap(), &[MAGIC_BIT]);
    assert_eq!(extract(&marked), Err(Error::NoWatermark));
}

#[test]
fn extract_reports_unsupported_version_after_version_bit_flip() {
    let marked = flip_bits(embed(rgb(64, 64), b"hello").unwrap(), &[VERSION_LOW_BIT]);
    assert_eq!(extract(&marked), Err(Error::UnsupportedVersion(0)));
}

#[test]
fn extract_reports_damaged_after_payload_bit_flip() {
    let marked = flip_bits(embed(rgb(64, 64), b"hello").unwrap(), &[PAYLOAD_BIT]);
    assert_eq!(extract(&marked), Err(Error::Damaged));
}

#[test]
fn extract_rejects_length_beyond_image_even_with_valid_checksum() {
    // The stored length is 12 but only 11 bytes fit before the image ends, so a read without the guard would find a valid CRC.
    let mut stream = vec![b'I', b'W', b'M', b'K', 1, 0, 0, 0, 12];
    stream.extend_from_slice(b"hello world");
    stream.extend_from_slice(&crc32fast::hash(&stream).to_be_bytes());
    let image = with_low_bits(rgb(8, 8), &stream);
    assert_eq!(extract(&image), Err(Error::Damaged));
}

#[test]
fn embed_leaves_alpha_and_bits_after_record_unchanged() {
    let record_bits = (9 + b"hello".len() + 4) * 8;
    let original = rgba(16, 16);
    let marked = embed(original.clone(), b"hello").unwrap();
    let before = original.to_rgba8();
    let after = marked.to_rgba8();
    for (index, (a, b)) in before.pixels().zip(after.pixels()).enumerate() {
        assert_eq!(a[3], b[3], "alpha changed at pixel {index}");
        for channel in 0..3 {
            let sample = index * 3 + channel;
            if sample >= record_bits {
                assert_eq!(a[channel], b[channel], "sample {sample} changed");
            } else {
                assert_eq!((a[channel] ^ b[channel]) & !1, 0, "sample {sample}");
            }
        }
    }
}

#[test]
fn embed_keeps_rgb_as_rgb_and_rgba_as_rgba() {
    assert_eq!(embed(rgb(16, 16), b"x").unwrap().color(), ColorType::Rgb8);
    assert_eq!(embed(rgba(16, 16), b"x").unwrap().color(), ColorType::Rgba8);
}

#[test]
fn embed_turns_grayscale_into_rgb() {
    let gray = DynamicImage::ImageLuma8(GrayImage::from_fn(16, 16, |x, y| {
        Luma([(x * 16 + y) as u8])
    }));
    let marked = embed(gray, b"x").unwrap();
    assert_eq!(marked.color(), ColorType::Rgb8);
    assert_eq!(extract(&marked).unwrap(), b"x");
}

#[test]
fn embed_reduces_16_bit_input_to_8_bit() {
    let rgb16 = DynamicImage::ImageRgb16(ImageBuffer::<Rgb<u16>, _>::new(16, 16));
    let rgba16 = DynamicImage::ImageRgba16(ImageBuffer::<Rgba<u16>, _>::new(16, 16));
    let marked = embed(rgb16, b"x").unwrap();
    assert_eq!(marked.color(), ColorType::Rgb8);
    assert_eq!(extract(&marked).unwrap(), b"x");
    let marked = embed(rgba16, b"x").unwrap();
    assert_eq!(marked.color(), ColorType::Rgba8);
    assert_eq!(extract(&marked).unwrap(), b"x");
}

#[test]
fn extract_reads_images_converted_to_16_bit() {
    let marked = embed(rgb(16, 16), b"hello").unwrap();
    let converted = DynamicImage::ImageRgb16(marked.to_rgb16());
    assert_eq!(extract(&converted).unwrap(), b"hello");
}

#[test]
fn jpeg_reencoding_destroys_watermark() {
    let payload = b"Copyright 2026 Example Studio";
    let marked = embed(rgb(64, 64), payload).unwrap();
    let mut jpeg = Vec::new();
    marked
        .write_with_encoder(JpegEncoder::new_with_quality(&mut jpeg, 90))
        .unwrap();
    let decoded =
        invisible_watermark::image::load_from_memory_with_format(&jpeg, ImageFormat::Jpeg).unwrap();
    assert_ne!(extract(&decoded).ok().as_deref(), Some(&payload[..]));
}

fn round_trip_with_password(text: &str) {
    let marked = embed_with_password(rgb(64, 64), text.as_bytes(), PASSWORD).unwrap();
    assert_eq!(
        extract_with_password(&marked, PASSWORD).unwrap(),
        text.as_bytes()
    );
}

#[test]
fn round_trips_ascii_text_with_password() {
    round_trip_with_password("Copyright 2026 Example Studio");
}

#[test]
fn round_trips_mixed_scripts_with_password() {
    round_trip_with_password("naïve café, Ελλάδα, 東京, Москва");
}

#[test]
fn round_trips_emoji_with_password() {
    round_trip_with_password("🦀 ok 👍🏽");
}

#[test]
fn extract_with_wrong_password_reports_wrong_password_or_damaged() {
    let marked = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    assert_eq!(
        extract_with_password(&marked, "wrong password"),
        Err(Error::WrongPasswordOrDamaged)
    );
}

#[test]
fn extract_reports_password_required_for_encrypted_image() {
    let marked = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    assert_eq!(extract(&marked), Err(Error::PasswordRequired));
}

#[test]
fn extract_with_password_reports_not_encrypted_for_plain_image() {
    let marked = embed(rgb(64, 64), b"hello").unwrap();
    assert_eq!(
        extract_with_password(&marked, PASSWORD),
        Err(Error::NotEncrypted)
    );
}

#[test]
fn empty_password_is_rejected() {
    assert_eq!(
        embed_with_password(rgb(64, 64), b"hello", "").unwrap_err(),
        Error::EmptyPassword
    );
    let marked = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    assert_eq!(
        extract_with_password(&marked, ""),
        Err(Error::EmptyPassword)
    );
}

#[test]
fn plaintext_is_not_in_the_sequential_low_bits() {
    let text = b"Copyright 2026 Example Studio";
    let marked = embed_with_password(rgb(64, 64), text, PASSWORD)
        .unwrap()
        .into_rgb8();
    let stream = low_bits(&marked, 64 * 64 * 3 / 8);
    assert_eq!(stream[..5], *b"IWMK\x02");
    assert!(
        !stream[21..]
            .windows(text.len())
            .any(|window| window == text)
    );
}

#[test]
fn embedding_twice_uses_a_fresh_salt() {
    let first = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    let second = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    assert_ne!(first.as_bytes(), second.as_bytes());
    let salt = |image: DynamicImage| low_bits(&image.into_rgb8(), 21)[5..].to_vec();
    assert_ne!(salt(first), salt(second));
}

#[test]
fn payload_that_fills_password_capacity_round_trips() {
    assert_eq!(capacity_with_password(20, 20), 85);
    let payload = [0xA5; 85];
    let marked = embed_with_password(rgb(20, 20), &payload, PASSWORD).unwrap();
    assert_eq!(extract_with_password(&marked, PASSWORD).unwrap(), payload);
}

#[test]
fn payload_one_byte_over_password_capacity_is_rejected() {
    let result = embed_with_password(rgb(20, 20), &[0; 86], PASSWORD);
    assert_eq!(
        result.unwrap_err(),
        Error::PayloadTooLarge {
            len: 86,
            capacity: 85
        }
    );
}
