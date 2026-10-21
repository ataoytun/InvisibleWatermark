use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

const MIXED_SCRIPTS: &str = "naïve café, Ελλάδα, 東京, Москва";
const EMOJI: &str = "🦀 ok 👍🏽";

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

/// Runs `iwm` with `dir` as the working directory.
fn iwm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_iwm"))
        .args(args)
        .current_dir(dir)
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

#[test]
fn embed_and_extract_round_trip_through_files() {
    let dir = scratch("round_trip");
    save_image(&dir.join("photo.jpg"));

    for text in [MIXED_SCRIPTS, EMOJI] {
        let embed = iwm(&dir, &["embed", "photo.jpg", "--text", text, "--force"]);
        assert!(embed.status.success(), "{}", stderr(&embed));
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
    assert!(stdout(&embed).contains("embedded 3 of 3 images"));

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
