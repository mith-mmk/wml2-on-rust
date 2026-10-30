# Baseline JPEG reduced decoding

`wml2::jpeg::image_from_scaled(bytes, JpegScale::{Half, Quarter, Eighth})`
returns `ScaledJpegImage { image: ImageBuffer, dimensions: ImageDimensions }`.
The `_with_limits(bytes, scale, DecodeLimits)` variant uses explicit budgets.
The JPEG feature still requires exactly one existing IDCT feature. The new
reduced kernel is shared by `idct_llm`, `idct_aan` and `idct_slower`; those
features continue selecting the ordinary full-resolution kernel unchanged.

This additive route accepts 8-bit SOF0 baseline Huffman JPEG with one grayscale
scan, or one interleaved three-component RGB/YCbCr scan. Common 4:4:4, 4:2:2 and
4:2:0 are covered by independent references. Integer sampling ratios are
supported; fractional ratios and MCUs above ten blocks are rejected. Scan
component order is mapped by ID, not assumed to equal frame order. A grayscale
scan has one block per MCU even when its SOF sampling factors exceed one.
Restart intervals count coded MCUs, validate RST0..7 order and reset DC predictors.
Odd sizes and partial edge MCUs are cropped to the rounded-up output dimensions.

Progressive, noninterleaved color/multiple scans, extended/arithmetic/lossless
processes, deferred height, and CMYK/YCCK are outside the initial reduced route.
They return an error; there is no full-size-decode fallback. Ordinary
`draw::image_from`, loader/callback and progressive paths retain their behavior.
The new route is owned-buffer-only and synchronous, including with `multithread`.
It does not add a CLI/Viewer thumbnail path or change the existing callbacks.

## Pixels and geometry

`dimensions.coded` is the SOF canvas. `dimensions.output` is the actual reduced
canvas, `ceil(coded.width / denominator) × ceil(coded.height / denominator)`.
`dimensions.intrinsic_display` stays unknown. EXIF orientation is neither applied
nor used to swap dimensions. Source metadata width/height remain coded dimensions;
use typed output dimensions or `image.width/height` to describe returned storage.
Header-only information queries still describe ordinary full-resolution output.

Output is straight, opaque RGBA8. Gray is replicated to RGB. Adobe transform 0
selects direct RGB, transform 1 selects YCbCr; otherwise JFIF/component identifiers
select the usual RGB/YCbCr interpretation. No ICC color transform is applied.
Upsampling uses component-grid replication without fancy chroma smoothing.
Thus reduced output need not equal a resize of the ordinary decoder's pixels.

Each entropy-coded block is read, but only one MCU's component samples and DC
predictors are retained. Reduced IDCT directly computes averages of adjacent
**unclamped** inverse-transform samples: 4×4 averages pairs, 2×2 averages groups
of four, and 1×1 uses DC/8. Odd high-frequency coefficients still contribute.
No full-resolution image or image-wide coefficient/plane buffer is constructed.
A component may keep a larger native IDCT when subsampling exactly covers the
reduced output grid (4:2:0 half-size uses Y=4, Cb/Cr=8). This follows the reduced
transform described in libjpeg's [jidctred.c](https://github.com/libjpeg-turbo/libjpeg-turbo/blob/2.1.5/jidctred.c)
and the component-size selection in [jdmaster.c](https://github.com/libjpeg-turbo/libjpeg-turbo/blob/3.1.2/src/jdmaster.c).
The separable floating-point implementation uses mirrored even/odd terms and
skips basis terms that vanish; it does not copy libjpeg implementation code.

## Metadata and bounds

Header scanning borrows marker payloads and preflights retained APP/COM bytes
plus 256 bytes per entry before cloning. EXIF is preserved as `EXIF Raw`, including
its `Exif\0\0` envelope, without parsing TIFF offsets or applying orientation.
Standard XMP is retained under `http://ns.adobe.com/xap/1.0/`, comments under
`comment`, and other APP payloads under `AppN Raw` (last entry per key wins).
The reduced route intentionally does not run the ordinary structured EXIF/C2PA
parser; callers needing that interpretation can use their existing workflow.

ICC APP2 chunks are assembled by sequence number regardless of marker order.
Missing, duplicate or inconsistent envelopes discard the association as a whole.
A three-component source with an RGB-profile declaration uses `ICC Profile`;
other source profiles use `Source ICC Profile` so Gray profiles do not label
replicated RGB output. ICC bytes are preserved, not validated or converted.

- `input_bytes` checks the complete borrowed input length.
- `pixels` checks **coded** width × height, even for 1/8 output. Reduction cannot
  bypass source-dimension admission. `frames` must admit one image.
- `expanded_bytes` checks reduced RGBA bytes + a conservative 64 KiB fixed
  table/MCU/IDCT/header allowance + eight times the admitted metadata accounting.
  Checked arithmetic precedes output and metadata allocation. This allowance is
  not a measurement of total process RSS or caller-owned storage.
- `metadata_bytes` checks all retained APP/COM payloads plus per-entry overhead;
  opaque EXIF never expands into a parsed object tree.
- `animation_bytes` must admit the single retained reduced canvas.

The parser validates lengths, SOF/SOS selectors/sampling, referenced nonzero
8-bit quantization tables, bounded canonical Huffman tables/symbol categories,
AC runs, checked DC predictor updates and restart boundaries. It requires EOI
immediately after the single scan (apart from byte padding/marker fill). Errors
return no partial image. Trailing bytes after EOI are ignored. This is not a
universal JPEG conformance checker or recovery decoder.

## Verification

`tests/jpeg_scaled.rs` covers 36 independent oracle combinations, 90 restart
comparisons, tiny/odd geometry, 4:4:4/4:2:2/4:2:0, Gray/direct RGB, component order,
noninterleaved Gray geometry, opaque EXIF orientation, ICC association, malformed
headers/tables/entropy/restarts, truncation and bounded byte mutations. Private
kernel tests demonstrate high-frequency contribution and averaging before
clamping; malformed AC runs and predictor overflow return errors.

An allocation probe successfully decodes 1152×896 at 1/8 with a 140,000-byte
expanded budget: the largest individual heap request is exactly the 64,512-byte
output. A 60,000-byte EXIF payload under a tiny metadata/workspace budget rejects
before any heap request of 1 KiB or more. The implementation and live-heap
benchmark separately establish the absence of a coded raster/coefficient store;
maximum individual allocation alone is not a total-memory measurement.

References were produced by libjpeg-turbo **2.1.5**, `JDCT_ISLOW`,
`do_fancy_upsampling=FALSE`, `scale_num=1`, `scale_denom=2/4/8`, `JCS_EXT_RGBA`.
Compressed goldens and uncompressed SHA256/dimensions are in
[`jpeg-scaled-oracles.txt`](../wml2/tests/fixtures/review/jpeg-scaled-oracles.txt).
Across the 36 checked combinations, Gray/direct RGB differ by at most 1/255;
YCbCr differs by at most 3/255 (integer-IDCT/conversion rounding). A local check
on the 1152×896 RGB benchmark also stays within 3/255 at every scale. These
bounds describe the checked inputs, not every possible JPEG.

Regenerate with Pillow 12.3.0 for encoding and matching libjpeg-turbo development
headers/runtime for the independent decoder:

```sh
cc -O2 wml2/tests/fixtures/review/jpeg_scaled_oracle.c -ljpeg -o .test-oracle
python3 wml2/tests/fixtures/review/generate_jpeg_scaled.py ./.test-oracle
cargo test -p wml2 --no-default-features --features jpeg,idct_llm --lib --test jpeg_scaled --test jpeg_restart --test image_info
```

The generation script reuses the unchanged restart inputs and derives benchmark
RGB/Gray JPEGs from the existing bundled `sample.bmp`, quality 90 (RGB 4:2:0).
Generated odd/small inputs use deterministic patterns, quality 91. The WML2
encoder is not used to generate the references.

## Local timing and memory evidence

Linux 6.18.55 x86_64, AMD EPYC 9V74, Rust 1.91.0 release, `jpeg,idct_llm`, one
thread pinned to CPU 0. Each of three separate processes uses three warmups and
20 timed resident-buffer decodes; mode order alternates between processes.
Values below are medians of the three per-process medians. Source file reads,
hashing, PPM export and printing are outside the timed interval;
metadata construction inside the decode is timed. Inputs and whole decode work
are shared across modes; no claim of proportional runtime or RSS improvement.

| Input | Mode | Output dimensions | Time µs | Peak additional requested heap bytes | Process peak RSS KiB (median) |
| --- | --- | --- | ---: | ---: | ---: |
| RGB 1152×896, 170943 bytes | full | 1152×896 | 23121.450 | 4319757 | 10100 |
| same | 1/2 | 576×448 | 21566.234 | 1033519 | 2868 |
| same | 1/4 | 288×224 | 11449.913 | 259375 | 2232 |
| same | 1/8 | 144×112 | 7966.270 | 65839 | 1980 |
| Gray 1152×896, 147189 bytes | full | 1152×896 | 11480.398 | 4288035 | 10024 |
| same | 1/2 | 576×448 | 10063.502 | 1033519 | 2752 |
| same | 1/4 | 288×224 | 7442.080 | 259375 | 2028 |
| same | 1/8 | 144×112 | 5592.031 | 65839 | 1912 |
| RGB 129×75, 9645 bytes | full | 129×75 | 488.136 | 68391 | 1780 |
| same | 1/2 | 65×38 | 425.191 | 11207 | 1632 |
| same | 1/4 | 33×19 | 347.564 | 3835 | 1608 |
| same | 1/8 | 17×10 | 279.491 | 2007 | 1608 |

Requested live heap uses a counting System allocator, resets the high-water
baseline after source loading/warmup, and holds the output through the peak
sample. It excludes allocator overhead/retention, stack, code and caller input.
Linux `/proc/self/status` VmHWM is a separate whole-process historical peak that
includes those costs and allocator reuse across warmups; it is not the decoder
workspace, and small process RSS differences should not be overinterpreted.
Every mode still consumes entropy for all coded MCUs. 1/2 saves little time on
these inputs even though output storage is about one quarter. No other-OS/WASM
runtime or universal speedup is claimed.

The existing full path was also checked against immutable base `81178d7` in
three alternating process pairs, 3 warmups +30 calls on the RGB benchmark:
base medians 22104.558/22128.314/22797.452 µs, candidate
22279.712/22399.132/22700.316 µs. Median-of-medians changes by 1.22%, within
local process variation; both retain exactly 4319757 additional requested heap
bytes and full-output FNV-1a `149918f302f29133`. No ordinary IDCT/decoder code is
changed. Exported half/quarter/eighth PPMs are readable; half-size display was
visually checked, and all benchmark scales compared to the independent decoder.

Raw process results and output hashes are in
[`jpeg-scaled-benchmark.txt`](../wml2/tests/fixtures/review/jpeg-scaled-benchmark.txt).
Reproduce the example without a native runtime dependency:

```sh
cargo build -p wml2 --release --no-default-features --features jpeg,idct_llm --example jpeg_scaled
# Use a separate CARGO_TARGET_DIR for a second worktree's baseline binary.
taskset -c 0 target/release/examples/jpeg_scaled wml2/tests/fixtures/review/jpeg-scaled-bench.jpg 4 20 .test-quarter.ppm
# Replace 4 with full / 2 / 8; Linux prints process VmHWM separately.
```

Final local checks: 107 default-feature tests passed (one manual information
benchmark ignored); each standalone LL&M/AAN/reference-IDCT matrix passed 42
tests, and JPEG+multithread passed 43. Feature-off compilation, fmt and diff
checks passed. Clippy completes with existing library warnings and no new
scaled API/test/example diagnostics.
