use invisible_watermark::image::codecs::jpeg::JpegEncoder;
use invisible_watermark::image::metadata::Orientation;
use invisible_watermark::image::{
    ColorType, DynamicImage, GrayImage, ImageBuffer, ImageFormat, Luma, Rgb, RgbImage, Rgba,
    RgbaImage,
};
use invisible_watermark::{
    Error, Watermark, capacity, capacity_with_password, embed, embed_resilient,
    embed_resilient_with_password, embed_with_password, extract, extract_resilient,
    extract_resilient_with_password, extract_with_password, read_watermark,
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

const RESILIENT_TEXT_32: &str = "0123456789 ABCDEF abcdef !#$%&*+";

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

fn photo_image(width: u32, height: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(photo(width, height))
}

fn jpeg(image: &DynamicImage, quality: u8) -> DynamicImage {
    let mut bytes = Vec::new();
    image
        .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, quality))
        .unwrap();
    invisible_watermark::image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg).unwrap()
}

fn resilient_round_trip(text: &str) {
    let marked = embed_resilient(photo_image(512, 512), text).unwrap();
    assert_eq!(extract_resilient(&marked).unwrap(), text);
}

fn resilient_round_trip_with_password(text: &str) {
    let marked = embed_resilient_with_password(photo_image(512, 512), text, PASSWORD).unwrap();
    assert_eq!(
        extract_resilient_with_password(&marked, PASSWORD).unwrap(),
        text
    );
}

#[test]
fn resilient_round_trips_ascii_text() {
    resilient_round_trip("Copyright 2026 Example Studio");
}

#[test]
fn resilient_round_trips_mixed_scripts() {
    resilient_round_trip("café, Ελλάδα, 東京");
}

#[test]
fn resilient_round_trips_emoji() {
    resilient_round_trip("🦀 ok 👍🏽");
}

#[test]
fn resilient_round_trips_ascii_text_with_password() {
    resilient_round_trip_with_password("Copyright 2026 Example Studio");
}

#[test]
fn resilient_round_trips_mixed_scripts_with_password() {
    resilient_round_trip_with_password("café, Ελλάδα, 東京");
}

#[test]
fn resilient_round_trips_emoji_with_password() {
    resilient_round_trip_with_password("🦀 ok 👍🏽");
}

#[test]
fn resilient_text_of_32_bytes_round_trips() {
    assert_eq!(RESILIENT_TEXT_32.len(), 32);
    resilient_round_trip(RESILIENT_TEXT_32);
}

#[test]
fn resilient_text_of_33_bytes_is_rejected() {
    let text = format!("{RESILIENT_TEXT_32}!");
    assert_eq!(
        embed_resilient(photo_image(512, 512), &text).unwrap_err(),
        Error::TextTooLong { len: 33 }
    );
}

#[test]
fn resilient_empty_text_round_trips() {
    resilient_round_trip("");
}

#[test]
fn resilient_mark_survives_jpeg_quality_75() {
    let marked = embed_resilient(photo_image(768, 512), "Example Studio").unwrap();
    assert_eq!(
        extract_resilient(&jpeg(&marked, 75)).unwrap(),
        "Example Studio"
    );
}

#[test]
fn resilient_mark_survives_half_size_and_jpeg_quality_75() {
    let marked = embed_resilient(photo_image(768, 512), "Example Studio").unwrap();
    let half = DynamicImage::ImageRgb8(invisible_watermark::image::imageops::resize(
        &marked.into_rgb8(),
        384,
        256,
        invisible_watermark::image::imageops::FilterType::Lanczos3,
    ));
    assert_eq!(
        extract_resilient(&jpeg(&half, 75)).unwrap(),
        "Example Studio"
    );
}

#[test]
fn resilient_embed_needs_512_pixels_on_the_short_side() {
    assert_eq!(
        embed_resilient(photo_image(700, 511), "hi").unwrap_err(),
        Error::ImageTooSmall { short_side: 511 }
    );
    let marked = embed_resilient(photo_image(512, 700), "hi").unwrap();
    assert_eq!(extract_resilient(&marked).unwrap(), "hi");
}

#[test]
fn extract_resilient_reports_no_watermark_in_unmarked_image() {
    let image = photo_image(768, 512);
    assert_eq!(extract_resilient(&image), Err(Error::NoWatermark));
    assert_eq!(
        extract_resilient_with_password(&image, PASSWORD),
        Err(Error::NoWatermark)
    );
}

#[test]
fn extract_resilient_reports_no_watermark_for_one_pixel_image() {
    assert_eq!(
        extract_resilient(&photo_image(1, 1)),
        Err(Error::NoWatermark)
    );
}

#[test]
fn extract_resilient_with_wrong_password_reports_wrong_password_or_damaged() {
    let marked = embed_resilient_with_password(photo_image(512, 512), "hello", PASSWORD).unwrap();
    assert_eq!(
        extract_resilient_with_password(&marked, "wrong password"),
        Err(Error::WrongPasswordOrDamaged)
    );
}

#[test]
fn extract_resilient_reports_password_required_for_encrypted_mark() {
    let marked = embed_resilient_with_password(photo_image(512, 512), "hello", PASSWORD).unwrap();
    assert_eq!(extract_resilient(&marked), Err(Error::PasswordRequired));
}

#[test]
fn extract_resilient_with_password_reports_not_encrypted_for_plain_mark() {
    let marked = embed_resilient(photo_image(512, 512), "hello").unwrap();
    assert_eq!(
        extract_resilient_with_password(&marked, PASSWORD),
        Err(Error::NotEncrypted)
    );
}

#[test]
fn resilient_empty_password_is_rejected() {
    assert_eq!(
        embed_resilient_with_password(photo_image(512, 512), "hello", "").unwrap_err(),
        Error::EmptyPassword
    );
    assert_eq!(
        extract_resilient_with_password(&photo_image(512, 512), ""),
        Err(Error::EmptyPassword)
    );
}

#[test]
fn lsb_after_resilient_keeps_both_marks() {
    let marked = embed_resilient(photo_image(512, 512), "Example Studio").unwrap();
    let both = embed(marked, b"hello").unwrap();
    assert_eq!(extract(&both).unwrap(), b"hello");
    assert_eq!(extract_resilient(&both).unwrap(), "Example Studio");
}

#[test]
fn resilient_after_lsb_reads_back() {
    let lsb = embed(photo_image(512, 512), b"hello").unwrap();
    let both = embed_resilient(lsb, "Example Studio").unwrap();
    assert_eq!(extract_resilient(&both).unwrap(), "Example Studio");
}

#[test]
fn resilient_embed_keeps_alpha() {
    let colors = photo(512, 512);
    let original = DynamicImage::ImageRgba8(RgbaImage::from_fn(512, 512, |x, y| {
        let Rgb([r, g, b]) = *colors.get_pixel(x, y);
        Rgba([r, g, b, (x * 5 + y * 17) as u8])
    }));
    let marked = embed_resilient(original.clone(), "Example Studio").unwrap();
    assert_eq!(marked.color(), ColorType::Rgba8);
    let alpha = |image: &DynamicImage| -> Vec<u8> {
        image.to_rgba8().pixels().map(|pixel| pixel[3]).collect()
    };
    assert_eq!(alpha(&marked), alpha(&original));
    assert_eq!(extract_resilient(&marked).unwrap(), "Example Studio");
}

#[test]
fn read_watermark_returns_an_lsb_payload() {
    let plain = embed(rgb(64, 64), b"hello").unwrap();
    assert_eq!(
        read_watermark(&plain, Orientation::NoTransforms, None),
        Ok(Watermark::Lsb(b"hello".to_vec()))
    );
    let encrypted = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    assert_eq!(
        read_watermark(&encrypted, Orientation::NoTransforms, Some(PASSWORD)),
        Ok(Watermark::Lsb(b"hello".to_vec()))
    );
}

#[test]
fn read_watermark_reads_lsb_in_stored_pixel_order() {
    let marked = embed(rgb(64, 32), b"hello").unwrap();
    assert_eq!(
        read_watermark(&marked, Orientation::Rotate90, None),
        Ok(Watermark::Lsb(b"hello".to_vec()))
    );
}

#[test]
fn read_watermark_returns_a_resilient_text() {
    let plain = embed_resilient(photo_image(768, 512), "Example Studio").unwrap();
    assert_eq!(
        read_watermark(&plain, Orientation::NoTransforms, None),
        Ok(Watermark::Resilient("Example Studio".to_owned()))
    );
    let encrypted =
        embed_resilient_with_password(photo_image(768, 512), "Example Studio", PASSWORD).unwrap();
    assert_eq!(
        read_watermark(&encrypted, Orientation::NoTransforms, Some(PASSWORD)),
        Ok(Watermark::Resilient("Example Studio".to_owned()))
    );
}

#[test]
fn read_watermark_reads_a_resilient_mark_after_orientation() {
    let marked = embed_resilient(photo_image(768, 512), "Example Studio").unwrap();
    let stored = marked.rotate270();
    assert_eq!(
        read_watermark(&stored, Orientation::Rotate90, None),
        Ok(Watermark::Resilient("Example Studio".to_owned()))
    );
}

#[test]
fn read_watermark_reads_a_resilient_mark_when_an_lsb_header_survives() {
    let cases = [
        (
            embed(photo_image(768, 512), b"hello").unwrap(),
            1,
            Error::Damaged,
        ),
        (
            embed_with_password(photo_image(768, 512), b"hello", PASSWORD).unwrap(),
            2,
            Error::PasswordRequired,
        ),
    ];
    for (lsb, version, lsb_error) in cases {
        let marked = embed_resilient(lsb, "Example Studio").unwrap();
        let marked = with_low_bits(marked, &[b'I', b'W', b'M', b'K', version]);
        assert_eq!(extract(&marked), Err(lsb_error));
        assert_eq!(
            read_watermark(&marked, Orientation::NoTransforms, None),
            Ok(Watermark::Resilient("Example Studio".to_owned()))
        );
    }
}

#[test]
fn read_watermark_reports_the_lsb_error_when_no_resilient_mark_is_found() {
    let encrypted = embed_with_password(rgb(64, 64), b"hello", PASSWORD).unwrap();
    let bad_version = flip_bits(embed(rgb(64, 64), b"hello").unwrap(), &[VERSION_LOW_BIT]);
    let plain = embed(rgb(64, 64), b"hello").unwrap();
    let cases = [
        (rgb(64, 64), None, Error::NoWatermark),
        (encrypted, None, Error::PasswordRequired),
        (bad_version, None, Error::UnsupportedVersion(0)),
        (plain, Some(PASSWORD), Error::NotEncrypted),
    ];
    for (image, password, lsb_error) in cases {
        assert_eq!(
            read_watermark(&image, Orientation::NoTransforms, password),
            Err(lsb_error)
        );
    }
}

#[test]
fn read_watermark_reports_resilient_errors_other_than_no_watermark() {
    let encrypted =
        embed_resilient_with_password(photo_image(512, 512), "hello", PASSWORD).unwrap();
    assert_eq!(
        read_watermark(&encrypted, Orientation::NoTransforms, None),
        Err(Error::PasswordRequired)
    );
    assert_eq!(
        read_watermark(
            &encrypted,
            Orientation::NoTransforms,
            Some("wrong password")
        ),
        Err(Error::WrongPasswordOrDamaged)
    );
    let plain = embed_resilient(photo_image(512, 512), "hello").unwrap();
    assert_eq!(
        read_watermark(&plain, Orientation::NoTransforms, Some(PASSWORD)),
        Err(Error::NotEncrypted)
    );
}

#[test]
fn read_watermark_rejects_an_empty_password() {
    assert_eq!(
        read_watermark(&rgb(64, 64), Orientation::NoTransforms, Some("")),
        Err(Error::EmptyPassword)
    );
}
