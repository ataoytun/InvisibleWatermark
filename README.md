<p align="center">
  <img src="https://raw.githubusercontent.com/ataoytun/InvisibleWatermark/HEAD/assets/logo.png" width="160" alt="InvisibleWatermark logo">
</p>

<p align="center">
  <a href="https://github.com/ataoytun/InvisibleWatermark/actions/workflows/ci.yml"><img src="https://github.com/ataoytun/InvisibleWatermark/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/invisible-watermark"><img src="https://img.shields.io/crates/v/invisible-watermark.svg" alt="crates.io"></a>
</p>

# InvisibleWatermark

`iwm` hides a short text watermark in an image and reads it back. By default the text goes into the least significant bits: the change is invisible to the eye, and the text survives only while the pixels stay unchanged. Saving the image again in a lossless format keeps it, while JPEG compression, resizing, cropping and mirroring remove it. With `--resilient`, up to 32 bytes of text survive moderate JPEG compression and resizing, stored as a faint texture that can show on flat areas of the image.

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

The result is written next to the input as `photo.wm.png`. Use `--output` to choose another path. It must end in `.png`: JPEG compression alters pixel values, and a single changed bit in the watermark is enough to lose it. With `--resilient` it may also end in `.jpg` or `.jpeg`.

PNG, JPEG, BMP and WebP files are accepted as input. The format is detected from the file content, and the EXIF orientation is applied, so a portrait photo stays upright. The output is PNG, except that `--resilient` with an `--output` path ending in `.jpg` or `.jpeg` writes a JPEG (quality 90, alpha dropped).

An existing output file is not overwritten unless you pass `--force`. With `--force` the image is first written to a temporary file next to the target, which then replaces it. A failed run leaves no partial file behind.

Text that starts with a hyphen is passed as `--text=-x`.

To read the text from a UTF-8 file, use `--text-file notice.txt`. The file is embedded byte for byte, so a trailing newline becomes part of the text and `extract` prints it back.

An image holds 3 bits per pixel minus 13 bytes of overhead, so a 640×480 image holds 115,187 bytes of text. With a password the overhead is 65 bytes and the same image holds 115,135. Longer text is rejected with the amount that fits. Here `tiny.png` is a 4×4 image:

```console
$ iwm embed tiny.png --text "© 2026 Example Studio"
error: cannot embed into tiny.png: payload is 22 bytes but the image holds at most 0
```

### Surviving JPEG and resizing

`--resilient` stores the text so that it survives moderate JPEG compression and resizing, which is what messengers and photo services do to an image. The text holds at most 32 bytes, the image needs at least 512 pixels on the short side, and the output can be a JPEG. Here Pillow stands in for any tool that shrinks an image and saves it again:

```console
$ iwm embed photo.png --text "© 2026 Example Studio" --resilient --output photo.wm.jpg
wrote photo.wm.jpg
$ python -c "from PIL import Image; im = Image.open('photo.wm.jpg'); im.resize((im.width // 2, im.height // 2)).save('half.jpg', quality=75)"
$ iwm extract half.jpg
© 2026 Example Studio
$ iwm verify half.jpg
half.jpg: ok, resilient
```

`--resilient` works with `--password`, `--text-file` and directories. In directory mode the output files are PNG. `extract` and `verify` detect the format by themselves: they look for an LSB watermark first and for a resilient mark when no LSB watermark can be read. If neither is found, the LSB error is reported. The resilient mark is read after the EXIF orientation is applied, as it was embedded.

Too small an image and too long a text are rejected:

```console
$ iwm embed small.png --text "hi" --resilient
error: cannot embed into small.png: image is 480 pixels on the short side but the resilient mode needs at least 512
$ iwm embed photo.png --text '0123456789 ABCDEF abcdef !#$%&*+!' --resilient
error: text is 33 bytes but the resilient mode holds at most 32
```

`iwm-bench` (see [Benchmark](#benchmark)) embedded four 32-byte texts into each of the 24 Kodak images, applied the edits below and tried to read the text back. JPEG compression and resizing use the pure-Rust encoder and the Lanczos3 filter of the `image` crate, and other tools map the quality setting differently. Cropping removes 5% on each side, and rotating turns the image by 1 degree on the same canvas.

| Attack          | Success       | Failed images |
|-----------------|---------------|---------------|
| none            | 96/96 (100%)  | none          |
| jpeg90          | 96/96 (100%)  | none          |
| jpeg75          | 96/96 (100%)  | none          |
| jpeg50          | 96/96 (100%)  | none          |
| jpeg30          | 93/96 (96.9%) | kodim20       |
| resize50        | 96/96 (100%)  | none          |
| resize75        | 96/96 (100%)  | none          |
| resize150       | 96/96 (100%)  | none          |
| resize50-jpeg75 | 96/96 (100%)  | none          |
| crop5           | 0/96 (0.0%)   | all           |
| rotate1         | 0/96 (0.0%)   | all           |

The PSNR of the marked images against the originals is at least 47.69 dB (kodim18). The RMS of the brightness change in the flattest 5% of 32 x 32 windows is at most 1.127 (kodim10). On one x86_64 Windows desktop a 12-megapixel photo took about 125 ms to embed and 38 ms to read on a single thread, with the image already decoded in memory.

The mark has limits. In a separate run on the same 24 images with Pillow, one 32-byte text each, it survived JPEG quality 40 on every image, quality 30 on 23 and quality 20 on 3. Shrunk with Lanczos and kept as PNG, it survived on every image down to 288 pixels on the long side and on 21 at 256. Shrunk and saved as JPEG quality 75, it survived on every image at half size (384 pixels), on 20 at 320 pixels and on none at 256.

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

A resilient mark is reported as `ok, resilient`, or `ok, resilient, encrypted` when a password was given.

## How it works

Three record formats exist. By default `iwm` writes v1, with `--password` it writes v2, and with `--resilient` it writes v3. All are read. For v1 and v2 a reader looks at the first 5 bytes: other magic bytes mean there is no watermark, version 1 is read as v1, and version 2 needs a password. Any other version is reported as an unsupported format version. v3 stores no magic in the pixels and is recognized by its checksum. `iwm` tries it when no LSB watermark can be read, which includes LSB magic bytes that survived a later resilient embed. If v3 finds no mark either, the LSB error is reported.

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

### Format v3

The resilient mode does not store bits in pixels. It shifts a few DCT coefficients of a 256 x 256 version of the brightness plane, and the shift is spread over the whole image, so resizing and JPEG compression leave it in place. The record is 46 bytes:

| Offset | Size | Field                                                           |
|--------|------|-----------------------------------------------------------------|
| 0      | 1    | mode, `0x31` plain, `0x32` encrypted                            |
| 1      | 4    | salt, random when encrypted, zero when plain                    |
| 5      | 1    | text length `L`, at most 32                                     |
| 6      | 32   | text as UTF-8, padded with zero bytes                           |
| 38     | 4    | inner CRC-32 (IEEE) of bytes 0 to 37, u32 big-endian            |
| 42     | 4    | outer CRC-32 (IEEE) of bytes 0 to 41 as stored, u32 big-endian  |

With a password, bytes 5 to 41 are XORed with a ChaCha20 keystream (RFC 8439, 96-bit zero nonce, block counter 0). The key is the 32-byte Argon2id output (version 0x13, 19,456 KiB of memory, 2 iterations, parallelism 1) over the password, as UTF-8 bytes without normalization, and the 8-byte salt `IWMK` followed by bytes 1 to 4. The inner CRC is computed over the plaintext before encryption and the outer CRC over the bytes as stored, after it.

The 46 bytes become 368 bits, most significant bit first, followed by 6 zero bits. A rate-1/2 convolutional code with constraint length 7 turns these 374 bits into `M = 748` coded bits. The encoder starts in state 0. For each input bit the register is `(bit << 6) | state`, the two output bits are the parity of the register ANDed with 171 (octal) and then with 133 (octal), and the new state is the register shifted right by one.

Embedding and reading both work on the image after its EXIF orientation is applied. Images with more than 8 bits per channel, or in grayscale, are converted to 8-bit RGB first. The brightness is `Y = 0.299 R + 0.587 G + 0.114 B` on the 8-bit values, with alpha ignored. It is resampled to 256 x 256 with a triangle kernel. For output index `i` along an axis with `scale = source size / 256`, the support is `max(scale, 1)`, the center is `(i + 0.5) * scale - 0.5`, and the taps are the source indices `j` from `ceil(center - support)` to `floor(center + support)` with weight `1 - |j - center| / support` where that is positive. Indices outside the image are clamped to the edge, the weights are scaled to sum to 1, and the horizontal pass runs before the vertical one.

The plane is cut into 32 x 32 blocks of 8 x 8 samples `S[y][x]`, numbered in row-major order. Each block gets the orthonormal 8 x 8 DCT-II, `F[k][l] = a(k) a(l) * sum over y, x of S[y][x] cos((2y + 1) k pi / 16) cos((2x + 1) l pi / 16)`, with `a(0) = sqrt(1/8)` and `a(k) = 1/2` for `k > 0`. Here `k` is the vertical and `l` the horizontal frequency. The mark uses the coefficients `(k, l)` = (1, 1), (1, 2), (2, 1), (2, 2), in this order.

Block `b` carries one coded bit. The layout is a permutation `p` of 0 to 1023, keyed with the public key `iwm v3 public layout key` padded with zero bytes to 32 bytes. The key is a constant of the format, not a password. Start with `p[n] = n` for `n` from 0 to 1023. For `n = 0, 1, ..., 1023`, draw `t` uniformly with `0 <= t < 1024 - n` and swap `p[n]` and `p[n + t]`. The draws use the v2 sampler: the ChaCha20 keystream (RFC 8439, 96-bit zero nonce, block counter 0) taken 8 bytes at a time as a little-endian u64, with values at or above `floor(2^64 / range) * range` discarded and the rest taken modulo `range = 1024 - n`. Block `p[n]` carries coded bit `n mod 748`, so coded bits 0 to 275 are carried by two blocks.

Every coefficient has a dither value `u` in [0, 1). For block `b` and the coefficient at position `m` (0 to 3) in the list above, `u` comes from word `4b + m` of a second ChaCha20 stream under the same key, with a nonce that is the byte `01` followed by 11 zero bytes and block counter 0. The words are read as u32 little-endian and `u = (word >> 8) / 2^24`.

To embed coded bit `c` into a coefficient `v` with step `D = 16`, the coefficient becomes `v' = (round(v / D - u - c / 2) + u + c / 2) * D`, and the inverse DCT turns the block back into samples. The difference between the changed plane and the original plane is resampled to the image size with the same kernel and added to the R, G and B values of every pixel. The results are rounded half away from zero, like every `round` in this section, and clamped to 0 to 255. Alpha is not touched. `iwm` computes the planes, the DCT and the soft values in 32-bit floats, with the kernel weights and the DCT matrix computed in 64-bit and rounded to 32-bit. Only an embedder that must match `iwm` byte for byte needs this detail.

To read, the brightness is resampled to 256 x 256 and every block goes through the DCT. For each of the four coefficients, `r = frac(v / D - u)` gives the soft value `1 - 4 * min(r, 1 - r)`, which is positive for a 0 and negative for a 1. The mean of the four values is the soft value of the block, and the soft values of all blocks carrying the same coded bit are summed. A soft-decision Viterbi decoder then finds the path from state 0 to state 0 that maximizes the sum of `+s` for an expected 0 and `-s` for an expected 1, where `s` is the summed soft value of each of the 748 coded bits. The first 368 decoded bits, most significant bit first, are the 46 record bytes, and the 6 tail bits are dropped.

The reader checks, in this order: the outer CRC (a mismatch means no watermark was found), the mode byte (`0x31` read with a password is reported as not encrypted, `0x32` read without one as password required, any other value as an unsupported format version), the decryption for `0x32`, and then `L` at most 32, zero padding, the inner CRC and valid UTF-8. A failure in the last group is reported as damaged, or as a wrong password or damaged watermark when the record is encrypted. Embedding needs 512 pixels on the short side. Reading enforces no minimum size.

### Library

The same code is available as the [`invisible-watermark`](https://crates.io/crates/invisible-watermark) library crate. It works on decoded images and byte slices and does no file I/O. It enables no codecs of the `image` crate, so you enable the ones you need, such as `png` or `jpeg`.

The resilient mode is `embed_resilient`, `embed_resilient_with_password`, `extract_resilient` and `extract_resilient_with_password`, with the constants `RESILIENT_MIN_SIDE` (512) and `RESILIENT_MAX_TEXT` (32). `extract` reads only the LSB formats and `extract_resilient` only v3. A caller that wants both does what `iwm` does: it calls `extract` first and `extract_resilient` on any error, and keeps the `extract` error if `extract_resilient` finds no watermark. Pass `extract_resilient` the image after EXIF orientation, as it was embedded. `Error` is `#[non_exhaustive]`, so a `match` on it needs a wildcard arm.

To use it on `wasm32-unknown-unknown` in a browser, also depend on `getrandom` with the `wasm_js` feature, as `crates/iwm-web` does, so the salt and nonce come from `crypto.getRandomValues`:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
getrandom = { version = "0.4", features = ["wasm_js"] }
```

## Limitations

- In the default LSB mode only lossless formats keep the watermark. JPEG compression, resizing, cropping and recompression by messengers or social networks remove it. The resilient mode survives JPEG compression and resizing, not cropping or rotation.
- Anyone can strip an LSB watermark by overwriting the low bits. Without a password they can also read the text, which is stored as plain bytes.
- The v2 prefix is not encrypted, so anyone can tell that an image carries a v2 watermark and read its salt. In v3 the mode byte and the 4-byte salt are not encrypted either, so anyone who runs the reader can tell that an image carries a resilient mark and whether it is encrypted.
- A password protects the text only as well as the password resists guessing.
- Statistical steganalysis can still detect that low bits were changed.
- A password shows who knew the password, not who owns the image: anyone can embed any text in any image.
- A password makes large texts slow and memory-hungry. On one x86_64 Windows desktop, embedding 4 MB of text into a 12-megapixel image took about 15 seconds and 560 MiB of memory with a password, against 0.4 seconds and 130 MiB without, and `verify --password` on the result took about the same as the embed.
- The resilient mark is a faint low-frequency texture. It can be seen on flat areas such as clear sky, white or pastel walls and studio backgrounds when the marked image is compared side by side with the original at full size. On textured areas it is invisible.
- The resilient mode needs at least 512 pixels on the short side to embed, holds at most 32 bytes of text, and breaks when the image is cropped or rotated.
- Embedding a resilient mark into an image that already carries an LSB watermark destroys the LSB watermark. Embedding the LSB watermark into a resilient image keeps both.
- With a password the resilient record has a 32-bit CRC as its only integrity check, and the salt is 4 bytes.
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

## Benchmark

`crates/iwm-bench` is the program behind the table in [Surviving JPEG and resizing](#surviving-jpeg-and-resizing). It is not published. It embeds four 32-byte texts into every PNG and JPEG file of a directory, applies the edits and counts how often the text reads back. To rerun it on the Kodak set, download `kodim01.png` to `kodim24.png` from <https://r0k.us/graphics/kodak/kodak/> into a folder named `kodak`, for example with a loop, and run the benchmark in release mode:

```
mkdir kodak
for i in $(seq -w 1 24); do curl -sSf -o "kodak/kodim$i.png" "https://r0k.us/graphics/kodak/kodak/kodim$i.png"; done
cargo run --release -p iwm-bench -- kodak
```

## Roadmap

- [x] v0.1 Rust rewrite: versioned format with checksum, UTF-8 text, lossless output, CLI, prebuilt binaries
- [x] v0.2 Password protection (encryption and scattered bits), `verify`, `--text-file`, crates.io release
- [x] v0.3 In-browser demo (WebAssembly)
- [x] v0.4 A mode that survives JPEG compression and resizing, with error correction
- [ ] v0.5 Python package
- [ ] v1.0 Stable format and API

The earlier C# WinForms version is kept in git history under the [`legacy-csharp`](https://github.com/ataoytun/InvisibleWatermark/tree/legacy-csharp) tag.

## License

MIT, see [LICENSE](https://github.com/ataoytun/InvisibleWatermark/blob/main/LICENSE).
