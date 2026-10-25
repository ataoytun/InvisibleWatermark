# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Library: format v3, a resilient mode that survives JPEG compression and
  resizing, with `embed_resilient`, `embed_resilient_with_password`,
  `extract_resilient`, `extract_resilient_with_password`, the constants
  `RESILIENT_MIN_SIDE` and `RESILIENT_MAX_TEXT`, and the error variants
  `TextTooLong` and `ImageTooSmall`.
- CLI: `embed --resilient`, which writes PNG or, for an output path ending
  in `.jpg` or `.jpeg`, JPEG. `extract` and `verify` read resilient marks,
  and `verify` reports them as `ok, resilient`.
- Demo: a checkbox that embeds in the resilient mode and downloads a
  `.wm.jpg` file.
- `iwm-bench`, a benchmark that measures how often a resilient watermark
  survives JPEG, resizing, cropping and rotation. It is not published.

### Changed

- `iwm extract`, `iwm verify` and the demo look for a resilient mark when no
  LSB watermark can be read.
- `Error` is `#[non_exhaustive]`, so a `match` on it needs a wildcard arm and
  later variants are no longer breaking changes.
- The unsupported version error prints the byte in hex, such as `0x03`.

## [0.3.0] - 2026-10-23

`invisible-watermark` and `iwm` move to 0.3.0 with no API or behavior changes.

### Added

- A browser demo at https://ataoytun.github.io/InvisibleWatermark/ that
  embeds and reads watermarks with the library compiled to WebAssembly.
  Images are processed in the browser and never uploaded.

## [0.2.0] - 2026-10-22

### Added

- Format v2 for password-protected watermarks: the text is encrypted with
  XChaCha20-Poly1305 under a key derived from the password with Argon2id,
  and its bits are scattered across the image. Images from 0.1.0 (format v1)
  still read, and embedding without a password still writes v1.
- Library: `embed_with_password`, `extract_with_password` and
  `capacity_with_password`, and error variants for password problems.
- `--text-file` reads the text from a UTF-8 file, byte for byte.
- `--password` on `embed` and `extract`. The password comes from
  `IWM_PASSWORD` or a hidden prompt.
- `iwm verify` checks an image or a directory of images and reports whether
  each carries a readable watermark, without printing the text.
- Both crates are published on crates.io: the library as
  `invisible-watermark`, and the CLI as `iwm`, which installs with
  `cargo install --locked iwm`.

### Changed

- The library no longer enables any `image` codecs. Library users enable the
  `image` features they need, such as `png` or `jpeg`.

## [0.1.0] - 2026-10-16

The C# WinForms application is replaced by a Rust library
(`invisible-watermark`) and a command-line tool (`iwm`).

### Added

- `iwm embed` for a single image or a directory tree, and `iwm extract`.
- Format v1: magic bytes, format version, payload length and a CRC-32 stored
  in the least significant bits of the red, green and blue channels.
- WebP input.
- The EXIF orientation of the input is applied before embedding.
- Prebuilt binaries for x86_64 Linux (static), x86_64 Windows and macOS
  (Intel and Apple silicon).

### Changed

- Bits are stored in the red, green and blue channels instead of red only, so
  an image holds three times as much text. Images watermarked by the C#
  version cannot be read by 0.1.0, and the reverse is also true.

### Fixed

- A watermarked JPEG came back as JPEG data under a `.png` name, and the
  compression destroyed the watermark. Output is now always PNG.
- Text outside Latin-1 was corrupted: "€" came back as "¬" and "日" as "å".
  Text is now stored as UTF-8.
- Reading returned the text repeated across the image and cut to 50
  characters, and it failed on images without a watermark. The length is now
  stored with the text.
- A missing watermark and a damaged one looked the same. A checksum now
  detects damage to the stored length and text, and an unknown format
  version is reported as such.
- Directory mode wrote `a/x.jpg` and `b/x.jpg` to the same file, and a failed
  image left an empty output file. Output now mirrors the input tree, and a
  failed write leaves no file behind.

### Removed

- The WinForms interface. The last C# version is tagged `legacy-csharp`.
- TIFF and GIF input, which the C# version read through GDI+.

[Unreleased]: https://github.com/ataoytun/InvisibleWatermark/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/ataoytun/InvisibleWatermark/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ataoytun/InvisibleWatermark/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ataoytun/InvisibleWatermark/releases/tag/v0.1.0
