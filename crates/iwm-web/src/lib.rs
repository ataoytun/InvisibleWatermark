//! JavaScript bindings for the browser demo.
//! `describe`, `embed` and `extract` take the raw bytes of an image file.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits};
use invisible_watermark::{Error, RESILIENT_MAX_TEXT, RESILIENT_MIN_SIDE};
use wasm_bindgen::prelude::*;

const JPEG_QUALITY: u8 = 90;

/// Size of an image and how much text it holds.
#[wasm_bindgen]
#[derive(Debug, PartialEq, Eq)]
pub struct Description {
    pub width: u32,
    pub height: u32,
    /// Text bytes that fit without a password.
    pub capacity: usize,
    /// Text bytes that fit with a password.
    pub capacity_with_password: usize,
    /// Text bytes that fit in the resilient mode, 0 when the image is too small for it.
    pub resilient_capacity: usize,
}

/// Reads the size from the image header, after EXIF orientation, without decoding the pixels.
#[wasm_bindgen]
pub fn describe(bytes: &[u8]) -> Result<Description, JsError> {
    describe_image(bytes).map_err(|err| JsError::new(&err))
}

/// Embeds `text`, encrypted if `password` is set and not empty, and returns the image as PNG, or as JPEG in the resilient mode.
#[wasm_bindgen]
pub fn embed(
    bytes: &[u8],
    text: &str,
    password: Option<String>,
    resilient: bool,
) -> Result<Vec<u8>, JsError> {
    embed_image(bytes, text, non_empty(password.as_deref()), resilient)
        .map_err(|err| JsError::new(&err))
}

/// Reads the watermark text, decrypting it if `password` is set and not empty.
#[wasm_bindgen]
pub fn extract(bytes: &[u8], password: Option<String>) -> Result<String, JsError> {
    extract_text(bytes, non_empty(password.as_deref())).map_err(|err| JsError::new(&err))
}

/// The version of this crate.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

fn non_empty(password: Option<&str>) -> Option<&str> {
    password.filter(|password| !password.is_empty())
}

fn read_error(err: ImageError) -> String {
    format!("cannot read the image: {err}")
}

/// Opens a decoder with format detection, refusing images over the default allocation limit of `image`.
fn open_decoder(bytes: &[u8]) -> Result<impl ImageDecoder, String> {
    let decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| format!("cannot read the image: {err}"))?
        .into_decoder()
        .map_err(read_error)?;
    Limits::default()
        .reserve(decoder.total_bytes())
        .map_err(read_error)?;
    Ok(decoder)
}

fn describe_image(bytes: &[u8]) -> Result<Description, String> {
    let mut decoder = open_decoder(bytes)?;
    let (width, height) = decoder.dimensions();
    let (width, height) = match decoder.orientation().map_err(read_error)? {
        Orientation::Rotate90
        | Orientation::Rotate270
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => (height, width),
        _ => (width, height),
    };
    Ok(Description {
        width,
        height,
        capacity: invisible_watermark::capacity(width, height),
        capacity_with_password: invisible_watermark::capacity_with_password(width, height),
        resilient_capacity: if width.min(height) >= RESILIENT_MIN_SIDE {
            RESILIENT_MAX_TEXT
        } else {
            0
        },
    })
}

fn embed_image(
    bytes: &[u8],
    text: &str,
    password: Option<&str>,
    resilient: bool,
) -> Result<Vec<u8>, String> {
    if text.is_empty() {
        return Err("text is empty".to_owned());
    }
    let mut decoder = open_decoder(bytes)?;
    let orientation = decoder.orientation().map_err(read_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(read_error)?;
    image.apply_orientation(orientation);
    let marked = match (resilient, password) {
        (false, Some(password)) => {
            invisible_watermark::embed_with_password(image, text.as_bytes(), password)
        }
        (false, None) => invisible_watermark::embed(image, text.as_bytes()),
        (true, Some(password)) => {
            invisible_watermark::embed_resilient_with_password(image, text, password)
        }
        (true, None) => invisible_watermark::embed_resilient(image, text),
    }
    .map_err(|err| format!("cannot embed into the image: {err}"))?;

    let mut encoded = Vec::new();
    if resilient {
        marked
            .write_with_encoder(JpegEncoder::new_with_quality(&mut encoded, JPEG_QUALITY))
            .map_err(|err| format!("cannot encode the image as JPEG: {err}"))?;
    } else {
        marked
            .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
            .map_err(|err| format!("cannot encode the image as PNG: {err}"))?;
    }
    Ok(encoded)
}

fn extract_text(bytes: &[u8], password: Option<&str>) -> Result<String, String> {
    let mut decoder = open_decoder(bytes)?;
    let orientation = decoder.orientation().map_err(read_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(read_error)?;
    let lsb = match password {
        Some(password) => invisible_watermark::extract_with_password(&image, password),
        None => invisible_watermark::extract(&image),
    };
    let lsb_error = match lsb {
        Ok(payload) => {
            return String::from_utf8(payload)
                .map_err(|_| "watermark is not valid UTF-8 text".to_owned());
        }
        Err(err) => err,
    };
    image.apply_orientation(orientation);
    let resilient = match password {
        Some(password) => invisible_watermark::extract_resilient_with_password(&image, password),
        None => invisible_watermark::extract_resilient(&image),
    };
    match resilient {
        Ok(text) => Ok(text),
        Err(Error::NoWatermark) => Err(lsb_error.to_string()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};

    use super::*;

    const MIXED_SCRIPTS: &str = "naïve café, Ελλάδα, 東京, Москва";
    const PASSWORD: &str = "correct horse battery staple";

    /// An APP1 segment whose only EXIF entry is orientation 6 (rotate 90 degrees clockwise).
    const EXIF_ORIENTATION_6: [u8; 36] = [
        0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0x00, 0x2A, 0x00, 0x00,
        0x00, 0x08, 0x00, 0x01, 0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x06, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    fn pattern(width: u32, height: u32) -> RgbImage {
        RgbImage::from_fn(width, height, |x, y| {
            Rgb([(x * 4) as u8, (y * 4) as u8, (x + y) as u8])
        })
    }

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

    fn encoded(image: &RgbImage, format: ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(image.clone())
            .write_to(&mut Cursor::new(&mut bytes), format)
            .unwrap();
        bytes
    }

    /// A 54-byte BMP header that declares a 24-bit image of the given size and carries no pixels.
    fn header_only_bmp(width: i32, height: i32) -> Vec<u8> {
        [
            &b"BM"[..],
            &54u32.to_le_bytes(),
            &0u32.to_le_bytes(),
            &54u32.to_le_bytes(),
            &40u32.to_le_bytes(),
            &width.to_le_bytes(),
            &height.to_le_bytes(),
            &1u16.to_le_bytes(),
            &24u16.to_le_bytes(),
            &[0; 24],
        ]
        .concat()
    }

    #[test]
    fn round_trip_without_password_on_png_and_jpeg() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg] {
            let input = encoded(&pattern(64, 64), format);
            let marked = embed_image(&input, MIXED_SCRIPTS, None, false).unwrap();
            assert_eq!(image::guess_format(&marked).unwrap(), ImageFormat::Png);
            assert_eq!(extract_text(&marked, None).unwrap(), MIXED_SCRIPTS);
        }
    }

    #[test]
    fn round_trip_with_password_on_png_and_jpeg() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg] {
            let input = encoded(&pattern(64, 64), format);
            let marked = embed_image(&input, MIXED_SCRIPTS, Some(PASSWORD), false).unwrap();
            assert_eq!(image::guess_format(&marked).unwrap(), ImageFormat::Png);
            assert_eq!(
                extract_text(&marked, Some(PASSWORD)).unwrap(),
                MIXED_SCRIPTS
            );
        }
    }

    #[test]
    fn empty_password_embeds_without_encryption() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed(&input, "hello", Some(String::new()), false).unwrap();
        assert_eq!(extract(&marked, None).unwrap(), "hello");
    }

    #[test]
    fn extract_reports_missing_watermark() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        assert_eq!(
            extract_text(&input, None).unwrap_err(),
            "no watermark found"
        );
    }

    #[test]
    fn extract_of_encrypted_image_requires_a_password() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed_image(&input, "hello", Some(PASSWORD), false).unwrap();
        assert_eq!(
            extract_text(&marked, None).unwrap_err(),
            "watermark is encrypted, a password is required"
        );
    }

    #[test]
    fn extract_with_wrong_password_fails() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed_image(&input, "hello", Some(PASSWORD), false).unwrap();
        assert_eq!(
            extract_text(&marked, Some("wrong password")).unwrap_err(),
            "wrong password or damaged watermark"
        );
    }

    #[test]
    fn extract_with_password_of_plain_watermark_reports_not_encrypted() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed_image(&input, "hello", None, false).unwrap();
        assert_eq!(
            extract_text(&marked, Some(PASSWORD)).unwrap_err(),
            "watermark is not encrypted"
        );
    }

    #[test]
    fn non_image_bytes_cannot_be_read() {
        let expected = "cannot read the image: The image format could not be determined";
        assert_eq!(describe_image(b"not an image").unwrap_err(), expected);
        assert_eq!(
            embed_image(b"not an image", "hello", None, false).unwrap_err(),
            expected
        );
        assert_eq!(extract_text(b"not an image", None).unwrap_err(), expected);
    }

    #[test]
    fn embed_rejects_text_over_capacity() {
        let input = encoded(&pattern(4, 4), ImageFormat::Png);
        assert_eq!(
            embed_image(&input, "© 2026 Example Studio", None, false).unwrap_err(),
            "cannot embed into the image: payload is 22 bytes but the image holds at most 0"
        );
    }

    #[test]
    fn embed_rejects_empty_text() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        assert_eq!(
            embed_image(&input, "", None, false).unwrap_err(),
            "text is empty"
        );
    }

    #[test]
    fn huge_declared_dimensions_are_refused() {
        let bmp = header_only_bmp(20000, 20000);
        let expected = "cannot read the image: Memory limit exceeded";
        assert_eq!(describe_image(&bmp).unwrap_err(), expected);
        assert_eq!(
            embed_image(&bmp, "hello", None, false).unwrap_err(),
            expected
        );
    }

    #[test]
    fn describe_reads_only_the_header() {
        let bmp = header_only_bmp(10000, 10000);
        assert_eq!(
            describe_image(&bmp).unwrap(),
            Description {
                width: 10000,
                height: 10000,
                capacity: 37_499_987,
                capacity_with_password: 37_499_935,
                resilient_capacity: 32,
            }
        );
        assert!(
            extract_text(&bmp, None)
                .unwrap_err()
                .starts_with("cannot read the image: ")
        );
    }

    #[test]
    fn embed_applies_exif_orientation() {
        let mut jpeg = encoded(&pattern(64, 32), ImageFormat::Jpeg);
        let upright = image::load_from_memory(&jpeg)
            .unwrap()
            .rotate90()
            .into_rgb8();
        jpeg.splice(2..2, EXIF_ORIENTATION_6);

        let description = describe_image(&jpeg).unwrap();
        assert_eq!((description.width, description.height), (32, 64));

        let marked = image::load_from_memory(&embed_image(&jpeg, "hello", None, false).unwrap())
            .unwrap()
            .into_rgb8();
        assert_eq!(marked.dimensions(), (32, 64));
        let high_bits =
            |image: &RgbImage| image.iter().map(|sample| sample & !1).collect::<Vec<_>>();
        assert_eq!(high_bits(&marked), high_bits(&upright));
    }

    #[test]
    fn resilient_round_trip_returns_jpeg() {
        let input = encoded(&photo(768, 512), ImageFormat::Png);
        let marked = embed_image(&input, "Example Studio", None, true).unwrap();
        assert_eq!(image::guess_format(&marked).unwrap(), ImageFormat::Jpeg);
        assert_eq!(extract_text(&marked, None).unwrap(), "Example Studio");
    }

    #[test]
    fn resilient_round_trip_with_password() {
        let input = encoded(&photo(768, 512), ImageFormat::Png);
        let marked = embed_image(&input, "Example Studio", Some(PASSWORD), true).unwrap();
        assert_eq!(
            extract_text(&marked, None).unwrap_err(),
            "watermark is encrypted, a password is required"
        );
        assert_eq!(
            extract_text(&marked, Some(PASSWORD)).unwrap(),
            "Example Studio"
        );
    }

    #[test]
    fn resilient_embed_rejects_small_image() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        assert_eq!(
            embed_image(&input, "hello", None, true).unwrap_err(),
            "cannot embed into the image: image is 64 pixels on the short side but the resilient mode needs at least 512"
        );
    }

    #[test]
    fn describe_reports_resilient_capacity() {
        let capacity = |width, height| {
            describe_image(&header_only_bmp(width, height))
                .unwrap()
                .resilient_capacity
        };
        assert_eq!(capacity(512, 511), 0);
        assert_eq!(capacity(512, 512), 32);
    }

    #[test]
    fn extract_applies_exif_orientation_to_resilient_marks() {
        let input = encoded(&photo(768, 512), ImageFormat::Png);
        let marked = embed_image(&input, "Example Studio", None, true).unwrap();
        let rotated = image::load_from_memory(&marked).unwrap().rotate270();
        let mut jpeg = Vec::new();
        rotated
            .write_with_encoder(JpegEncoder::new_with_quality(&mut jpeg, 95))
            .unwrap();
        jpeg.splice(2..2, EXIF_ORIENTATION_6);
        assert_eq!(extract_text(&jpeg, None).unwrap(), "Example Studio");
    }

    #[test]
    fn extract_reads_resilient_mark_when_an_lsb_header_survives() {
        let lsb = invisible_watermark::embed(DynamicImage::ImageRgb8(photo(768, 512)), b"hello");
        let marked = invisible_watermark::embed_resilient(lsb.unwrap(), "Example Studio").unwrap();
        let mut marked = marked.into_rgb8();
        let bits = b"IWMK\x01"
            .iter()
            .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
        for (sample, bit) in marked.iter_mut().zip(bits) {
            *sample = (*sample & !1) | bit;
        }
        let input = encoded(&marked, ImageFormat::Png);
        assert_eq!(
            invisible_watermark::extract(&image::load_from_memory(&input).unwrap()),
            Err(Error::Damaged)
        );
        assert_eq!(extract_text(&input, None).unwrap(), "Example Studio");
    }
}
