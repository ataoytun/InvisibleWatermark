//! JavaScript bindings for the browser demo.
//! `describe`, `embed` and `extract` take the raw bytes of an image file.

use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits};
use wasm_bindgen::prelude::*;

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
}

/// Reads the size from the image header, after EXIF orientation, without decoding the pixels.
#[wasm_bindgen]
pub fn describe(bytes: &[u8]) -> Result<Description, JsError> {
    describe_image(bytes).map_err(|err| JsError::new(&err))
}

/// Embeds `text`, encrypted if `password` is set and not empty, and returns the image as PNG.
#[wasm_bindgen]
pub fn embed(bytes: &[u8], text: &str, password: Option<String>) -> Result<Vec<u8>, JsError> {
    embed_png(bytes, text, non_empty(password.as_deref())).map_err(|err| JsError::new(&err))
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
    })
}

fn embed_png(bytes: &[u8], text: &str, password: Option<&str>) -> Result<Vec<u8>, String> {
    if text.is_empty() {
        return Err("text is empty".to_owned());
    }
    let mut decoder = open_decoder(bytes)?;
    let orientation = decoder.orientation().map_err(read_error)?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(read_error)?;
    image.apply_orientation(orientation);
    let marked = match password {
        Some(password) => {
            invisible_watermark::embed_with_password(image, text.as_bytes(), password)
        }
        None => invisible_watermark::embed(image, text.as_bytes()),
    }
    .map_err(|err| format!("cannot embed into the image: {err}"))?;

    let mut png = Cursor::new(Vec::new());
    marked
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|err| format!("cannot encode the image as PNG: {err}"))?;
    Ok(png.into_inner())
}

fn extract_text(bytes: &[u8], password: Option<&str>) -> Result<String, String> {
    let image = DynamicImage::from_decoder(open_decoder(bytes)?).map_err(read_error)?;
    let payload = match password {
        Some(password) => invisible_watermark::extract_with_password(&image, password),
        None => invisible_watermark::extract(&image),
    }
    .map_err(|err| err.to_string())?;
    String::from_utf8(payload).map_err(|_| "watermark is not valid UTF-8 text".to_owned())
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
            let marked = embed_png(&input, MIXED_SCRIPTS, None).unwrap();
            assert_eq!(image::guess_format(&marked).unwrap(), ImageFormat::Png);
            assert_eq!(extract_text(&marked, None).unwrap(), MIXED_SCRIPTS);
        }
    }

    #[test]
    fn round_trip_with_password_on_png_and_jpeg() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg] {
            let input = encoded(&pattern(64, 64), format);
            let marked = embed_png(&input, MIXED_SCRIPTS, Some(PASSWORD)).unwrap();
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
        let marked = embed(&input, "hello", Some(String::new())).unwrap();
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
        let marked = embed_png(&input, "hello", Some(PASSWORD)).unwrap();
        assert_eq!(
            extract_text(&marked, None).unwrap_err(),
            "watermark is encrypted, a password is required"
        );
    }

    #[test]
    fn extract_with_wrong_password_fails() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed_png(&input, "hello", Some(PASSWORD)).unwrap();
        assert_eq!(
            extract_text(&marked, Some("wrong password")).unwrap_err(),
            "wrong password or damaged watermark"
        );
    }

    #[test]
    fn extract_with_password_of_plain_watermark_reports_not_encrypted() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        let marked = embed_png(&input, "hello", None).unwrap();
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
            embed_png(b"not an image", "hello", None).unwrap_err(),
            expected
        );
        assert_eq!(extract_text(b"not an image", None).unwrap_err(), expected);
    }

    #[test]
    fn embed_rejects_text_over_capacity() {
        let input = encoded(&pattern(4, 4), ImageFormat::Png);
        assert_eq!(
            embed_png(&input, "© 2026 Example Studio", None).unwrap_err(),
            "cannot embed into the image: payload is 22 bytes but the image holds at most 0"
        );
    }

    #[test]
    fn embed_rejects_empty_text() {
        let input = encoded(&pattern(64, 64), ImageFormat::Png);
        assert_eq!(embed_png(&input, "", None).unwrap_err(), "text is empty");
    }

    #[test]
    fn huge_declared_dimensions_are_refused() {
        let bmp = header_only_bmp(20000, 20000);
        let expected = "cannot read the image: Memory limit exceeded";
        assert_eq!(describe_image(&bmp).unwrap_err(), expected);
        assert_eq!(embed_png(&bmp, "hello", None).unwrap_err(), expected);
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

        let marked = image::load_from_memory(&embed_png(&jpeg, "hello", None).unwrap())
            .unwrap()
            .into_rgb8();
        assert_eq!(marked.dimensions(), (32, 64));
        let high_bits =
            |image: &RgbImage| image.iter().map(|sample| sample & !1).collect::<Vec<_>>();
        assert_eq!(high_bits(&marked), high_bits(&upright));
    }
}
