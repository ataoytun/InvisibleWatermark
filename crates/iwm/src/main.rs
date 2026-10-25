use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Cursor, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{self, ExitCode};

use anyhow::{Context, Result, anyhow, bail};
use clap::builder::NonEmptyStringValueParser;
use clap::{ArgGroup, Parser, Subcommand};
use image::codecs::jpeg::JpegEncoder;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageError, ImageFormat, ImageReader, Limits};
use invisible_watermark::{Error, RESILIENT_MAX_TEXT};

const INPUT_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "webp"];
const JPEG_EXTENSIONS: [&str; 2] = ["jpg", "jpeg"];
const JPEG_QUALITY: u8 = 90;

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
        /// Output PNG file (or JPEG with --resilient), or output directory when PATH is a directory
        #[arg(long)]
        output: Option<PathBuf>,
        /// Overwrite existing output files
        #[arg(long)]
        force: bool,
        /// Encrypt the text with a password from IWM_PASSWORD or a prompt
        #[arg(long)]
        password: bool,
        /// Use the resilient mode, which survives JPEG compression and resizing
        #[arg(long)]
        resilient: bool,
    },
    /// Print the watermark text of an image
    Extract {
        /// Watermarked image
        path: PathBuf,
        /// Ask for a password, or read it from IWM_PASSWORD
        #[arg(long)]
        password: bool,
    },
    /// Check that images carry a readable watermark, without printing the text
    Verify {
        /// Image file or directory
        path: PathBuf,
        /// Ask for the password, or read it from IWM_PASSWORD
        #[arg(long)]
        password: bool,
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
            password,
            resilient,
        } => {
            let text = match text_file {
                Some(file) => read_text_file(&file)?,
                None => text.context("--text or --text-file is required")?,
            };
            embed_path(&path, &text, output.as_deref(), password, resilient, force)
        }
        Command::Extract { path, password } => {
            extract_file(&path, password).map(|()| ExitCode::SUCCESS)
        }
        Command::Verify { path, password } => verify_path(&path, password),
    }
}

const PASSWORD_VAR: &str = "IWM_PASSWORD";

/// Reads the password from `IWM_PASSWORD`, or prompts on the terminal, twice if `confirm` is set.
fn read_password(confirm: bool) -> Result<String> {
    let password = match env::var(PASSWORD_VAR) {
        Ok(password) => password,
        Err(env::VarError::NotUnicode(_)) => bail!("{PASSWORD_VAR} is not valid UTF-8"),
        Err(env::VarError::NotPresent) => {
            if !io::stdin().is_terminal() {
                bail!("no terminal to ask for the password, set {PASSWORD_VAR} instead");
            }
            let password =
                rpassword::prompt_password("Password: ").context("cannot read the password")?;
            if confirm
                && rpassword::prompt_password("Repeat password: ")
                    .context("cannot read the password")?
                    != password
            {
                bail!("passwords do not match");
            }
            password
        }
    };
    if password.is_empty() {
        bail!("password is empty");
    }
    Ok(password)
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

fn embed_path(
    path: &Path,
    text: &str,
    output: Option<&Path>,
    password: bool,
    resilient: bool,
    force: bool,
) -> Result<ExitCode> {
    let metadata = metadata(path)?;
    if resilient && text.len() > RESILIENT_MAX_TEXT {
        bail!("{}", Error::TextTooLong { len: text.len() });
    }
    if metadata.is_dir() {
        let output = output.context("--output is required when PATH is a directory")?;
        return embed_dir(path, output, text, password, resilient, force);
    }

    let output = match output {
        Some(output) => {
            let allowed: &[&str] = if resilient {
                &["png", "jpg", "jpeg"]
            } else {
                &["png"]
            };
            if !has_extension(output, allowed) {
                if resilient {
                    bail!("--output must end in .png, .jpg or .jpeg");
                }
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
    check_overwrite(&output, force)?;
    let password = password.then(|| read_password(true)).transpose()?;
    embed_file(path, &output, text, password.as_deref(), resilient, force)?;
    print_line(format_args!("wrote {}", output.display()))?;
    Ok(ExitCode::SUCCESS)
}

fn check_overwrite(output: &Path, force: bool) -> Result<()> {
    if !force && fs::symlink_metadata(output).is_ok() {
        bail!(
            "{} already exists, use --force to overwrite",
            output.display()
        );
    }
    Ok(())
}

fn embed_file(
    input: &Path,
    output: &Path,
    text: &str,
    password: Option<&str>,
    resilient: bool,
    force: bool,
) -> Result<()> {
    let (mut image, orientation) = decode(input)?;
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
    .with_context(|| format!("cannot embed into {}", input.display()))?;

    let mut bytes = Vec::new();
    if has_extension(output, &JPEG_EXTENSIONS) {
        marked
            .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY))
            .map_err(|err| anyhow!("cannot encode {} as JPEG: {err}", output.display()))?;
    } else {
        marked
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .map_err(|err| anyhow!("cannot encode {} as PNG: {err}", output.display()))?;
    }

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    write_output(output, &bytes, force)
}

/// Creates `path` with `create_new`, or with `force` writes a temporary file next to it and
/// renames it over `path`. On failure the file this call created is removed.
fn write_output(path: &Path, bytes: &[u8], force: bool) -> Result<()> {
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
    let written = file.write_all(bytes);
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

fn embed_dir(
    input: &Path,
    output: &Path,
    text: &str,
    password: bool,
    resilient: bool,
    force: bool,
) -> Result<ExitCode> {
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
    let password = password.then(|| read_password(true)).transpose()?;

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
                check_overwrite(&target, force)?;
                embed_file(
                    &source,
                    &target,
                    text,
                    password.as_deref(),
                    resilient,
                    force,
                )?;
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

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension().is_some_and(|ext| {
        extensions
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

fn is_image(path: &Path) -> bool {
    has_extension(path, &INPUT_EXTENSIONS)
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

fn decode(path: &Path) -> Result<(DynamicImage, Orientation)> {
    let mut decoder = open_decoder(path)?;
    let orientation = decoder.orientation().map_err(|err| read_error(path, err))?;
    let image = DynamicImage::from_decoder(decoder).map_err(|err| read_error(path, err))?;
    Ok((image, orientation))
}

/// A watermark read from an image.
enum Watermark {
    Lsb(Vec<u8>),
    Resilient(String),
}

/// Reads the LSB formats in stored pixel order and, if they fail, the resilient format after
/// EXIF orientation. The LSB error is reported when the image carries no resilient mark either.
fn read_watermark(
    mut image: DynamicImage,
    orientation: Orientation,
    password: Option<&str>,
) -> Result<Watermark, Error> {
    let lsb = match password {
        Some(password) => invisible_watermark::extract_with_password(&image, password),
        None => invisible_watermark::extract(&image),
    };
    let lsb_error = match lsb {
        Ok(bytes) => return Ok(Watermark::Lsb(bytes)),
        Err(err) => err,
    };
    image.apply_orientation(orientation);
    let resilient = match password {
        Some(password) => invisible_watermark::extract_resilient_with_password(&image, password),
        None => invisible_watermark::extract_resilient(&image),
    };
    match resilient {
        Ok(text) => Ok(Watermark::Resilient(text)),
        Err(Error::NoWatermark) => Err(lsb_error),
        Err(err) => Err(err),
    }
}

fn extract_file(path: &Path, password: bool) -> Result<()> {
    let (image, orientation) = decode(path)?;
    let password = password.then(|| read_password(false)).transpose()?;
    let watermark =
        read_watermark(image, orientation, password.as_deref()).map_err(|err| match err {
            Error::PasswordRequired => anyhow!(
                "watermark in {} is encrypted, run with --password",
                path.display()
            ),
            err => anyhow!("{err} in {}", path.display()),
        })?;
    let text = match watermark {
        Watermark::Lsb(bytes) => String::from_utf8(bytes)
            .map_err(|_| anyhow!("watermark in {} is not valid UTF-8 text", path.display()))?,
        Watermark::Resilient(text) => text,
    };
    print_line(format_args!("{text}"))
}

fn metadata(path: &Path) -> Result<fs::Metadata> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            bail!("{} does not exist", path.display())
        }
        Err(err) => Err(err).with_context(|| format!("cannot read {}", path.display())),
    }
}

fn verify_path(path: &Path, password: bool) -> Result<ExitCode> {
    let is_dir = metadata(path)?.is_dir();
    let password = password.then(|| read_password(false)).transpose()?;
    let password = password.as_deref();
    if !is_dir {
        return Ok(if verify_file(path, password)? {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    let mut found = Vec::new();
    let complete = collect_images(path, Path::new(""), &mut found);
    found.sort();

    let mut verified = 0;
    for relative in &found {
        match verify_file(&path.join(relative), password) {
            Ok(true) => verified += 1,
            Ok(false) => {}
            Err(err) => {
                let _ = writeln!(io::stderr(), "error: {err:#}");
            }
        }
    }

    print_line(format_args!(
        "verified {verified} of {} images",
        found.len()
    ))?;
    Ok(if complete && verified == found.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn verify_file(path: &Path, password: Option<&str>) -> Result<bool> {
    let (image, orientation) = decode(path)?;
    let (status, verified) = match read_watermark(image, orientation, password) {
        Ok(Watermark::Lsb(bytes)) if std::str::from_utf8(&bytes).is_err() => {
            ("watermark is not valid UTF-8 text".to_owned(), false)
        }
        Ok(Watermark::Lsb(_)) if password.is_some() => ("ok, encrypted".to_owned(), true),
        Ok(Watermark::Lsb(_)) => ("ok".to_owned(), true),
        Ok(Watermark::Resilient(_)) if password.is_some() => {
            ("ok, resilient, encrypted".to_owned(), true)
        }
        Ok(Watermark::Resilient(_)) => ("ok, resilient".to_owned(), true),
        Err(Error::PasswordRequired) => (
            "encrypted, run with --password to check it".to_owned(),
            false,
        ),
        Err(err) => (err.to_string(), false),
    };
    print_line(format_args!("{}: {status}", path.display()))?;
    Ok(verified)
}
