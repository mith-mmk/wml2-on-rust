# Bounded JPEG2000 multiple tile-parts

The later [unknown/mixed total extension](jpeg2000-unknown-tile-parts.md) records
support added after this checkpoint.

Continuation of `0f35f184424abd20d4f6bbfa4c1c6103b4e584c1` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Ordinary remote read/fetch
matched that SHA before implementation. Repository AGENTS.md, Rust 1.91.0,
Cargo and the real wasm32 target were checked. Temporary folders use `.test*`.

## Profile and implementation

Multiple tile-parts extend the existing packet/5/3 path without a new transform
or partial code-block pass continuation. A tile has 1–18 nonempty parts, with a
known identical TNsot count, strict TPsot 0 through count-1, and identical Isot.
All parts for one tile are contiguous; tiles remain in raster order. Every part
header contains SOD immediately after SOT. Each part ends on a whole packet
boundary. With one LRCP layer, one implicit precinct, six resolutions and three
components, 18 bounds the number of nonempty parts in the supported profile.

The framing parser checks all parts before joining their borrowed bodies. The
packet parser preserves cumulative body ends and checks them against actual
packet ends, rejecting splits inside packet headers or arithmetic segments.
Concatenation alone would reproduce the original byte stream and miss those
invalid boundaries. Const specialization retains the existing single-part path
without a join allocation or packet-boundary checks.

Multipart workspace is `64 * actual_width * actual_height * components + 8192`
plus the joined payload size of the largest multipart tile. Each tile's join
allocation is checked before reserve/copy; it is dropped before the next tile.
A single-part tile uses the existing budget. Input limits still bound bytes read,
and framing/packets/EOC complete before output initialization.

Gray8/RGB8, RCT on/off, reversible 5/3, levels 0–5, actual/tile axes up to 1024,
reference ends up to 32768, at most 256 tiles, legal bounded code-block shapes,
full passes and style zero remain as previously verified. Public APIs, RGBA8,
other codecs, defaults, dependencies and WML2 0.0.35 are unchanged.

## Independent fixture evidence

The fixture generator requests OpenJPEG PLT packet-length markers via
[Pillow's documented `plt=True` option](https://pillow.readthedocs.io/en/stable/handbook/image-file-formats.html#jpeg-2000).
It parses the independent PLT lengths, checks their sum against tile payload
extent, groups whole packets, removes PLT, and rebuilds SOT/Psot/JP2 box lengths.
Neither Rust's packet parser nor decoder output chooses fixture boundaries.
OpenJPEG checks both the original and transformed streams against source
pixels. OpenJPEG remains an encoder/oracle only, outside production dependencies.

The positive matrix contains 126 source-exact encodings: 122 multipart and
four single-part controls. It spans Gray/RGB/RCT, levels 1–5, 18 parts, packet
and paired packet groups, tiles mixing one/multiple parts, clipped/nonzero
origins, reference end 32768, 1024² canvas, 256 tiles and tiny flat edges. Level0
RGB multipart cases are covered by random testing. Peak fixed join is 46,030
bytes. Positive/negative encoding and source hashes were checked independently.
The negative matrix has 12 valid-unsupported unknown-total/interleaved files
and 36 malformed split-header/body, missing/inconsistent/incorrect sequence
and 19-part files. OpenJPEG permissively accepts some malformed byte-identical
joins; that does not make them supported. Reasons and oracle outcomes are
separately persisted in `jpeg2000-parts-proof.json`.

## Native verification and review

Release workspace regression passed 362 tests, zero failures, 12 existing
ignored tests across 43 targets including doctests. Feature-off minimal native
regression passed 16. New fixed tests check every source RGBA byte/dimension and
exact/one-short input, pixel, workspace and frame budgets. Early/late part
framing, truncation and repaired empty bodies reject before output callbacks.
Bounded packet-byte mutations may decode or return errors; no panic occurred.
Clippy reports only 11 existing warnings elsewhere; no new JPEG2000 warning.
Formatting, diff and Python syntax checks pass.

Seeded independent-PLT random verification has 24 source-exact geometries and
144 pixel comparisons (126 multipart). Five further geometries / 30 comparisons
have the previously known OpenJPEG encoder/source mismatch and are classified
as compatibility-only; Rust matches OpenJPEG on every one. They are excluded
from source-exact counts. Reports include source/encoding/pixel hashes, profile
parameters and PLT proofs. Six public-API PPM outputs were displayed and match
OpenJPEG, including 18 parts, 256 tiles, 1024² and 1×1.

Independent core review found no actionable correctness, resource or API defect
in source SHA256
`4b677999f4a0d50a6ff0871612b9023f03b2d3b5f426a91bc62ad6815f18c1cd`.
Checkpoint `79cf2dbe08aa0243e933766c80662ce91523a5dc` was committed, normally
pushed on the same work branch and matched by remote SHA. Main and stable
`jpeg2000` were not updated. Production changes are confined to `jpeg2000.rs`.

## Real WASM verification

Minimal and default-codec builds each pass 1330 pixel/API cases: 1329 source-
exact encodings and the prior singleton compatibility case, separately counted.
Both reject 72 fixed negative inputs before callbacks, including all 48 new
multipart negatives. Node.js v24.19.0 instantiates the actual wasm32 module
without host imports. Checks include every RGBA byte/dimension, exact/one-short
limits, limit scope restoration, callbacks, init/draw Abort, truncation, missing
parts, late framing and repaired empty bodies.

Both runs have checkpoint `79cf2dbe08aa0243e933766c80662ce91523a5dc`, source
SHA256 `4b677999f4a0d50a6ff0871612b9023f03b2d3b5f426a91bc62ad6815f18c1cd`,
and identical fixture-manifest hashes. The dirty-tree flag records the untracked
report draft; production/test inputs were already committed and remained unchanged.
Linear memory, including embedded fixture bytes, was 74,776,576 bytes minimal
and 74,842,112 bytes default. Module/source/manifest hashes and checks are in
`jpeg2000-wasm-runtime.json` under `tile_parts_minimal/default`.

## Paired performance

Immutable release verifiers compare baseline `0f35f18` and checkpoint `79cf2db`.
Fixed CPU affinity, B/H/H/B/H/B (three processes per version), 100 warmups,
Rust1.91 release; no concurrent CPU tests during final timing. All paired output
hashes match. Values are whole-process child CPU divided by timed calls plus101,
including startup/export/warmup. The initial incomplete run overlapping fixture
generation was stopped; it is excluded from these measurements.

| Existing input | Baseline CPU us/call | Extension | CPU change |
| --- | ---: | ---: | ---: |
| Gray16 | 51.874 | 51.383 | -0.95% |
| Gray65 | 1185.152 | 1218.902 | +2.85% |
| RGB16 | 305.049 | 308.323 | +1.07% |
| RGB65 | 4368.003 | 4130.761 | -5.43% |
| Gray256tiles | 794.723 | 799.973 | +0.66% |
| RGB9tiles | 116335.019 | 147771.727 | +27.02% |
| RGB2oddtiles | 196.777 | 189.965 | -3.46% |
| RGB9tiles targeted repeat | 127162.113 | 131119.802 | +3.11% |

Six initial cases range from -5.43% to +2.85%. RGB9tiles initially averaged
+27.02%: candidate samples were 205491/114432/123392 CPU us/call against baseline
116495/112160/120350. The timed20-call phase of the first candidate was120338
wall us/call, so the extra whole-process cost was outside that phase. A targeted
repeat averaged +3.11%; both versions also had large fluctuations. No source
change or broader regression rerun was needed. Original and repeat rows remain
in `jpeg2000-parts-benchmark.json`; no outlier is silently removed.

These short shared-host samples do not demonstrate a consistent major
regression, nor guarantee no cross-host/cumulative regression. The historical
larger-RGB +6.11% result against an older baseline remains a separate caveat.
The baseline cannot decode new multipart inputs, so paired timings use existing
single-part inputs. Binary/input/output hashes and methodology are recorded.

Full scope/check evidence is in `jpeg2000-parts-validation.json`, random cases
in `jpeg2000-parts-random.json`, and the displayed six-output contact sheet in
`jpeg2000-parts-display.png` with output hashes in `jpeg2000-parts-display.json`.

## Reproduction

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_parts.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_parts_random.py \
  /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

## Remaining limits

Unknown TNsot, empty parts, more than 18 parts, interleaved or unordered tiles,
tile-header overrides, split packets, multiple quality layers/partial passes,
9/7, other progressions, explicit precincts/SOP/EPH, subsampling, signed or
high-bit-depth/alpha components and ROI remain unsupported. Real Node WASM
execution does not verify browser UI, other OS or optional AVIF matrices.
