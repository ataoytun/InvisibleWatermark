<p align="center">
  <img src="https://raw.githubusercontent.com/ataoytun/InvisibleWatermark/HEAD/assets/logo.png" width="160" alt="InvisibleWatermark logo">
</p>

<p align="center">
  <a href="https://github.com/ataoytun/InvisibleWatermark/actions/workflows/ci.yml"><img src="https://github.com/ataoytun/InvisibleWatermark/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/invisible-watermark"><img src="https://img.shields.io/crates/v/invisible-watermark.svg" alt="crates.io"></a>
</p>

# InvisibleWatermark

`iwm` hides a short text watermark in the least significant bits of an image and reads it back. The change is invisible to the eye. The text survives only while the pixels stay unchanged: saving the image again in a lossless format keeps it, while JPEG compression, resizing, cropping and mirroring remove it.

```console
$ iwm embed photo.jpg --text "© 2026 Example Studio"
wrote photo.wm.png
$ iwm extract photo.wm.png
© 2026 Example Studio
```

[Try it in your browser](https://ataoytun.github.io/InvisibleWatermark/). The page runs the library compiled to WebAssembly, and your images are never uploaded.

## Install

Prebuilt binaries for x86_64 Linux (static), x86_64 Windows and macOS (Intel and Apple silicon) are on the [releases page](https://github.com/ataoytun/InvisibleWatermark/releases). To build from source, use Rust 1.88 or later:

```
cargo install --locked iwm
```

Or install the latest code from GitHub:

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

To read the text from a UTF-8 file, use `--text-file notice.txt`. The file is embedded byte for byte, so a trailing newline becomes part of the text and `extract` prints it back.

An image holds 3 bits per pixel minus 13 bytes of overhead, so a 640×480 image holds 115,187 bytes of text. With a password the overhead is 65 bytes and the same image holds 115,135. Longer text is rejected with the amount that fits. Here `tiny.png` is a 4×4 image:

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
$ iwm extract secret.png
error: watermark in secret.png is encrypted, run with --password
```

The text goes to stdout. Problems go to stderr with the file name and a non-zero exit status. A damaged watermark, an unknown format version and text that is not valid UTF-8 are each reported as such. An encrypted watermark needs `--password`, described next.

### Protecting the text with a password

`--password` encrypts the text on `embed` and decrypts it on `extract`. The password is asked for on the terminal without echo, twice when embedding:

```console
$ iwm embed photo.jpg --text "© 2026 Example Studio" --password
Password:
Repeat password:
wrote photo.wm.png
$ iwm extract photo.wm.png --password
Password:
© 2026 Example Studio
```

With the wrong password, or with an image that was changed after embedding, the answer is the same:

```console
$ iwm extract photo.wm.png --password
Password:
error: wrong password or damaged watermark in photo.wm.png
```

The environment variable `IWM_PASSWORD` replaces the prompt, which is what scripts need:

```
export IWM_PASSWORD="$(cat password.txt)"
iwm embed photos --output watermarked --text-file notice.txt --password
iwm verify watermarked --password
```

Without a terminal and without the variable, the command stops instead of waiting:

```console
$ iwm embed photo.jpg --text "© 2026 Example Studio" --password < /dev/null
error: no terminal to ask for the password, set IWM_PASSWORD instead
```

### Checking images with verify

`iwm verify` reads the watermark of an image, or of every image under a directory, and reports whether it can be read. It never prints the text. Here `gallery` holds a plain watermark, an encrypted one and an image without a watermark:

```console
$ iwm verify gallery
gallery/beach.png: ok
gallery/portrait.png: encrypted, run with --password to check it
gallery/sketch.jpg: no watermark found
verified 1 of 3 images
$ iwm verify gallery --password
gallery/beach.png: watermark is not encrypted
gallery/portrait.png: ok, encrypted
gallery/sketch.jpg: no watermark found
verified 1 of 3 images
```

The exit status is 0 only when every image verified. A single file prints one line. The text is never printed.

## How it works

Two record formats exist. Without a password `iwm` writes v1, with a password it writes v2, and both are read. A reader looks at the first 5 bytes: other magic bytes mean there is no watermark, version 1 is read as v1, and version 2 needs a password. Any other version is reported as an unsupported format version.

### Format v1

The text is stored as UTF-8 in a small record. Its bits go into the least significant bit of the red, green and blue channels, pixel by pixel in row-major order, most significant bit first. Alpha is never touched, and neither are the bits after the record. A CRC-32 checksum detects damage to the stored length and the text. Damaged magic bytes make the watermark look missing, and a damaged version byte reads as an unknown format version.

| Offset | Size | Field                                                |
|--------|------|------------------------------------------------------|
| 0      | 4    | magic, ASCII `IWMK`                                  |
| 4      | 1    | format version, `1`                                  |
| 5      | 4    | payload length in bytes, u32 big-endian              |
| 9      | n    | payload                                              |
| 9 + n  | 4    | CRC-32 (IEEE) of all bytes before it, u32 big-endian |

Capacity in payload bytes is `floor(width * height * 3 / 8) - 13`, or zero for images too small to hold the 13 bytes of overhead. It never exceeds 4,294,967,295, the largest length the header can store.

### Format v2

Let `n` be `width * height * 3`, the number of red, green and blue samples, numbered as in v1: pixels in row-major order, red, green, blue, alpha skipped. Each sample carries one bit in its lowest bit, and bytes are written most significant bit first.

The first 21 bytes sit in order in samples 0 to 167 and are not encrypted:

| Offset | Size | Field                    |
|--------|------|--------------------------|
| 0      | 4    | magic, ASCII `IWMK`      |
| 4      | 1    | format version, `2`      |
| 5      | 16   | salt, random             |

A v2 image too small to hold these 21 bytes is reported as damaged.

The password, as UTF-8 bytes without normalization, and the salt go through Argon2id (version 0x13, 19,456 KiB of memory, 2 iterations, parallelism 1) to produce 64 bytes. Bytes 0 to 31 are the cipher key and bytes 32 to 63 are the position key. An empty password is rejected.

The rest of the record goes into samples 168 to `n - 1` in a shuffled order. Let `m = n - 168` and start with `a[i] = 168 + i` for `i` in `0..m`. For `i = 0, 1, ...`, draw `r` uniformly with `0 <= r < m - i`, swap `a[i]` and `a[i + r]`, and store bit `i` of the body in sample `a[i]`. The random numbers come from the ChaCha20 keystream (RFC 8439, 96-bit zero nonce, block counter 0) keyed with the position key, taken 8 bytes at a time as a little-endian u64. To draw below `range = m - i`, discard values greater than or equal to `floor(2^64 / range) * range` and take the rest modulo `range`.

The body, in this order on the shuffled positions:

| Size   | Field                                          |
|--------|------------------------------------------------|
| 4      | plaintext length `L` in bytes, u32 big-endian  |
| 24     | nonce, random                                  |
| L + 16 | XChaCha20-Poly1305 ciphertext and tag          |

XChaCha20-Poly1305 encrypts the text with the cipher key and the 24-byte nonce from the body. The associated data is the 21 prefix bytes followed by the 4 length bytes. A length fits when `L + 65` is at most `floor(n / 8)`. Any failure after the prefix, such as a length that does not fit or a failed tag, is reported as a wrong password or a damaged watermark, because the two cannot be told apart. The overhead is 21 + 4 + 24 + 16 = 65 bytes, so the capacity is `floor(n / 8) - 65` payload bytes, zero for smaller images, and never more than 4,294,967,295.

### Library

The same code is available as the [`invisible-watermark`](https://crates.io/crates/invisible-watermark) library crate. It works on decoded images and byte slices and does no file I/O. It enables no codecs of the `image` crate, so you enable the ones you need, such as `png` or `jpeg`.

To use it on `wasm32-unknown-unknown` in a browser, also depend on `getrandom` with the `wasm_js` feature, as `crates/iwm-web` does, so the salt and nonce come from `crypto.getRandomValues`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.4", features = ["wasm_js"] }
```

## Limitations

- Only lossless formats keep the watermark. JPEG compression, resizing, cropping and recompression by messengers or social networks remove it.
- Anyone can strip a watermark by overwriting the low bits. Without a password they can also read the text, which is stored as plain bytes.
- The v2 prefix is not encrypted, so anyone can tell that an image carries a v2 watermark and read its salt.
- A password protects the text only as well as the password resists guessing.
- Statistical steganalysis can still detect that low bits were changed.
- A password shows who knew the password, not who owns the image: anyone can embed any text in any image.
- A password makes large texts slow and memory-hungry. On one x86_64 Windows desktop, embedding 4 MB of text into a 12-megapixel image took about 15 seconds and 560 MiB of memory with a password, against 0.4 seconds and 130 MiB without, and `verify --password` on the result took about the same as the embed.
- Embedding returns an 8-bit image. 16-bit images lose precision, and grayscale images become RGB.
- ICC color profiles and other metadata, such as EXIF, are not copied to the output.
- Symlinks inside an input directory are not followed: linked images are skipped with a message, linked directories silently.

## Browser demo

`web/` is the demo page: plain HTML, CSS and JavaScript around the library compiled by `crates/iwm-web`. To build it, add the `wasm32-unknown-unknown` target and install `wasm-bindgen-cli` at the version of `wasm-bindgen` in `Cargo.lock`, then run:

```
cargo build -p iwm-web --target wasm32-unknown-unknown --profile web --locked
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web/iwm_web.wasm
```

Serve `web/` with any static server, for example `python -m http.server -d web`, and open `http://localhost:8000`. Opening `index.html` from disk does not work, because browsers do not load JavaScript modules from `file://` URLs.

## Roadmap

- [x] v0.1 Rust rewrite: versioned format with checksum, UTF-8 text, lossless output, CLI, prebuilt binaries
- [x] v0.2 Password protection (encryption and scattered bits), `verify`, `--text-file`, crates.io release
- [x] v0.3 In-browser demo (WebAssembly)
- [ ] v0.4 A mode that survives JPEG compression and resizing, with error correction
- [ ] v0.5 Python package
- [ ] v1.0 Stable format and API

The earlier C# WinForms version is kept in git history under the [`legacy-csharp`](https://github.com/ataoytun/InvisibleWatermark/tree/legacy-csharp) tag.

## License

MIT, see [LICENSE](https://github.com/ataoytun/InvisibleWatermark/blob/main/LICENSE).
