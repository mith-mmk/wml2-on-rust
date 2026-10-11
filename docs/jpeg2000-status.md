# JPEG2000 first implementation milestone

This is a **new implementation from public main e42d2cf** (WML2 0.0.35), not
restoration of previously lost unpublished code. The package version, existing
codec dependencies, default feature set, README and legacy RGBA8 callback API
are preserved. No external JPEG2000 implementation is linked or invoked by the
Rust decoder. The algorithms were implemented from ITU-T T.800 (08/2002),
Annexes A–G and I:
https://www.itu.int/rec/T-REC-T.800-200208-S/en

## Supported profile

Enable the optional, nondefault `jpeg2000` feature. Both raw J2K codestreams and
JP2 files reach this decoder through `draw::image_load`, `image_from_with_limits`
and the existing callback entry points.

- One unsigned 8-bit grayscale component, no subsampling; zero canvas/tile origin.
- Dimensions 1–16 in each axis; one tile and exactly one tile-part.
- Reversible integer 5/3; zero or one decomposition. One decomposition requires
  both dimensions to be at least two.
- LRCP, one layer, implicit precincts, 16×16 code-blocks, style zero, no MCT.
- No quantization; coefficient precision at most 16 bits; full code-block coding
  passes only. A single code-block per band permits single-leaf tag trees.
- JP2 brand `jp2 `, ordinary box lengths (zero length only for final jp2c), one
  ihdr and one enumerated grayscale colr (17). Free boxes and COM are accepted
  and charged against the aggregate metadata budget.

This is deliberately an experimental subset, not a general Part 1 conforming
decoder. RGB/alpha, signed/16-bit samples, larger images, multiple tiles/layers,
other progression orders, 9/7, deeper decompositions, partial passes, code-block
styles, explicit precincts, tile header overrides, ROI, palettes, ICC and extended
JP2 box lengths are not connected. Unsupported profiles return a clear error;
there is no fallback to another product. Existing large JP2 samples are outside
this milestone's profile.

## Bounds and callback contract

All box, marker, packet header and code-block extents are validated before use.
No unsafe code or alignment reinterpretation is used. Packet fields, zero-plane
unary codes and code-block pass counts are bounded. MQ byte stuffing is checked;
standard virtual arithmetic termination bytes are permitted. JPEG2000 does not
provide an integrity checksum: an arbitrary entropy mutation can still represent
a valid alternate image, so rejecting every changed byte is not promised.

The existing input budget is checked before reading/allocating the input buffer.
Pixel and frame budgets are checked before reconstruction; expanded memory is
conservatively charged as `64 * width * height + 8192`, excluding separately
charged input. This bounds the small vectors, MQ contexts, coefficients, lifting
scratch and RGBA output. No allocation is proportional to an unvalidated canvas,
packet length or declared bit depth. Integer lifting uses floor division for
negative values and whole-sample symmetric edges; horizontal synthesis precedes
vertical synthesis. With at most 16 coefficient bits and one level, i32 lifting
arithmetic is bounded well below overflow.

Output is opaque RGBA8, with DC shift +128 and clamping to 0–255. Grayscale is
replicated into RGB. Parsing and reconstruction finish before init; malformed or
unsupported input causes no callback. Init Abort skips draw and terminates once.
A successful draw (including Abort) terminates once; callback errors propagate
immediately without additional callbacks.

The new unconditional `ImageFormat::Jpeg2000` variant is an additive public enum
change: downstream exhaustive matches may need updating. Signatures remain
recognized with the feature disabled, but decoder availability is false and
decode returns an error. `InitOptions` has no added fields.

## Reproduce fixtures and inspect output

`wml2/tests/fixtures/review/generate_jpeg2000.py` creates 44 original synthetic
Gray8 sources and 88 J2K/JP2 encodings. Pillow 12.3.0 / OpenJPEG 2.5.4 were used
only as fixture encoders and independent pixel oracles. The generator verifies
that each encoded file decodes to its source bytes; Cargo tests compare the Rust
RGBA output with checked-in `.gray` sources. Input hashes and tool versions are
in `jpeg2000-manifest.txt` beside the fixtures. Shapes include odd/even dimensions,
1×1, 2×2 and 16×16; patterns include noise, ramp, flat and checkerboard.

```
python wml2/tests/fixtures/review/generate_jpeg2000.py
cargo test -p wml2 --locked --no-default-features --features jpeg2000 --lib --test jpeg2000
cargo run -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify -- \
  wml2/tests/fixtures/review/jp2_16x16_d1_noise.jp2 .test-noise.ppm 20000
```

The example exports actual public-API decoded pixels to PPM and times full
`image_load` calls. Locally, noise, ramp and checker outputs were displayed as a
nearest-neighbor contact sheet and independently compared with OpenJPEG pixels.
A further 256 generated random cases (seed 200053, dimensions 1–16, both
containers, zero/one levels, singleton dimensions at level zero) matched exactly.

## Performance

Release Rust 1.91, shared Linux host. Newly implemented 16×16 decodes, 100 warmups
and 20,000 measured calls: noise JP2 242.843 us, ramp J2K 65.970 us, checker JP2
30.318 us per decode. There is no previous decoder at the base to compare with.
This implementation favors a small auditable profile over throughput.

Existing 256×256 RGB BMP decoding was measured on base/head in alternating
B/H/H/B/B/H order, CPU affinity fixed, 100 warmups and 2,000 calls per process.
Base: 408.732, 427.969, 416.163 us; head: 401.414, 393.128, 381.680 us.
All six exported outputs have identical SHA256. No slowdown was observed in this
small shared-host sample; this is not a guarantee for all codecs or machines.
Raw results are in `jpeg2000-benchmark.json`. The same example source was used
on the base with `bmp` and on the head with `bmp,jpeg2000`.

## Local verification (2026-10-11)

Rust 1.91.0; initialize repository submodules and generate the ignored workspace
Cargo.lock before using `--locked` on a fresh checkout.

| Check | Result |
| --- | --- |
| JPEG2000-only unit + targeted integration | 22 passed |
| Minimal build, feature disabled, lib/tests | 26 passed |
| Default codecs + jpeg2000 + high-bit-depth, lib/tests | 234 passed, 4 ignored |
| jpeg2000 + noretoro, lib + targeted integration | 22 passed |
| jpeg2000 + TGA/PCX/DDS/PIC2, lib + new/collected formats | 51 passed |
| Default codecs + jpeg2000 doctests | 12 passed |
| Clippy minimal/jpeg2000 lib, targeted tests and example | Passed; no new diagnostics versus base minimal |
| cargo fmt --all --check; git diff --check | Passed |
| wasm32-unknown-unknown jpeg2000-only cargo check | Passed; compile only, not wasm execution |
| Additional randomized OpenJPEG comparisons | 256/256 exact matches |
| Actual display and BMP base/head benchmark | Passed as described above |

Focused tests exercise detection with the feature on/off, all 88 oracle fixtures,
negative lifting rounding and edges, exact resource limits, unsupported COD/SIZ/
QCD/JP2 settings, all truncated prefixes, invalid box/marker/tile extents,
callback Abort/error behavior and deterministic single-byte mutations. The final
runs had no failures. An initially incorrect hand-calculated negative lifting
expectation was corrected to the standard floor-division result before these runs.
Existing warnings remain (including unused retro draw_rgba in some feature sets).

No full external sample corpus, Windows/macOS execution, AVIF-specific matrix,
remote CI run, or larger JPEG2000 profile was exercised for this milestone.

For the additional randomized oracle run:

```
python wml2/tests/fixtures/review/check_jpeg2000_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify
```
