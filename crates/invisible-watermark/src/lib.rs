//! Hides a short payload in the least significant bits of an image and reads it back.
//!
//! ```
//! use invisible_watermark::image::{DynamicImage, RgbImage};
//!
//! let image = DynamicImage::ImageRgb8(RgbImage::new(64, 64));
//! let marked = invisible_watermark::embed(image, b"hello")?;
//! assert_eq!(invisible_watermark::extract(&marked)?, b"hello");
//! # Ok::<(), invisible_watermark::Error>(())
//! ```

/// Re-exported so callers build images with the same version of `image`.
pub use image;

use image::DynamicImage;

const MAGIC: [u8; 4] = *b"IWMK";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 9;
const CRC_LEN: usize = 4;
const OVERHEAD: usize = HEADER_LEN + CRC_LEN;

/// Why embedding or extraction failed.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The image does not start with the magic bytes, which is also how a damaged magic looks.
    #[error("no watermark found")]
    NoWatermark,
    /// The record has a format version this library does not know, or a damaged version byte.
    #[error("unsupported watermark format version {0}")]
    UnsupportedVersion(u8),
    /// The stored length does not fit the image, or the checksum does not match.
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
}

fn available_bytes(width: u32, height: u32) -> u128 {
    u128::from(width) * u128::from(height) * 3 / 8
}

/// Whether a record with a payload of `len` bytes fits in the image and in the length field.
fn fits(len: usize, width: u32, height: u32) -> bool {
    let len = len as u128;
    len <= u128::from(u32::MAX) && len + OVERHEAD as u128 <= available_bytes(width, height)
}

/// Payload bytes an image of this size can hold. Images too small for the 13 bytes of header
/// and checksum also report 0, although not even an empty payload fits.
pub fn capacity(width: u32, height: u32) -> usize {
    let bytes = available_bytes(width, height)
        .saturating_sub(OVERHEAD as u128)
        .min(u128::from(u32::MAX));
    usize::try_from(bytes).unwrap_or(usize::MAX)
}

/// Embeds `payload` and returns an 8-bit copy of the image: RGBA if it has alpha, RGB otherwise.
/// Alpha and the bits after the record are left unchanged.
pub fn embed(image: DynamicImage, payload: &[u8]) -> Result<DynamicImage, Error> {
    let (width, height) = (image.width(), image.height());
    let too_large = || Error::PayloadTooLarge {
        len: payload.len(),
        capacity: capacity(width, height),
    };
    if !fits(payload.len(), width, height) {
        return Err(too_large());
    }
    let len = u32::try_from(payload.len()).map_err(|_| too_large())?;

    let mut record = Vec::with_capacity(payload.len() + OVERHEAD);
    record.extend_from_slice(&MAGIC);
    record.push(VERSION);
    record.extend_from_slice(&len.to_be_bytes());
    record.extend_from_slice(payload);
    record.extend_from_slice(&crc32fast::hash(&record).to_be_bytes());

    if image.color().has_alpha() {
        let mut buffer = image.into_rgba8();
        write_record(&mut buffer, 4, &record);
        Ok(DynamicImage::ImageRgba8(buffer))
    } else {
        let mut buffer = image.into_rgb8();
        write_record(&mut buffer, 3, &record);
        Ok(DynamicImage::ImageRgb8(buffer))
    }
}

/// Reads the payload back from a watermarked image.
pub fn extract(image: &DynamicImage) -> Result<Vec<u8>, Error> {
    let (width, height) = (image.width(), image.height());
    match image {
        DynamicImage::ImageRgb8(buffer) => read_record(buffer, 3, width, height),
        DynamicImage::ImageRgba8(buffer) => read_record(buffer, 4, width, height),
        other => read_record(&other.to_rgb8(), 3, width, height),
    }
}

fn write_record(samples: &mut [u8], channels: usize, record: &[u8]) {
    let bits = record
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
    let colors = samples
        .chunks_exact_mut(channels)
        .flat_map(|pixel| pixel.iter_mut().take(3));
    for (sample, bit) in colors.zip(bits) {
        *sample = (*sample & !1) | bit;
    }
}

fn read_record(samples: &[u8], channels: usize, width: u32, height: u32) -> Result<Vec<u8>, Error> {
    let mut lsbs = samples
        .chunks_exact(channels)
        .flat_map(|pixel| pixel.iter().take(3))
        .map(|sample| sample & 1);
    let mut bytes = std::iter::from_fn(move || {
        (0..8).try_fold(0u8, |byte, _| Some((byte << 1) | lsbs.next()?))
    });

    let mut header = [0u8; HEADER_LEN];
    for slot in &mut header {
        *slot = bytes.next().ok_or(Error::NoWatermark)?;
    }
    if header[..4] != MAGIC {
        return Err(Error::NoWatermark);
    }
    if header[4] != VERSION {
        return Err(Error::UnsupportedVersion(header[4]));
    }
    let len = u32::from_be_bytes([header[5], header[6], header[7], header[8]]);
    let len = usize::try_from(len).map_err(|_| Error::Damaged)?;
    if !fits(len, width, height) {
        return Err(Error::Damaged);
    }

    let mut record = Vec::with_capacity(HEADER_LEN + len + CRC_LEN);
    record.extend_from_slice(&header);
    record.extend(bytes.take(len + CRC_LEN));
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
