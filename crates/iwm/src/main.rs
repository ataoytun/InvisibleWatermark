use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};

use anyhow::{Context, Result, anyhow, bail};
use clap::builder::NonEmptyStringValueParser;
use clap::{ArgGroup, Parser, Subcommand};
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits};

const INPUT_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "webp"];

/// Hide a short text watermark in an image and read it back.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Embed text into an image, or into every image under a directory
    #[command(group(ArgGroup::new("source").required(true).args(["text", "text_file"])))]
    Embed {
        /// Image file or directory
        path: PathBuf,
        /// Text to embed
        #[arg(long, value_parser = NonEmptyStringValueParser::new())]
        text: Option<String>,
        /// Read the text from a UTF-8 file, exactly as stored
        #[arg(long, value_name = "PATH")]
        text_file: Option<PathBuf>,
        /// Output PNG file, or output directory when PATH is a directory
        #[arg(long)]
        output: Option<PathBuf>,
        /// Overwrite existing output files
        #[arg(long)]
        force: bool,
    },
    /// Print the watermark text of an image
    Extract {
        /// Watermarked image
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(io::stderr(), "error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Embed {
            path,
            text,
            text_file,
            output,
            force,
        } => {
            let text = match text_file {
                Some(file) => read_text_file(&file)?,
                None => text.context("--text or --text-file is required")?,
            };
            embed_path(&path, &text, output.as_deref(), force)
        }
        Command::Extract { path } => extract_file(&path).map(|()| ExitCode::SUCCESS),
    }
}

fn read_text_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| anyhow!("{} is not valid UTF-8 text", path.display()))?;
    if text.is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(text)
}

fn embed_path(path: &Path, text: &str, output: Option<&Path>, force: bool) -> Result<ExitCode> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            bail!("{} does not exist", path.display())
        }
        Err(err) => return Err(err).with_context(|| format!("cannot read {}", path.display())),
    };
    if metadata.is_dir() {
        let output = output.context("--output is required when PATH is a directory")?;
        return embed_dir(path, output, text.as_bytes(), force);
    }

    let output = match output {
        Some(output) => {
            if !output
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
            {
                bail!(
                    "--output must end in .png: lossy formats such as JPEG destroy the watermark"
                );
            }
            output.to_path_buf()
        }
        None => {
            let mut name = path.file_stem().unwrap_or_default().to_os_string();
            name.push(".wm.png");
            path.with_file_name(name)
        }
    };
    embed_file(path, &output, text.as_bytes(), force)?;
    print_line(format_args!("wrote {}", output.display()))?;
    Ok(ExitCode::SUCCESS)
}

fn embed_file(input: &Path, output: &Path, payload: &[u8], force: bool) -> Result<()> {
    if !force && fs::symlink_metadata(output).is_ok() {
        bail!(
            "{} already exists, use --force to overwrite",
            output.display()
        );
    }
    let mut decoder = open_decoder(input)?;
    let orientation = decoder
        .orientation()
        .map_err(|err| read_error(input, err))?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|err| read_error(input, err))?;
    image.apply_orientation(orientation);
    let marked = invisible_watermark::embed(image, payload)
        .with_context(|| format!("cannot embed into {}", input.display()))?;

    let mut png = Cursor::new(Vec::new());
    marked
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|err| anyhow!("cannot encode {} as PNG: {err}", output.display()))?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    write_png(output, png.get_ref(), force)
}

/// Creates `path` with `create_new`, or with `force` writes a temporary file next to it and
/// renames it over `path`. On failure the file this call created is removed.
fn write_png(path: &Path, png: &[u8], force: bool) -> Result<()> {
    let target = if force {
        let mut name = OsString::from(".");
        name.push(path.file_name().context("output path has no file name")?);
        name.push(format!(".{}.tmp", process::id()));
        path.with_file_name(name)
    } else {
        path.to_path_buf()
    };
    let mut file =
        File::create_new(&target).with_context(|| format!("cannot create {}", target.display()))?;
    let written = file.write_all(png);
    drop(file);
    let result = written.and_then(|()| {
        if force {
            fs::rename(&target, path)
        } else {
            Ok(())
        }
    });
    if result.is_err() {
        let _ = fs::remove_file(&target);
    }
    result.with_context(|| format!("cannot write {}", path.display()))
}

fn embed_dir(input: &Path, output: &Path, payload: &[u8], force: bool) -> Result<ExitCode> {
    let input_dir = input
        .canonicalize()
        .with_context(|| format!("cannot read {}", input.display()))?;
    let output_dir =
        resolve(output).with_context(|| format!("cannot resolve {}", output.display()))?;
    if output_dir.starts_with(&input_dir) {
        bail!(
            "output directory {} is inside the input directory {}",
            output.display(),
            input.display()
        );
    }

    let mut found = Vec::new();
    let complete = collect_images(input, Path::new(""), &mut found);
    found.sort();

    let mut written = HashSet::new();
    let mut embedded = 0;
    for relative in &found {
        let source = input.join(relative);
        let target = output.join(relative).with_extension("png");
        let result = resolve(&target)
            .with_context(|| format!("cannot resolve {}", target.display()))
            .and_then(|key| {
                if key.starts_with(&input_dir) {
                    bail!(
                        "{} maps to {}, which is inside the input directory",
                        source.display(),
                        target.display()
                    );
                }
                if written.contains(&key) {
                    bail!(
                        "{} maps to {}, which this run already wrote",
                        source.display(),
                        target.display()
                    );
                }
                embed_file(&source, &target, payload, force)?;
                target
                    .canonicalize()
                    .with_context(|| format!("cannot resolve {}", target.display()))
            });
        match result {
            Ok(key) => {
                print_line(format_args!("wrote {}", target.display()))?;
                embedded += 1;
                written.insert(key);
            }
            Err(err) => {
                let _ = writeln!(io::stderr(), "error: {err:#}");
            }
        }
    }

    print_line(format_args!(
        "embedded {embedded} of {} images",
        found.len()
    ))?;
    Ok(if complete && embedded == found.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Canonicalizes a path that may not exist yet by resolving its longest existing ancestor.
fn resolve(path: &Path) -> io::Result<PathBuf> {
    let mut existing = std::path::absolute(path)?;
    let mut missing: Vec<OsString> = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            break;
        };
        missing.push(name.to_os_string());
        existing.pop();
    }
    let mut resolved = existing.canonicalize()?;
    resolved.extend(missing.iter().rev());
    Ok(resolved)
}

/// Collects image paths under `root`, relative to `root`. Directories that cannot be read are
/// reported on stderr and skipped; the result is false if that happened.
fn collect_images(root: &Path, relative: &Path, found: &mut Vec<PathBuf>) -> bool {
    let dir = root.join(relative);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            let _ = writeln!(
                io::stderr(),
                "error: cannot read directory {}: {err}",
                dir.display()
            );
            return false;
        }
    };
    let mut complete = true;
    for entry in entries {
        let (entry, file_type) =
            match entry.and_then(|entry| entry.file_type().map(|file_type| (entry, file_type))) {
                Ok(pair) => pair,
                Err(err) => {
                    let _ = writeln!(
                        io::stderr(),
                        "error: cannot read directory {}: {err}",
                        dir.display()
                    );
                    complete = false;
                    break;
                }
            };
        let path = relative.join(entry.file_name());
        // Symlinks are skipped so a link cycle cannot loop the walk.
        if file_type.is_dir() {
            complete &= collect_images(root, &path, found);
        } else if file_type.is_file() && is_image(&path) {
            found.push(path);
        } else if file_type.is_symlink() && is_image(&path) {
            let _ = writeln!(
                io::stderr(),
                "skipped {}: symlinks are not followed",
                root.join(&path).display()
            );
        }
    }
    complete
}

fn is_image(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        INPUT_EXTENSIONS
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

/// Opens a decoder for `path`, detecting the format from the file content. Images whose
/// decoded size exceeds the default allocation limit of `image` are refused.
fn open_decoder(path: &Path) -> Result<impl ImageDecoder> {
    let decoder = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .with_context(|| format!("cannot read {}", path.display()))?
        .into_decoder()
        .map_err(|err| read_error(path, err))?;
    Limits::default()
        .reserve(decoder.total_bytes())
        .map_err(|err| read_error(path, err))?;
    Ok(decoder)
}

/// Formats the error by hand because `ImageError` already prints its source in its message.
fn read_error(path: &Path, err: ImageError) -> anyhow::Error {
    anyhow!("cannot read {}: {err}", path.display())
}

/// Prints a line to stdout and treats a closed pipe as success.
fn print_line(line: fmt::Arguments) -> Result<()> {
    match writeln!(io::stdout(), "{line}") {
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.context("cannot write to stdout"),
    }
}

fn extract_file(path: &Path) -> Result<()> {
    let image =
        DynamicImage::from_decoder(open_decoder(path)?).map_err(|err| read_error(path, err))?;
    let bytes = invisible_watermark::extract(&image)
        .map_err(|err| anyhow!("{err} in {}", path.display()))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| anyhow!("watermark in {} is not valid UTF-8 text", path.display()))?;
    print_line(format_args!("{text}"))
}
