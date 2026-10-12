# Bounded JPEG2000 unknown and mixed tile-part totals

Continuation of `38e2b6b6aba8c641d3a4aedca9f91d61b22a11d8` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Ordinary remote read/fetch
matched that SHA. AGENTS.md, Rust/Cargo1.91.0 and the real wasm32 target were
checked. Temporary folders use `.test*`. OpenJPEG remains an independent
encoder/pixel oracle, with no production dependency or copied decoder.

## Selected stage and profile

Unknown TNsot extends the previously bounded multipart framing and packet/5/3
path without new quantization or code-block pass continuation. It enables valid
streams that omit total counts per header or announce a total in a later part.
[T.800 A.4.2/TableA.6](https://www.itu.int/rec/dologin_pub.asp?id=T-REC-T.800-200208-S%21%21PDF-E&lang=e&type=items)
allows every TNsot to be zero or the correct total. Thus zero/nonzero headers
may mix, but all nonzero declarations must agree with the actual group size.

TPsot starts at zero and remains strictly sequential. Actual and declared parts
are bounded to18, part bodies are nonempty, complete packets end each part,
and all parts for a tile remain contiguous in raster tile order. Psot must be
known/nonzero; only Psot advances to the next header, never marker scanning
inside compressed bytes. For unknown groups, another Isot or EOC ends the
group; the outer raster/EOC checks then validate that transition. Late totals,
incomplete/false totals, gaps/duplicates, excessive sequences and extra parts
reject before output callbacks. The 19th unknown body is rejected before join
allocation. Known/unknown singlepart groups retain borrowed data without extra
workspace charge. Multipart budgets remain old64*w*h*components+8192 plus the
peak joined tile payload, checked before reserve/copy. PacketData's independently
verified whole-packet end enforcement is unchanged.

Gray8/RGB8/RCT, reversible5/3, levels0–5, actual/tile axes≤1024, reference
ends≤32768, ≤256tiles, LRCP, one layer, full passes and style zero remain.
APIs, RGBA8, other formats, feature defaults, dependencies and WML2 0.0.35 are
unchanged. Production edits are confined to `jpeg2000.rs`.

## Fixtures and native evidence

156 fixed source-exact encodings comprise126 allzero variants,24 small mixed
variants and6 formerly unsupported aliases promoted without rewriting their
saved files. They include152 multipart and4 singlepart controls, all modes,
levels1–5, 18parts, 256tiles, 1024² canvas, clipped/global/tiny origins and
mixed single/multipart groups. Level0 is additionally covered by random tests.
Only TNsot changes in positives; original packet payloads, independent PLT
length proofs, Psot/JP2 lengths and peak join budgets remain byte-identical.
OpenJPEG validates each final positive against its existing source once; no
encoder rerun was needed. Source/encoding/header/body/PLT proof hashes are saved
in `jpeg2000-unknown-parts-proof.json`. The 20field positive manifest SHA256 is
`c396e8af87f7cc87bfc0ef9a226eda0de06930ab79759543edfe1c1e1632a482`.
54 negatives comprise48 malformed and6 valid-unsupported interleaved files.
Some malformed byte-identical joins are permissively decoded by OpenJPEG;
classifications and oracle outcomes are recorded separately. Original negative
manifests remain historical; their six unknown-total inputs now appear among
positive tests. Former zero-count mutations now use out-of-bound19, because
zero is valid in the new profile.

Ordinary release workspace regression passed364 tests, zero failures and12
existing ignored tests across43 targets, including doctests. Feature-off native
minimal regression passed16. Every new fixed encoding checks source RGBA bytes,
dimensions and exact/one-short input, pixel, workspace and frame limits. Framing,
truncation, repaired empty bodies, sequence/count errors and packet splits
reject before callbacks; bounded packet mutations decode or error without panic.
Clippy has11 existing warnings elsewhere and no new JPEG2000 warning.

Independent core review found no actionable correctness/resource/API defect in
source SHA256 `e4e9b714f02c9a41a0bb363a0989221d9b8043a24e12e181572ef7ca44174a5c`.
Checkpoint `c592916ed262254106bfcf93fd3cf0de2c0af8b6` was normally pushed and
matched by remote SHA. Main/stable jpeg2000 were not updated.

Seeded PLT plus independent header rewriting produces24 source-exact random
geometries /144 pixel comparisons (126 multipart) spanning levels0–5 and all
five header modes. Four additional geometries /24 comparisons have known
OpenJPEG encoder/source mismatches; they are recorded as compatibility-only
and Rust matches OpenJPEG on every one. They are excluded from source-exact
counts. The random JSON retains parameters and PLT/header/encoding/source/pixel
hashes for both categories.

## Real WASM and display

Minimal and default-codec builds each pass1486 pixel/API cases:1485 source-exact
encodings plus the prior singleton encoder-compatibility case, counted separately.
Both reject120 fixed negative inputs before callbacks, including54 new negatives.
Node.js v24.19.0 instantiates actual wasm32 modules without host imports and
checks every RGBA byte/dimension, exact/one-short resource limits, limit scope
restoration, callbacks, init/draw Abort, truncation, missing parts, late framing
and repaired empty-body rejection. Both runs use clean committed checkpoint
`c592916ed262254106bfcf93fd3cf0de2c0af8b6` with identical source/manifest hashes.
Linear memory includes embedded fixture data:79,036,416 bytes minimal and
79,101,952 bytes default. Module/source/manifest hashes and checks are preserved
in `jpeg2000-wasm-runtime.json` under `unknown_parts_minimal/default`.

Nine public-API PPM outputs match OpenJPEG and were actually displayed: allzero,
lateknown, known-then-unknown, alternating and knownends,18parts,256tiles,1024²
and1×1. Contact sheet/output hashes are in `jpeg2000-unknown-parts-display.*`.

## Paired performance

Immutable release verifiers compare baseline `38e2b6b` with checkpoint `c592916`.
Rust1.91 release, fixed CPU affinity, B/H/H/B/H/B (three processes/version),100
warmups, and no concurrent CPU tests during timing. All paired output hashes
match. Whole-process child CPU divided by timed calls plus101 includes startup,
export and warmup. Binary/input/output/source hashes and raw CPU/wall rows are
saved in `jpeg2000-unknown-parts-benchmark.json`.

| Existing input | Baseline CPU us/call | Extension | CPU change |
| --- | ---: | ---: | ---: |
| Gray16 | 52.850 | 51.664 | -2.24% |
| Gray65 | 1228.610 | 1176.815 | -4.22% |
| RGB16 | 291.599 | 287.351 | -1.46% |
| RGB65 | 4144.928 | 4045.698 | -2.39% |
| Gray256tiles | 917.705 | 889.969 | -3.02% |
| RGB9tiles | 116789.424 | 118651.612 | +1.59% |
| RGB2oddtiles | 185.216 | 195.474 | +5.54% |

The seven samples range from -4.22% to +5.54%; the maximum increase is retained,
not discarded. No major deterioration appears in these short measurements.
Shared-host variation remains material and the samples do not guarantee no
cumulative/cross-host regression. The historical larger-RGB +6.11% result against
an older baseline, and the previous stage's initial +27% outlier / +3.11% targeted
repeat, remain separate historical records. No unnecessary regression repeat
was performed. New unknown-total inputs cannot be decoded by the baseline, so
paired timing uses existing known-total inputs.

Full validation is in `jpeg2000-unknown-parts-validation.json`; seeded source-
exact and compatibility-only cases are in `jpeg2000-unknown-parts-random.json`.

## Reproduction

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_unknown_parts.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 \
  --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_unknown_parts_random.py \
  /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

## Remaining limits

Psot0, empty or over18parts, tile interleaving/nonraster order, tile-header
coding overrides, packet splits, multiple quality layers/partial passes,9/7,
other progressions, explicit precinct/SOP/EPH, subsampling/signed/high-depth/
alpha/ROI remain unsupported. Real Node WASM execution does not establish
browser UI, other OS, optional AVIF matrices or cross-host performance.
