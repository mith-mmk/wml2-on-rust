# JPEG2000 continuation: review, WASM execution and next profile

Resume point: `d1e128f01812d591e67f9811d8dab0c5c4eb85fb` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Normal Git remote read/fetch
matched that SHA; stable `jpeg2000` remained at `7043bf083203234c1193cf1c22a70ab61d6dc3df`.
Repository AGENTS.md was read; the workspace `.agents` directory was empty and
there were no repository `.agents/skills` instructions. Working folders use
`.test*`. No old ZIP investigation or GitHub app approval flow was resumed.

## RGB/API integration review

An independent correctness review found no concrete defect in the supported
RGB subset or its integration into the existing memory/callback APIs. Three
fixed regression gaps were filled: exact expected width/height assertions;
neutral, monochrome and flat RGB images (empty packets and zero RCT chroma);
and active component packets with distinct QCC maximum bit-plane precision.

The RGB generator now has 46 sources / 184 encodings. Three additional 1×1
QCC fixtures increase Mb from 9 to 10 for component 0, 1 or 2, inserting a zero
into the corresponding unary zero-plane tag to keep the encoded depth/passes
unchanged. All three decode to RGB (31,83,197) with OpenJPEG and Rust; ignoring
QCC fails the full-pass check. `generate_jpeg2000_qcc.py` reproduces them, and
`jpeg2000-qcc-manifest.txt` records their hashes. OpenJPEG remains only a fixture
encoder/pixel oracle, with no production implementation dependency.

The JPEG2000-only native unit/integration run passed (19 + 17 tests); the new
distinct-QCC integration test passed separately. The independent review's
unchanged baseline run also passed all 36 unit/integration tests. Ordinary
release workspace regression with default codecs plus JPEG2000 passed 346
tests, 0 failed, 12 existing ignored; formatting and diff checks passed.

## Actual WASM execution

`wml2-test/scripts/verify_jpeg2000_wasm.py` builds a disposable cdylib against
this repository's wml2 crate, embedding the existing synthetic source/encoding
bytes. It instantiates and executes the actual wasm32-unknown-unknown module
with Node.js v24.19.0 using no host imports. It does not change production API,
crate types, output format, dependencies, or add unsafe decoder code.

Each fixture checks public-API output dimensions and every RGBA byte; exact and
one-below input/pixel/expanded/frame/metadata limits; finite limit scope
restoration; callback output and init/draw Abort; and truncated input rejection
before callbacks. Minimal-feature execution passed all original 330 encodings;
default-codec-plus-JPEG2000 execution passed 366 encodings, including the added
neutral/monochrome/flat cases. Native QCC cases were added after those executions.
Module hashes, memory sizes and checks are in `jpeg2000-wasm-runtime.json`.
This is WASM execution in Node, not a browser UI or graphics integration test.

```sh
# With Rust 1.91+, the wasm32 target installed, and Node on PATH:
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

## Code-block geometry extension

Code-block geometry was the next dependency removed: COD, packet partitioning
and Tier-1 previously forced 16×16, rejecting common default 64×64 encodings.
The decoder now accepts all 45 power-of-two block width/height pairs, each axis
4–1024 with area at most 4096, as bounded by
[T.800 Table A.18](https://www.itu.int/rec/T-REC-T.800-200208-S/en).
COD validates both exponent offsets and their sum before shifting. Packet grids,
tag trees, cropped edge dimensions and placement use the declared dimensions.
Tier-1 validates actual dimensions and area before allocating its arrays.

Existing small Gray images keep their single-block packet parser when the
declared block can cover the canvas; smaller declared blocks use the shared
multiple-block parser. Actual Tier-1 blocks no larger than 16×16 use a dedicated
const-bound instantiation of the same algorithm. Larger blocks use a 1024-axis
instantiation with the same 4096-area and bit-depth checks. This retains the
optimizer's small-block bounds without duplicating coefficient algorithms.

Independent review found no concrete defect in exponent/area guards, fast-path
selection, zero-origin band placement or tag-tree geometry. The expanded charge
remains `64 * width * height * components + 8192` before allocations. One
resolution/component's descriptors and trees are retained at a time, and actual
cropped block scratch is at most eight bytes per coefficient (4096 coefficients
maximum); minimum 4×4 blocks increase descriptors/tree nodes within that
conservative budget. No public API, RGBA8 output, feature defaults, version or
production dependency changed.

## Geometry verification

`generate_jpeg2000_blocks.py` creates 318 encodings: all 45 shapes with partial
edges (270); 64×64 blocks at levels 0–5 on 257×129 (36); unspecified/default
OpenJPEG 64×64 blocks on 65×67 (6); and 1024×1024 level-5 64×64 blocks (6).
Gray, independent RGB and RCT RGB each contribute 106 raw J2K/JP2 encodings.
All nine maximum-area block shapes are exercised at level zero so their full
dimensions reach Tier-1. The independent oracle verifies source pixels, mode,
dimensions and actual SIZ/COD fields. Repeated generation produced the same
manifest SHA256 `c675b60d2b06127e36ea8e3cf78a351fdfaf7fa216a07bc72a73accbd4ae46ee`.
Encoding/source hashes and parameters are in `jpeg2000-blocks-manifest.txt`.

Rust public APIs match every new source RGBA byte and dimension, pass the exact
geometry-scaled memory budget and reject a budget one byte shorter. Additional
tests reject exponent offsets above 8 and areas above 4096 before output
initialization; guard actual Tier-1 dimensions; and exercise repaired short
tile payloads, invalid marker lengths and bounded header/body mutations for
4×4, 64×64, 1024×4 and 4×1024 blocks in Gray/RCT RGB. A case-selection count
error in the new mutation test was corrected before the final passing run.

Seeded variable-block random checks matched all 144 pixel comparisons across
24 geometries, both containers, Gray and both RGB MCT flags. The script's
`--vary-blocks` option selects from all legal shapes; results are recorded in
`jpeg2000-blocks-random.json`. Fixed encodings total 687: 182 Gray, 184 RGB,
three distinct-QCC images and 318 geometry images. Actual public-API decoded
4×4/64×64/1024×4 RGB output images were displayed and matched OpenJPEG pixels.

Final ordinary release workspace regression passed 350 tests, 0 failures and
12 existing ignored. Feature-off minimal lib/detection passed 16 tests. Clippy,
formatting and diff checks passed; Clippy has existing warnings elsewhere and
none in the JPEG2000 code. WASM execution in Node v24.19.0 passes all 687 fixed
encodings with minimal and default-plus-JPEG2000 features, including pixels,
limits, callbacks/Abort, truncation and scope restoration. Module/source hashes
and memory sizes are recorded in `jpeg2000-wasm-runtime.json`.

## Performance and specialization

The first widened Tier-1 implementation slowed 16×16 Gray noise by about 22%
wall/CPU time. A longer CPU-affined follow-up still showed about 15%, so this
was addressed rather than dismissed as scheduling noise. Preserving a dedicated
16-axis instantiation brought a short paired sample to +2.6%; the final longer
paired sample was −5.2% CPU for that input. Output hashes matched in every pair.
Both initial and follow-up data are retained alongside the final measurements.

Final B/H/H/B/H/B, three processes per version, fixed CPU affinity, 100 warmups,
Rust 1.91 release; public-API whole-process CPU means in microseconds/call:

| Existing 16×16-block input | Base | Extension | Change |
| --- | ---: | ---: | ---: |
| Gray 16×16 level-1 noise, 30,000 calls | 60.807 | 57.659 | −5.18% |
| Gray 65×49 level-1 noise, 500 calls | 1314.136 | 1288.392 | −1.96% |
| RGB 16×16 level-1 noise/RCT, 5,000 calls | 401.857 | 371.945 | −7.44% |
| RGB 65×49 level-2 noise/RCT, 200 calls | 4882.829 | 5181.328 | +6.11% |

The last input's wall mean was +12.94%, whereas earlier CPU samples were about
−12.7%. Shared-host variation remains material; these observations do not
guarantee no slowdown across systems. Whole-process CPU includes startup,
warmups and export, divided by total calls. Detailed rows, wall times and hashes
are in `jpeg2000-blocks-benchmark-final.json`; initial and follow-up files are
`jpeg2000-blocks-benchmark.json` and `jpeg2000-blocks-benchmark-repeat.json`.

## Reproduction and remaining work

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_blocks.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_rgb_random.py \
  /absolute/path/to/release/examples/jpeg2000_verify --cases 24 --vary-blocks
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

The subsequent [bounded multi-tile extension](jpeg2000-tiles.md) accepts at most
256 raster tiles with one part/layer each, aligned tile origins and no tile-header
overrides. The other Gray8/equal unsampled RGB8/RCT, LRCP, levels 0–5, reversible
5/3, style-zero and full-pass constraints remain. Multi-layer partial-pass
continuation needs persistent MQ/block/tag-tree state; arbitrary wavelet origins
and other progressions still need new positioning/packet ordering. 9/7, subsampling, signed/high-bit-
depth/alpha, component coding overrides, ROI, precinct overrides/SOP/EPH,
general JP2 palette/ICC boxes and extended lengths remain unsupported.
Browser UI, Windows/macOS, optional AVIF-specific matrices and external corpus
validation remain untested. No merge, release or remote CI completion was done.
