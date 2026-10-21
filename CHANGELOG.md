# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/ataoytun/InvisibleWatermark/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ataoytun/InvisibleWatermark/releases/tag/v0.1.0
