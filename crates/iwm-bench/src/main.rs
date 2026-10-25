//! Embeds a resilient watermark into every image in a directory, applies common edits and prints how often the text reads back.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{
    DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, ImageReader, Rgb, RgbImage,
};
use invisible_watermark::{embed_resilient, extract_resilient};

const TEXTS: [&str; 4] = [
    "Example Studio 2026 photo.jpg #1",
    "https://example.com/photo/000042",
    "© 例 Пример Studio 2026!",
    "0123456789 ABCDEF abcdef !#$%&*+",
];
const WINDOW: usize = 32;
const FLAT_SHARE: f64 = 0.05;

#[derive(Clone, Copy)]
enum Attack {
    None,
    Jpeg(u8),
    Resize(u32),
    Resize50Jpeg75,
    Crop5,
    Rotate1,
}

/// An attack, its table name and the success rate the resilient mode is expected to reach.
const ATTACKS: [(Attack, &str, Option<f64>); 11] = [
    (Attack::None, "none", None),
    (Attack::Jpeg(90), "jpeg90", Some(0.99)),
    (Attack::Jpeg(75), "jpeg75", Some(0.99)),
    (Attack::Jpeg(50), "jpeg50", Some(0.99)),
    (Attack::Jpeg(30), "jpeg30", None),
    (Attack::Resize(50), "resize50", Some(0.99)),
    (Attack::Resize(75), "resize75", Some(0.99)),
    (Attack::Resize(150), "resize150", Some(0.99)),
    (Attack::Resize50Jpeg75, "resize50-jpeg75", Some(0.95)),
    (Attack::Crop5, "crop5", None),
    (Attack::Rotate1, "rotate1", None),
];

#[derive(Default)]
struct Tally {
    successes: usize,
    failed_images: BTreeSet<String>,
}

fn main() -> Result<ExitCode> {
    let mut args = std::env::args().skip(1);
    let (Some(dir), None) = (args.next(), args.next()) else {
        eprintln!("usage: iwm-bench <image dir>");
        return Ok(ExitCode::from(2));
    };
    let dir = PathBuf::from(dir);
    let paths = image_paths(&dir)?;
    if paths.is_empty() {
        bail!("no PNG or JPEG images in {}", dir.display());
    }

    let mut tallies: Vec<Tally> = ATTACKS.iter().map(|_| Tally::default()).collect();
    let mut min_psnr = (f64::INFINITY, String::new());
    let mut max_flat_rms = (0.0, String::new());
    let (mut embed_time, mut extract_time) = (Duration::ZERO, Duration::ZERO);
    for path in &paths {
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let original =
            read_image(path).with_context(|| format!("cannot read {}", path.display()))?;
        for text in TEXTS {
            let input = DynamicImage::ImageRgb8(original.clone());
            let started = Instant::now();
            let marked = embed_resilient(input, text)
                .with_context(|| format!("cannot embed into {}", path.display()))?
                .into_rgb8();
            embed_time += started.elapsed();

            let psnr = psnr(&original, &marked);
            if psnr < min_psnr.0 {
                min_psnr = (psnr, stem.clone());
            }
            let rms = flat_area_rms(&original, &marked);
            if rms > max_flat_rms.0 {
                max_flat_rms = (rms, stem.clone());
            }

            for (tally, &(attack, name, _)) in tallies.iter_mut().zip(&ATTACKS) {
                let attacked = apply(attack, &marked)
                    .with_context(|| format!("cannot apply {name} to {}", path.display()))?;
                let started = Instant::now();
                let read = extract_resilient(&DynamicImage::ImageRgb8(attacked));
                if matches!(attack, Attack::None) {
                    extract_time += started.elapsed();
                }
                if read.as_deref() == Ok(text) {
                    tally.successes += 1;
                } else {
                    tally.failed_images.insert(stem.clone());
                }
            }
        }
    }

    let trials = paths.len() * TEXTS.len();
    println!(
        "{} images in {}, {} texts, {trials} trials per attack.\n",
        paths.len(),
        dir.display(),
        TEXTS.len()
    );
    println!("|Attack|Success|Failed images|\n|---|---|---|");
    let (mut met, mut targets) = (0, 0);
    for (tally, &(_, name, target)) in tallies.iter().zip(&ATTACKS) {
        if let Some(target) = target {
            targets += 1;
            met += usize::from(tally.successes as f64 / trials as f64 >= target);
        }
        println!(
            "|{name}|{}|{}|",
            success_cell(tally.successes, trials),
            failed_cell(&tally.failed_images, paths.len())
        );
    }
    println!("\nTargets met: {met}/{targets}");
    println!("PSNR min: {:.2} dB ({})", min_psnr.0, min_psnr.1);
    println!(
        "Worst flat-area RMS: {:.3} ({})",
        max_flat_rms.0, max_flat_rms.1
    );
    println!(
        "Mean time per image: embed {:.1} ms, extract {:.1} ms (single thread, decoded images in memory)",
        embed_time.as_secs_f64() * 1000.0 / trials as f64,
        extract_time.as_secs_f64() * 1000.0 / trials as f64
    );
    Ok(ExitCode::SUCCESS)
}

fn read_image(path: &Path) -> Result<RgbImage> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgb8())
}

fn image_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("cannot read {}", dir.display()))?
            .path();
        let is_image = path.is_file()
            && path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| {
                    ["png", "jpg", "jpeg"]
                        .iter()
                        .any(|known| extension.eq_ignore_ascii_case(known))
                });
        if is_image {
            paths.push(path);
        }
    }
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(paths)
}

fn apply(attack: Attack, image: &RgbImage) -> Result<RgbImage> {
    Ok(match attack {
        Attack::None => image.clone(),
        Attack::Jpeg(quality) => jpeg(image, quality)?,
        Attack::Resize(percent) => resize(image, percent),
        Attack::Resize50Jpeg75 => jpeg(&resize(image, 50), 75)?,
        Attack::Crop5 => {
            let x0 = (f64::from(image.width()) * 0.05).round() as u32;
            let y0 = (f64::from(image.height()) * 0.05).round() as u32;
            imageops::crop_imm(
                image,
                x0,
                y0,
                image.width() - 2 * x0,
                image.height() - 2 * y0,
            )
            .to_image()
        }
        Attack::Rotate1 => rotate(image, 1.0),
    })
}

fn jpeg(image: &RgbImage, quality: u8) -> Result<RgbImage> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, quality).write_image(
        image,
        image.width(),
        image.height(),
        ExtendedColorType::Rgb8,
    )?;
    Ok(image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg)?.into_rgb8())
}

fn resize(image: &RgbImage, percent: u32) -> RgbImage {
    let scaled = |side: u32| (f64::from(side) * f64::from(percent) / 100.0).round() as u32;
    imageops::resize(
        image,
        scaled(image.width()),
        scaled(image.height()),
        FilterType::Lanczos3,
    )
}

/// Bilinear rotation about the center on the same canvas; pixels from outside the image count as black.
fn rotate(image: &RgbImage, degrees: f64) -> RgbImage {
    let (width, height) = (image.width(), image.height());
    let (cx, cy) = (
        (f64::from(width) - 1.0) / 2.0,
        (f64::from(height) - 1.0) / 2.0,
    );
    let (sin, cos) = degrees.to_radians().sin_cos();
    let at = |x: i64, y: i64, channel: usize| {
        if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
            0.0
        } else {
            f64::from(image.get_pixel(x as u32, y as u32)[channel])
        }
    };
    RgbImage::from_fn(width, height, |x, y| {
        let (dx, dy) = (f64::from(x) - cx, f64::from(y) - cy);
        let source_x = cos * dx + sin * dy + cx;
        let source_y = -sin * dx + cos * dy + cy;
        let (x0, y0) = (source_x.floor(), source_y.floor());
        let (fx, fy) = (source_x - x0, source_y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mut pixel = [0; 3];
        for (channel, out) in pixel.iter_mut().enumerate() {
            let top = at(x0, y0, channel) * (1.0 - fx) + at(x0 + 1, y0, channel) * fx;
            let bottom = at(x0, y0 + 1, channel) * (1.0 - fx) + at(x0 + 1, y0 + 1, channel) * fx;
            *out = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
        }
        Rgb(pixel)
    })
}

fn psnr(original: &RgbImage, marked: &RgbImage) -> f64 {
    let squares: f64 = original
        .as_raw()
        .iter()
        .zip(marked.as_raw())
        .map(|(&a, &b)| (f64::from(a) - f64::from(b)).powi(2))
        .sum();
    let mse = squares / original.as_raw().len() as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

fn luma(image: &RgbImage, x: usize, y: usize) -> f64 {
    let [r, g, b] = image.get_pixel(x as u32, y as u32).0;
    0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)
}

/// RMS of the luma change over the flattest 5% of the 32x32 windows of `original`.
fn flat_area_rms(original: &RgbImage, marked: &RgbImage) -> f64 {
    let (columns, rows) = (
        original.width() as usize / WINDOW,
        original.height() as usize / WINDOW,
    );
    let mut windows = Vec::with_capacity(columns * rows);
    for wy in 0..rows {
        for wx in 0..columns {
            let (mut sum, mut sum_squares) = (0.0, 0.0);
            for y in wy * WINDOW..(wy + 1) * WINDOW {
                for x in wx * WINDOW..(wx + 1) * WINDOW {
                    let value = luma(original, x, y);
                    sum += value;
                    sum_squares += value * value;
                }
            }
            let n = (WINDOW * WINDOW) as f64;
            windows.push((sum_squares / n - (sum / n) * (sum / n), wx, wy));
        }
    }
    windows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let flattest = ((FLAT_SHARE * windows.len() as f64).ceil() as usize).max(1);
    let (mut squares, mut count) = (0.0, 0usize);
    for &(_, wx, wy) in windows.iter().take(flattest) {
        for y in wy * WINDOW..(wy + 1) * WINDOW {
            for x in wx * WINDOW..(wx + 1) * WINDOW {
                let difference = luma(marked, x, y) - luma(original, x, y);
                squares += difference * difference;
                count += 1;
            }
        }
    }
    (squares / count as f64).sqrt()
}

fn success_cell(successes: usize, trials: usize) -> String {
    if successes == trials {
        format!("{successes}/{trials} (100%)")
    } else {
        format!(
            "{successes}/{trials} ({:.1}%)",
            100.0 * successes as f64 / trials as f64
        )
    }
}

fn failed_cell(failed: &BTreeSet<String>, images: usize) -> String {
    if failed.is_empty() {
        "none".to_owned()
    } else if failed.len() == images {
        "all".to_owned()
    } else {
        failed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
