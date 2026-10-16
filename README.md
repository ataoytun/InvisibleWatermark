<p align="center">
  <img src="assets/logo.png" width="160" alt="InvisibleWatermark logo">
</p>

# InvisibleWatermark

`iwm` hides a short text watermark in the least significant bits of an image and reads it back. The change is invisible to the eye. The text survives only while the pixels stay unchanged: saving the image again in a lossless format keeps it, while JPEG compression, resizing, cropping and mirroring remove it.

```console
$ iwm embed photo.jpg --text "© 2026 Example Studio"
wrote photo.wm.png
$ iwm extract photo.wm.png
© 2026 Example Studio
```

## Install

Prebuilt binaries for x86_64 Linux (static), x86_64 Windows and macOS (Intel and Apple silicon) are on the [releases page](https://github.com/ataoytun/InvisibleWatermark/releases). To build from source, use Rust 1.88 or later:

```
cargo install --locked --git https://github.com/ataoytun/InvisibleWatermark iwm
```

## Usage

### A single image

```
iwm embed photo.jpg --text "© 2026 Example Studio"
```

The result is written next to the input as `photo.wm.png`. Use `--output` to choose another path. It must end in `.png`: JPEG compression alters pixel values, and a single changed bit in the watermark is enough to lose it.

PNG, JPEG, BMP and WebP files are accepted as input. The format is detected from the file content, and the EXIF orientation is applied, so a portrait photo stays upright. The output is always PNG.

An existing output file is not overwritten unless you pass `--force`. With `--force` the image is first written to a temporary file next to the target, which then replaces it. A failed run leaves no partial file behind.

Text that starts with a hyphen is passed as `--text=-x`.

An image holds 3 bits per pixel minus 13 bytes of overhead, so a 640×480 image holds 115,187 bytes of text. Longer text is rejected with the amount that fits. Here `tiny.png` is a 4×4 image:

```console
$ iwm embed tiny.png --text "© 2026 Example Studio"
error: cannot embed into tiny.png: payload is 22 bytes but the image holds at most 0
```

### A directory

```
iwm embed photos --output watermarked --text "© 2026 Example Studio"
```

`iwm` walks `photos` recursively and picks files ending in `.png`, `.jpg`, `.jpeg`, `.bmp` or `.webp` (in any letter case). The folder structure is kept, so `photos/2025/beach.jpg` becomes `watermarked/2025/beach.png`.

`--output` is required in this mode. No output file is written inside the input directory, not even through a symlink in the output tree. Two inputs that map to the same output, such as `x.jpg` and `x.png`, are not both written: the first input in sorted order is written and the second is reported as an error. On case-insensitive file systems (the default on Windows and macOS), `X.jpg` and `x.png` map to the same output too.

Symlinked images are skipped with a message on stderr. A file that fails is reported on stderr and the run continues, and directories that cannot be read are reported and skipped. The run ends with a line like `embedded 48 of 50 images` and exits with a non-zero status if any file or directory failed.

### Reading a watermark

```console
$ iwm extract photo.wm.png
© 2026 Example Studio
$ iwm extract photo.jpg
error: no watermark found in photo.jpg
```

The text goes to stdout. Problems go to stderr with the file name and a non-zero exit status. A damaged watermark, an unknown format version and text that is not valid UTF-8 are each reported as such.

## How it works

The text is stored as UTF-8 in a small record. Its bits go into the least significant bit of the red, green and blue channels, pixel by pixel in row-major order, most significant bit first. Alpha is never touched, and neither are the bits after the record. A CRC-32 checksum detects damage to the stored length and the text. Damaged magic bytes make the watermark look missing, and a damaged version byte reads as an unknown format version.

| Offset | Size | Field                                                |
|--------|------|------------------------------------------------------|
| 0      | 4    | magic, ASCII `IWMK`                                  |
| 4      | 1    | format version, `1`                                  |
| 5      | 4    | payload length in bytes, u32 big-endian              |
| 9      | n    | payload                                              |
| 9 + n  | 4    | CRC-32 (IEEE) of all bytes before it, u32 big-endian |

Capacity in payload bytes is `floor(width * height * 3 / 8) - 13`, or zero for images too small to hold the 13 bytes of overhead. It never exceeds 4,294,967,295, the largest length the header can store.

The same code is available as the `invisible-watermark` library crate. It works on decoded images and byte slices and does no file I/O.

## Limitations

- Only lossless formats keep the watermark. JPEG compression, resizing, cropping and recompression by messengers or social networks remove it.
- Anyone who knows the format above can read the text or strip it.
- It is not encryption, and it is not proof of ownership: anyone can embed any text in any image.
- Embedding returns an 8-bit image. 16-bit images lose precision, and grayscale images become RGB.
- ICC color profiles and other metadata, such as EXIF, are not copied to the output.
- Symlinks inside an input directory are not followed: linked images are skipped with a message, linked directories silently.

## Roadmap

- [x] v0.1 Rust rewrite: versioned format with checksum, UTF-8 text, lossless output, CLI, prebuilt binaries
- [ ] v0.2 Password-based embedding and encryption, error correction, `verify`
- [ ] v0.3 In-browser demo (WebAssembly)
- [ ] v0.4 A mode that survives JPEG compression and resizing
- [ ] v0.5 Python package
- [ ] v1.0 Stable format and API, published on crates.io

The earlier C# WinForms version is kept in git history under the [`legacy-csharp`](https://github.com/ataoytun/InvisibleWatermark/tree/legacy-csharp) tag.

## License

MIT, see [LICENSE](LICENSE).
