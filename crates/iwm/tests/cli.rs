use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use invisible_watermark::Error;

const MIXED_SCRIPTS: &str = "naïve café, Ελλάδα, 東京, Москва";
const EMOJI: &str = "🦀 ok 👍🏽";
const PASSWORD: &str = "correct horse battery staple";

/// An APP1 segment whose only EXIF entry is orientation 6 (rotate 90 degrees clockwise).
const EXIF_ORIENTATION_6: [u8; 36] = [
    0xFF, 0xE1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'M', b'M', 0x00, 0x2A, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x01, 0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x06, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00,
];

/// A 54-byte BMP header that declares a 20000×20000 24-bit image and carries no pixels.
fn huge_bmp() -> Vec<u8> {
    [
        &b"BM"[..],
        &54u32.to_le_bytes(),
        &0u32.to_le_bytes(),
        &54u32.to_le_bytes(),
        &40u32.to_le_bytes(),
        &20000i32.to_le_bytes(),
        &20000i32.to_le_bytes(),
        &1u16.to_le_bytes(),
        &24u16.to_le_bytes(),
        &[0; 24],
    ]
    .concat()
}

/// An empty directory unique to the test.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if let Err(err) = fs::remove_dir_all(&dir) {
        assert_eq!(
            err.kind(),
            io::ErrorKind::NotFound,
            "cannot clear {}: {err}",
            dir.display()
        );
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn iwm_command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_iwm"));
    command
        .args(args)
        .current_dir(dir)
        .env_remove("IWM_PASSWORD");
    command
}

/// Runs `iwm` with `dir` as the working directory.
fn iwm(dir: &Path, args: &[&str]) -> Output {
    iwm_command(dir, args).output().unwrap()
}

fn iwm_with_password(dir: &Path, args: &[&str], password: &str) -> Output {
    iwm_command(dir, args)
        .env("IWM_PASSWORD", password)
        .output()
        .unwrap()
}

fn pattern(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x * 4) as u8, (y * 4) as u8, (x + y) as u8])
    })
}

fn save_image(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    pattern(64, 64).save(path).unwrap();
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
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

fn save_photo(path: &Path, width: u32, height: u32) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    photo(width, height).save(path).unwrap();
}

#[test]
fn embed_and_extract_round_trip_through_files() {
    let dir = scratch("round_trip");
    save_image(&dir.join("photo.jpg"));

    for text in [MIXED_SCRIPTS, EMOJI] {
        let embed = iwm(&dir, &["embed", "photo.jpg", "--text", text, "--force"]);
        assert!(embed.status.success(), "{}", stderr(&embed));
        assert_eq!(stdout(&embed), "wrote photo.wm.png\n");
        assert!(dir.join("photo.wm.png").is_file());

        let extract = iwm(&dir, &["extract", "photo.wm.png"]);
        assert!(extract.status.success(), "{}", stderr(&extract));
        assert_eq!(stdout(&extract), format!("{text}\n"));
    }
}

#[test]
fn embed_rejects_empty_text() {
    let dir = scratch("empty_text");
    save_image(&dir.join("photo.png"));

    let embed = iwm(&dir, &["embed", "photo.png", "--text", ""]);
    assert_eq!(embed.status.code(), Some(2));
}

#[test]
fn embed_reads_text_file_exactly() {
    let dir = scratch("text_file");
    save_image(&dir.join("photo.png"));
    fs::write(dir.join("notice.txt"), "Copyright 2026 Example Studio\n").unwrap();

    let embed = iwm(&dir, &["embed", "photo.png", "--text-file", "notice.txt"]);
    assert!(embed.status.success(), "{}", stderr(&embed));

    let extract = iwm(&dir, &["extract", "photo.wm.png"]);
    assert!(extract.status.success(), "{}", stderr(&extract));
    assert_eq!(stdout(&extract), "Copyright 2026 Example Studio\n\n");
}

#[test]
fn embed_rejects_text_together_with_text_file() {
    let dir = scratch("text_and_text_file");
    save_image(&dir.join("photo.png"));
    fs::write(dir.join("notice.txt"), "hello").unwrap();

    let embed = iwm(
        &dir,
        &[
            "embed",
            "photo.png",
            "--text",
            "hello",
            "--text-file",
            "notice.txt",
        ],
    );
    assert_eq!(embed.status.code(), Some(2));
    assert!(stderr(&embed).contains("cannot be used with"));
    assert!(!dir.join("photo.wm.png").exists());
}

#[test]
fn embed_reports_missing_path() {
    let dir = scratch("missing_path");

    let embed = iwm(&dir, &["embed", "missing.jpg", "--text", "hello"]);
    assert!(!embed.status.success());
    assert_eq!(stderr(&embed), "error: missing.jpg does not exist\n");
}

#[test]
fn embed_rejects_huge_declared_dimensions() {
    let dir = scratch("huge_dimensions");
    fs::write(dir.join("huge.bmp"), huge_bmp()).unwrap();

    let embed = iwm(&dir, &["embed", "huge.bmp", "--text", "hello"]);
    assert!(!embed.status.success());
    assert_eq!(
        stderr(&embed),
        "error: cannot read huge.bmp: Memory limit exceeded\n"
    );
    assert!(!dir.join("huge.wm.png").exists());
}

#[test]
fn embed_directory_keeps_same_named_files_apart() {
    let dir = scratch("directory");
    save_image(&dir.join("in/a/x.jpg"));
    save_image(&dir.join("in/b/x.jpg"));
    save_image(&dir.join("in/b/Y.PNG"));
    fs::write(dir.join("in/b/notes.txt"), "not an image").unwrap();

    let embed = iwm(
        &dir,
        &["embed", "in", "--output", "out", "--text", MIXED_SCRIPTS],
    );
    assert!(embed.status.success(), "{}", stderr(&embed));
    let wrote = |dir: &str, name: &str| {
        format!(
            "wrote {}\n",
            Path::new("out").join(dir).join(name).display()
        )
    };
    assert_eq!(
        stdout(&embed),
        wrote("a", "x.png")
            + &wrote("b", "Y.png")
            + &wrote("b", "x.png")
            + "embedded 3 of 3 images\n"
    );

    for name in ["a/x.png", "b/x.png", "b/Y.png"] {
        assert!(dir.join("out").join(name).is_file(), "{name} is missing");
    }
    assert!(!dir.join("out/b/notes.txt").exists());
    assert!(!dir.join("out/b/notes.png").exists());
    for name in ["out/a/x.png", "out/b/x.png"] {
        let extract = iwm(&dir, &["extract", name]);
        assert_eq!(stdout(&extract), format!("{MIXED_SCRIPTS}\n"));
    }
}

#[test]
fn embed_directory_reports_failures_and_continues() {
    let dir = scratch("directory_failure");
    save_image(&dir.join("in/good.png"));
    fs::write(dir.join("in/bad.png"), b"not an image").unwrap();

    let embed = iwm(&dir, &["embed", "in", "--output", "out", "--text", "hello"]);
    assert!(!embed.status.success());
    assert_eq!(
        stderr(&embed),
        format!(
            "error: cannot read {}: Format error decoding Png: Invalid PNG signature.\n",
            Path::new("in").join("bad.png").display()
        )
    );
    assert!(stdout(&embed).contains("embedded 1 of 2 images"));
    assert!(dir.join("out/good.png").is_file());
    assert!(!dir.join("out/bad.png").exists());
}

#[test]
fn embed_directory_continues_after_huge_declared_dimensions() {
    let dir = scratch("directory_huge_dimensions");
    save_image(&dir.join("in/good.png"));
    fs::write(dir.join("in/huge.bmp"), huge_bmp()).unwrap();

    let embed = iwm(&dir, &["embed", "in", "--output", "out", "--text", "hello"]);
    assert!(!embed.status.success());
    assert_eq!(
        stderr(&embed),
        format!(
            "error: cannot read {}: Memory limit exceeded\n",
            Path::new("in").join("huge.bmp").display()
        )
    );
    assert!(stdout(&embed).contains("embedded 1 of 2 images"));
    assert!(dir.join("out/good.png").is_file());
}

#[test]
fn embed_directory_rejects_two_inputs_with_the_same_output() {
    let dir = scratch("directory_collision");
    save_image(&dir.join("in/x.jpg"));
    pattern(32, 32).save(dir.join("in/x.png")).unwrap();

    let embed = iwm(
        &dir,
        &[
            "embed", "in", "--output", "out", "--text", "hello", "--force",
        ],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("already wrote"));
    assert!(stdout(&embed).contains("embedded 1 of 2 images"));
    assert_eq!(
        image::image_dimensions(dir.join("out/x.png")).unwrap(),
        (64, 64)
    );
}

#[test]
fn embed_directory_rejects_output_inside_input() {
    let dir = scratch("directory_nested");
    save_image(&dir.join("in/x.png"));

    let embed = iwm(
        &dir,
        &["embed", "in", "--output", "in/out", "--text", "hello"],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("inside the input directory"));
    assert!(!dir.join("in/out").exists());
}

#[test]
fn embed_directory_never_overwrites_an_input() {
    let dir = scratch("directory_overwrite");
    save_image(&dir.join("photos/x.png"));
    save_image(&dir.join("photos/photos/x.png"));
    let before = fs::read(dir.join("photos/x.png")).unwrap();

    let embed = iwm(
        &dir,
        &[
            "embed", "photos", "--output", ".", "--text", "hello", "--force",
        ],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("inside the input directory"));
    assert_eq!(fs::read(dir.join("photos/x.png")).unwrap(), before);
}

#[test]
fn embed_directory_detects_case_only_collisions() {
    let dir = scratch("directory_case");
    save_image(&dir.join("in/X.jpg"));
    pattern(32, 32).save(dir.join("in/x.png")).unwrap();
    fs::write(dir.join("probe"), "").unwrap();
    let case_insensitive = dir.join("PROBE").exists();

    let embed = iwm(&dir, &["embed", "in", "--output", "out", "--text", "hello"]);
    if case_insensitive {
        assert!(!embed.status.success());
        assert!(stderr(&embed).contains("already wrote"));
        assert_eq!(
            image::image_dimensions(dir.join("out/X.png")).unwrap(),
            (64, 64)
        );
    } else {
        assert!(embed.status.success(), "{}", stderr(&embed));
        assert!(dir.join("out/X.png").is_file());
        assert!(dir.join("out/x.png").is_file());
    }
}

#[test]
fn embed_with_force_detects_case_only_collision_with_existing_output() {
    let dir = scratch("directory_case_existing");
    save_image(&dir.join("in/X.jpg"));
    pattern(32, 32).save(dir.join("in/x.png")).unwrap();
    fs::create_dir(dir.join("out")).unwrap();
    pattern(16, 16).save(dir.join("out/x.png")).unwrap();
    fs::write(dir.join("probe"), "").unwrap();
    let case_insensitive = dir.join("PROBE").exists();

    let embed = iwm(
        &dir,
        &[
            "embed", "in", "--output", "out", "--text", "hello", "--force",
        ],
    );
    if case_insensitive {
        assert!(!embed.status.success());
        assert!(stderr(&embed).contains("already wrote"));
        assert!(stdout(&embed).contains("embedded 1 of 2 images"));
        assert_eq!(
            image::image_dimensions(dir.join("out/x.png")).unwrap(),
            (64, 64)
        );
    } else {
        assert!(embed.status.success(), "{}", stderr(&embed));
        assert_eq!(
            image::image_dimensions(dir.join("out/X.png")).unwrap(),
            (64, 64)
        );
        assert_eq!(
            image::image_dimensions(dir.join("out/x.png")).unwrap(),
            (32, 32)
        );
    }
}

#[test]
fn embed_refuses_to_overwrite_without_force() {
    let dir = scratch("overwrite");
    save_image(&dir.join("photo.png"));

    assert!(
        iwm(&dir, &["embed", "photo.png", "--text", "one"])
            .status
            .success()
    );
    let second = iwm(&dir, &["embed", "photo.png", "--text", "two"]);
    assert!(!second.status.success());
    assert!(stderr(&second).contains("already exists"));
    assert_eq!(stdout(&iwm(&dir, &["extract", "photo.wm.png"])), "one\n");

    let forced = iwm(&dir, &["embed", "photo.png", "--text", "two", "--force"]);
    assert!(forced.status.success(), "{}", stderr(&forced));
    assert_eq!(stdout(&iwm(&dir, &["extract", "photo.wm.png"])), "two\n");
}

#[test]
fn embed_rejects_non_png_output() {
    let dir = scratch("non_png_output");
    save_image(&dir.join("photo.png"));

    let embed = iwm(
        &dir,
        &["embed", "photo.png", "--text", "x", "--output", "out.jpg"],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("lossy"));
    assert!(!dir.join("out.jpg").exists());
}

#[test]
fn embed_keeps_dots_in_the_stem() {
    let dir = scratch("dotted_stem");
    save_image(&dir.join("a.b.jpg"));

    let embed = iwm(&dir, &["embed", "a.b.jpg", "--text", "hello"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    assert!(dir.join("a.b.wm.png").is_file());
}

#[test]
fn embed_reads_bmp_webp_and_jpeg_files() {
    let dir = scratch("formats");
    for (name, stem) in [("a.bmp", "a"), ("b.webp", "b"), ("c.jpeg", "c")] {
        save_image(&dir.join(name));
        let embed = iwm(&dir, &["embed", name, "--text", "hello"]);
        assert!(embed.status.success(), "{name}: {}", stderr(&embed));
        let extract = iwm(&dir, &["extract", &format!("{stem}.wm.png")]);
        assert_eq!(stdout(&extract), "hello\n", "{name}");
    }
}

#[test]
fn embed_detects_format_from_content() {
    let dir = scratch("content_format");
    pattern(64, 64)
        .save_with_format(dir.join("png-named.jpg"), ImageFormat::Png)
        .unwrap();

    let embed = iwm(&dir, &["embed", "png-named.jpg", "--text", "hello"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    let extract = iwm(&dir, &["extract", "png-named.wm.png"]);
    assert_eq!(stdout(&extract), "hello\n");
}

#[test]
fn embed_applies_exif_orientation() {
    let dir = scratch("orientation");
    let mut jpeg = Vec::new();
    DynamicImage::ImageRgb8(pattern(64, 32))
        .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
        .unwrap();
    let upright = image::load_from_memory(&jpeg)
        .unwrap()
        .rotate90()
        .into_rgb8();
    jpeg.splice(2..2, EXIF_ORIENTATION_6);
    fs::write(dir.join("portrait.jpg"), jpeg).unwrap();

    let embed = iwm(&dir, &["embed", "portrait.jpg", "--text", "hello"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    let marked = image::open(dir.join("portrait.wm.png"))
        .unwrap()
        .into_rgb8();
    assert_eq!(marked.dimensions(), (32, 64));
    let high_bits = |image: &RgbImage| image.iter().map(|sample| sample & !1).collect::<Vec<_>>();
    assert_eq!(high_bits(&marked), high_bits(&upright));
}

#[test]
fn embed_failure_leaves_no_output_file() {
    let dir = scratch("no_partial_output");
    pattern(4, 4).save(dir.join("tiny.png")).unwrap();

    let embed = iwm(&dir, &["embed", "tiny.png", "--text", "hello", "--force"]);
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("holds at most 0"));
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn embed_with_force_removes_the_temporary_file_when_replacing_fails() {
    let dir = scratch("temporary_file");
    save_image(&dir.join("photo.png"));
    fs::create_dir_all(dir.join("photo.wm.png/inner")).unwrap();

    let embed = iwm(&dir, &["embed", "photo.png", "--text", "hello", "--force"]);
    assert!(!embed.status.success());
    assert!(
        stderr(&embed).starts_with("error: cannot write photo.wm.png:"),
        "{}",
        stderr(&embed)
    );
    let mut names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["photo.png", "photo.wm.png"]);
}

#[test]
fn embed_accepts_text_starting_with_a_hyphen() {
    let dir = scratch("hyphen_text");
    save_image(&dir.join("photo.png"));

    let embed = iwm(&dir, &["embed", "photo.png", "--text=-x"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    assert_eq!(stdout(&iwm(&dir, &["extract", "photo.wm.png"])), "-x\n");
}

#[test]
fn embed_text_without_a_value_does_not_take_the_next_flag() {
    let dir = scratch("text_missing_value");
    save_image(&dir.join("photo.png"));

    let embed = iwm(&dir, &["embed", "photo.png", "--text", "--help"]);
    assert_eq!(embed.status.code(), Some(2));
    assert!(stderr(&embed).contains("a value is required for '--text <TEXT>'"));
    assert!(!dir.join("photo.wm.png").exists());
}

#[test]
fn extract_names_the_file_when_no_watermark_is_found() {
    let dir = scratch("no_watermark");
    save_image(&dir.join("plain.png"));

    let extract = iwm(&dir, &["extract", "plain.png"]);
    assert!(!extract.status.success());
    assert_eq!(stderr(&extract), "error: no watermark found in plain.png\n");
}

#[test]
fn extract_reports_invalid_utf8() {
    let dir = scratch("invalid_utf8");
    invisible_watermark::embed(DynamicImage::ImageRgb8(pattern(64, 64)), &[0xFF, 0xFE])
        .unwrap()
        .save(dir.join("binary.png"))
        .unwrap();

    let extract = iwm(&dir, &["extract", "binary.png"]);
    assert!(!extract.status.success());
    assert_eq!(
        stderr(&extract),
        "error: watermark in binary.png is not valid UTF-8 text\n"
    );
}

#[test]
fn extract_exits_cleanly_when_stdout_is_closed() {
    let dir = scratch("closed_stdout");
    save_image(&dir.join("photo.png"));
    assert!(
        iwm(&dir, &["embed", "photo.png", "--text", "hello"])
            .status
            .success()
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_iwm"))
        .args(["extract", "photo.wm.png"])
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
}

#[cfg(unix)]
#[test]
fn embed_directory_reports_symlinked_images() {
    let dir = scratch("directory_symlink");
    save_image(&dir.join("photo.png"));
    save_image(&dir.join("in/real.png"));
    std::os::unix::fs::symlink(dir.join("photo.png"), dir.join("in/link.png")).unwrap();

    let embed = iwm(&dir, &["embed", "in", "--output", "out", "--text", "hello"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    assert!(stderr(&embed).contains("skipped in/link.png"));
    assert!(stdout(&embed).contains("embedded 1 of 1 images"));
    assert!(!dir.join("out/link.png").exists());
}

#[cfg(unix)]
#[test]
fn embed_directory_refuses_symlink_from_output_into_input() {
    let dir = scratch("directory_output_symlink");
    save_image(&dir.join("in/sub/x.png"));
    fs::create_dir(dir.join("out")).unwrap();
    std::os::unix::fs::symlink(dir.join("in"), dir.join("out/sub")).unwrap();

    let embed = iwm(
        &dir,
        &[
            "embed", "in", "--output", "out", "--text", "hello", "--force",
        ],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("inside the input directory"));
    assert!(!dir.join("in/x.png").exists());
}

#[cfg(windows)]
#[test]
fn embed_directory_refuses_junction_from_output_into_input() {
    let dir = scratch("directory_output_junction");
    save_image(&dir.join("in/sub/x.png"));
    fs::create_dir(dir.join("out")).unwrap();
    let mklink = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dir.join("out").join("sub"))
        .arg(dir.join("in"))
        .output()
        .unwrap();
    assert!(
        mklink.status.success(),
        "{}",
        String::from_utf8_lossy(&mklink.stderr)
    );

    let embed = iwm(
        &dir,
        &[
            "embed", "in", "--output", "out", "--text", "hello", "--force",
        ],
    );
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("inside the input directory"));
    assert!(!dir.join("in/x.png").exists());
}

#[cfg(unix)]
#[test]
fn embed_with_force_replaces_a_symlinked_output_not_its_target() {
    let dir = scratch("force_symlink_output");
    save_image(&dir.join("photo.png"));
    fs::write(dir.join("target.txt"), "keep").unwrap();
    std::os::unix::fs::symlink(dir.join("target.txt"), dir.join("photo.wm.png")).unwrap();

    let embed = iwm(&dir, &["embed", "photo.png", "--text", "hello", "--force"]);
    assert!(embed.status.success(), "{}", stderr(&embed));
    assert!(
        fs::symlink_metadata(dir.join("photo.wm.png"))
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(fs::read_to_string(dir.join("target.txt")).unwrap(), "keep");
    assert_eq!(stdout(&iwm(&dir, &["extract", "photo.wm.png"])), "hello\n");
}

#[cfg(unix)]
#[test]
fn embed_directory_skips_unreadable_subdirectories() {
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("directory_unreadable");
    save_image(&dir.join("in/good.png"));
    save_image(&dir.join("in/locked/hidden.png"));
    let locked = dir.join("in/locked");
    fs::set_permissions(&locked, Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&locked).is_ok() {
        // Root can read the directory anyway, so there is nothing to check.
        fs::set_permissions(&locked, Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let embed = iwm(&dir, &["embed", "in", "--output", "out", "--text", "hello"]);
    fs::set_permissions(&locked, Permissions::from_mode(0o755)).unwrap();
    assert!(!embed.status.success());
    assert_eq!(
        stderr(&embed),
        "error: cannot read directory in/locked: Permission denied (os error 13)\n"
    );
    assert!(stdout(&embed).contains("embedded 1 of 1 images"));
    assert!(dir.join("out/good.png").is_file());
}

#[test]
fn embed_and_extract_round_trip_with_password() {
    let dir = scratch("password_round_trip");
    save_image(&dir.join("photo.png"));

    let args = ["embed", "photo.png", "--text", MIXED_SCRIPTS, "--password"];
    let embed = iwm_with_password(&dir, &args, PASSWORD);
    assert!(embed.status.success(), "{}", stderr(&embed));

    let extract = iwm_with_password(&dir, &["extract", "photo.wm.png", "--password"], PASSWORD);
    assert!(extract.status.success(), "{}", stderr(&extract));
    assert_eq!(stdout(&extract), format!("{MIXED_SCRIPTS}\n"));
}

#[test]
fn extract_with_wrong_password_fails() {
    let dir = scratch("password_wrong");
    save_image(&dir.join("photo.png"));
    let args = ["embed", "photo.png", "--text", "hello", "--password"];
    assert!(iwm_with_password(&dir, &args, PASSWORD).status.success());

    let extract = iwm_with_password(
        &dir,
        &["extract", "photo.wm.png", "--password"],
        "wrong password",
    );
    assert_eq!(extract.status.code(), Some(1));
    assert_eq!(
        stderr(&extract),
        "error: wrong password or damaged watermark in photo.wm.png\n"
    );
}

#[test]
fn extract_of_encrypted_image_without_password_names_the_flag() {
    let dir = scratch("password_missing_flag");
    save_image(&dir.join("photo.png"));
    let args = ["embed", "photo.png", "--text", "hello", "--password"];
    assert!(iwm_with_password(&dir, &args, PASSWORD).status.success());

    let extract = iwm(&dir, &["extract", "photo.wm.png"]);
    assert_eq!(extract.status.code(), Some(1));
    assert_eq!(
        stderr(&extract),
        "error: watermark in photo.wm.png is encrypted, run with --password\n"
    );
}

#[test]
fn password_without_terminal_or_variable_names_iwm_password() {
    let dir = scratch("password_no_terminal");
    save_image(&dir.join("photo.png"));

    let embed = iwm(
        &dir,
        &["embed", "photo.png", "--text", "hello", "--password"],
    );
    assert_eq!(embed.status.code(), Some(1));
    assert_eq!(
        stderr(&embed),
        "error: no terminal to ask for the password, set IWM_PASSWORD instead\n"
    );
    assert!(!dir.join("photo.wm.png").exists());
}

#[test]
fn missing_input_is_reported_before_the_password_is_read() {
    let dir = scratch("password_missing_input");
    let cases: [(&[&str], &str); 3] = [
        (
            &["embed", "missing.png", "--text", "hello", "--password"],
            "error: missing.png does not exist\n",
        ),
        (
            &["extract", "missing.png", "--password"],
            "error: cannot read missing.png: ",
        ),
        (
            &["verify", "missing.png", "--password"],
            "error: missing.png does not exist\n",
        ),
    ];
    for (args, expected) in cases {
        let output = iwm(&dir, args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(stderr(&output).starts_with(expected), "{}", stderr(&output));
    }
}

/// Writes `marked.png` (format v1), `secret.png` (encrypted) and `plain.png` (unmarked) under `dir`.
fn save_verify_samples(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let image = || DynamicImage::ImageRgb8(pattern(64, 64));
    invisible_watermark::embed(image(), b"hello")
        .unwrap()
        .save(dir.join("marked.png"))
        .unwrap();
    invisible_watermark::embed_with_password(image(), b"hello", PASSWORD)
        .unwrap()
        .save(dir.join("secret.png"))
        .unwrap();
    image().save(dir.join("plain.png")).unwrap();
}

#[test]
fn verify_reports_each_state_of_a_file() {
    let dir = scratch("verify_file");
    save_verify_samples(&dir);

    let cases: [(&[&str], Option<&str>, &str, bool); 6] = [
        (&["verify", "marked.png"], None, "marked.png: ok\n", true),
        (
            &["verify", "secret.png"],
            None,
            "secret.png: encrypted, run with --password to check it\n",
            false,
        ),
        (
            &["verify", "secret.png", "--password"],
            Some(PASSWORD),
            "secret.png: ok, encrypted\n",
            true,
        ),
        (
            &["verify", "secret.png", "--password"],
            Some("wrong password"),
            "secret.png: wrong password or damaged watermark\n",
            false,
        ),
        (
            &["verify", "marked.png", "--password"],
            Some(PASSWORD),
            "marked.png: watermark is not encrypted\n",
            false,
        ),
        (
            &["verify", "plain.png"],
            None,
            "plain.png: no watermark found\n",
            false,
        ),
    ];
    for (args, password, expected, success) in cases {
        let output = match password {
            Some(password) => iwm_with_password(&dir, args, password),
            None => iwm(&dir, args),
        };
        assert_eq!(stdout(&output), expected, "{args:?}");
        assert_eq!(output.status.success(), success, "{args:?}");
        assert!(!stdout(&output).contains("hello"), "{args:?}");
    }
}

#[test]
fn verify_directory_counts_verified_images() {
    let dir = scratch("verify_directory");
    save_verify_samples(&dir.join("in"));
    let line =
        |name: &str, status: &str| format!("{}: {status}\n", Path::new("in").join(name).display());

    let output = iwm(&dir, &["verify", "in"]);
    assert_eq!(
        stdout(&output),
        line("marked.png", "ok")
            + &line("plain.png", "no watermark found")
            + &line("secret.png", "encrypted, run with --password to check it")
            + "verified 1 of 3 images\n"
    );
    assert_eq!(output.status.code(), Some(1));

    let output = iwm_with_password(&dir, &["verify", "in", "--password"], PASSWORD);
    assert_eq!(
        stdout(&output),
        line("marked.png", "watermark is not encrypted")
            + &line("plain.png", "no watermark found")
            + &line("secret.png", "ok, encrypted")
            + "verified 1 of 3 images\n"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn embed_and_extract_round_trip_resilient() {
    let dir = scratch("resilient_round_trip");
    save_photo(&dir.join("photo.png"), 768, 512);

    let args = [
        "embed",
        "photo.png",
        "--text",
        "Example Studio",
        "--resilient",
    ];
    let embed = iwm(&dir, &args);
    assert!(embed.status.success(), "{}", stderr(&embed));
    assert_eq!(
        stdout(&iwm(&dir, &["extract", "photo.wm.png"])),
        "Example Studio\n"
    );

    let secret = [&args[..], &["--password", "--output", "secret.png"]].concat();
    let embed = iwm_with_password(&dir, &secret, PASSWORD);
    assert!(embed.status.success(), "{}", stderr(&embed));
    let extract = iwm_with_password(&dir, &["extract", "secret.png", "--password"], PASSWORD);
    assert_eq!(stdout(&extract), "Example Studio\n");
}

#[test]
fn embed_resilient_writes_jpeg_output() {
    let dir = scratch("resilient_jpeg_output");
    save_photo(&dir.join("photo.png"), 768, 512);

    let args = [
        "embed",
        "photo.png",
        "--text",
        "Example Studio",
        "--resilient",
    ];
    let embed = iwm(&dir, &[&args[..], &["--output", "out.jpg"]].concat());
    assert!(embed.status.success(), "{}", stderr(&embed));
    let bytes = fs::read(dir.join("out.jpg")).unwrap();
    assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Jpeg);
    assert_eq!(
        stdout(&iwm(&dir, &["extract", "out.jpg"])),
        "Example Studio\n"
    );
}

#[test]
fn resilient_mark_survives_resize_and_jpeg_through_files() {
    let dir = scratch("resilient_edit");
    save_photo(&dir.join("photo.png"), 768, 512);
    let args = [
        "embed",
        "photo.png",
        "--text",
        "Example Studio",
        "--resilient",
    ];
    assert!(iwm(&dir, &args).status.success());

    let marked = image::open(dir.join("photo.wm.png")).unwrap();
    let half = marked.resize_exact(384, 256, image::imageops::FilterType::Lanczos3);
    let mut jpeg = fs::File::create(dir.join("half.jpg")).unwrap();
    half.write_with_encoder(JpegEncoder::new_with_quality(&mut jpeg, 75))
        .unwrap();
    drop(jpeg);

    assert_eq!(
        stdout(&iwm(&dir, &["extract", "half.jpg"])),
        "Example Studio\n"
    );
}

#[test]
fn embed_resilient_rejects_small_image() {
    let dir = scratch("resilient_small");
    save_photo(&dir.join("small.png"), 640, 480);

    let embed = iwm(&dir, &["embed", "small.png", "--text", "hi", "--resilient"]);
    assert_eq!(embed.status.code(), Some(1));
    assert_eq!(
        stderr(&embed),
        "error: cannot embed into small.png: image is 480 pixels on the short side but the resilient mode needs at least 512\n"
    );
    assert!(!dir.join("small.wm.png").exists());
}

#[test]
fn embed_resilient_rejects_text_over_32_bytes() {
    let dir = scratch("resilient_long_text");
    save_photo(&dir.join("photo.png"), 768, 512);

    let text = "0123456789 ABCDEF abcdef !#$%&*+!";
    let embed = iwm(&dir, &["embed", "photo.png", "--text", text, "--resilient"]);
    assert_eq!(embed.status.code(), Some(1));
    assert_eq!(
        stderr(&embed),
        "error: text is 33 bytes but the resilient mode holds at most 32\n"
    );
}

#[test]
fn embed_resilient_output_must_be_png_or_jpeg() {
    let dir = scratch("resilient_output_extension");
    save_photo(&dir.join("photo.png"), 768, 512);

    let args = [
        "embed",
        "photo.png",
        "--text",
        "hi",
        "--resilient",
        "--output",
        "out.webp",
    ];
    let embed = iwm(&dir, &args);
    assert!(!embed.status.success());
    assert!(stderr(&embed).contains("--output must end in .png, .jpg or .jpeg"));
    assert!(!dir.join("out.webp").exists());
}

#[test]
fn embed_directory_with_resilient() {
    let dir = scratch("resilient_directory");
    save_photo(&dir.join("in/good.png"), 768, 512);
    save_photo(&dir.join("in/small.png"), 640, 480);

    let args = [
        "embed",
        "in",
        "--output",
        "out",
        "--text",
        "hi",
        "--resilient",
    ];
    let embed = iwm(&dir, &args);
    assert_eq!(embed.status.code(), Some(1));
    assert!(stdout(&embed).ends_with("embedded 1 of 2 images\n"));
    let errors = stderr(&embed);
    assert!(
        errors.contains("small.png") && errors.contains("512"),
        "{errors}"
    );
    assert_eq!(stdout(&iwm(&dir, &["extract", "out/good.png"])), "hi\n");
}

#[test]
fn verify_reports_resilient_marks() {
    let dir = scratch("resilient_verify");
    save_photo(&dir.join("photo.png"), 768, 512);
    let args = ["embed", "photo.png", "--text", "hi", "--resilient"];
    assert!(iwm(&dir, &args).status.success());
    let secret = [&args[..], &["--password", "--output", "secret.png"]].concat();
    assert!(iwm_with_password(&dir, &secret, PASSWORD).status.success());

    let plain = iwm(&dir, &["verify", "photo.wm.png"]);
    assert_eq!(stdout(&plain), "photo.wm.png: ok, resilient\n");
    assert!(plain.status.success());

    let locked = iwm(&dir, &["verify", "secret.png"]);
    assert_eq!(
        stdout(&locked),
        "secret.png: encrypted, run with --password to check it\n"
    );
    assert_eq!(locked.status.code(), Some(1));

    let opened = iwm_with_password(&dir, &["verify", "secret.png", "--password"], PASSWORD);
    assert_eq!(stdout(&opened), "secret.png: ok, resilient, encrypted\n");
    assert!(opened.status.success());
}

#[test]
fn extract_applies_exif_orientation_to_resilient_marks() {
    let dir = scratch("resilient_orientation");
    save_photo(&dir.join("photo.png"), 768, 512);
    let args = [
        "embed",
        "photo.png",
        "--text",
        "Example Studio",
        "--resilient",
    ];
    assert!(iwm(&dir, &args).status.success());

    let rotated = image::open(dir.join("photo.wm.png")).unwrap().rotate270();
    let mut jpeg = Vec::new();
    rotated
        .write_with_encoder(JpegEncoder::new_with_quality(&mut jpeg, 95))
        .unwrap();
    jpeg.splice(2..2, EXIF_ORIENTATION_6);
    fs::write(dir.join("portrait.jpg"), jpeg).unwrap();

    assert_eq!(
        stdout(&iwm(&dir, &["extract", "portrait.jpg"])),
        "Example Studio\n"
    );
}

/// Writes the LSB magic and `version` into the low bits of the first 40 samples.
fn with_lsb_header(mut image: RgbImage, version: u8) -> RgbImage {
    let header = [b'I', b'W', b'M', b'K', version];
    let bits = header
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |shift| (byte >> shift) & 1));
    for (sample, bit) in image.iter_mut().zip(bits) {
        *sample = (*sample & !1) | bit;
    }
    image
}

#[test]
fn extract_reads_resilient_mark_when_an_lsb_header_survives() {
    let dir = scratch("resilient_over_lsb");
    let photo = || DynamicImage::ImageRgb8(photo(768, 512));
    let cases = [
        (
            invisible_watermark::embed(photo(), b"hello").unwrap(),
            1,
            Error::Damaged,
        ),
        (
            invisible_watermark::embed_with_password(photo(), b"hello", PASSWORD).unwrap(),
            2,
            Error::PasswordRequired,
        ),
    ];
    for (lsb, version, lsb_error) in cases {
        let marked = invisible_watermark::embed_resilient(lsb, "Example Studio").unwrap();
        let marked = DynamicImage::ImageRgb8(with_lsb_header(marked.into_rgb8(), version));
        assert_eq!(invisible_watermark::extract(&marked), Err(lsb_error));
        let name = format!("v{version}.png");
        marked.save(dir.join(&name)).unwrap();

        assert_eq!(stdout(&iwm(&dir, &["extract", &name])), "Example Studio\n");
        assert_eq!(
            stdout(&iwm(&dir, &["verify", &name])),
            format!("{name}: ok, resilient\n")
        );
    }
}
