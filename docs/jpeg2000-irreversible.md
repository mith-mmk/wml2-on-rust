# Independent irreversible JPEG2000, unsigned8bit stage

Continuation of the reviewed lossless checkpoint `3dcd7dd8a49c665675dcddec0c77ff04621d8e2d`
on `codex/jpeg2000-rgb-reimplementation-20261011`. Normal Git access works;
Rust/Cargo1.91.0 and installed wasm32 target remain. WML2 0.0.35, default features,
existing API and other codecs are preserved. This branch is not a release.

## Scope and numerical policy

Raw J2K / enumerated Gray or sRGB JP2, unsigned Gray8 or three equal-size,
non-subsampled RGB8 components, levels0–5, LRCP one quality layer and style0 use
an independent Rust9/7 pipeline. Scalar-derived(style1) and scalar-expounded
(style2) QCD plus RGB QCC are supported. MCT selects ICT for9/7 and RCT for5/3.
Rate-limited single-layer inputs can end code-blocks after any significance,
refinement or cleanup pass. Declared passes are checked1..3P−2 and executed
exactly. Reversible5/3 keeps its full-pass integer reconstruction and16bit
coefficient bound; irreversible internal quantized precision is1..24bits.

The implementation follows [T.800 Annexes E/F/G](https://www.itu.int/rec/dologin_pub.asp?id=T-REC-T.800-200208-S%21%21PDF-E&lang=e&type=items).
It uses per-coefficient last decoded plane and an explicitly chosen midpoint
parameter r=1/2, doubled signed integers, standard band gains/K normalization,
whole-sample reflection/global parity and reduced-grid origins. Floating f64
synthesis and ICT retain precision until final nearest-even rounding, DC shift
and saturation to RGBA8 with alpha255. OpenJPEG supplies fixture encodings and
independently decoded pixels only; no decoder code is copied, linked or invoked
by the production Rust library.

Comparison policy is maximum absolute difference1 per8bit channel against the
independent decoded oracle. This is an engineering allowance for numeric and
final rounding differences, not a normative T.800 conformance tolerance. Every
lossless test still requires exact pixels. Nonzero source-to-oracle differences
measure quantization/rate loss and are recorded separately from decoder error.
No observed difference above1 is accepted by widening the gate.

Geometry and framing bounds are unchanged: image/tile axes≤1024, reference-grid
ends≤32768, at most256 raster tiles, power-of-two block axes4..1024 with area≤4096,
one through18 nonempty contiguous packet-aligned parts per tile, known/unknown/
mixed correct totals, and terminal Psot0 only in the last codestream part.
The existing conservative `64*w*h*components+8192` plus peak joined multipart
payload charge covers integer coefficients, additional f64 planes, Tier1 flags/
midpoint state, transform scratch and RGBA output. Input/metadata/frame budgets
and pre-output validation remain in force. Mandatory QCD is validated even when
all components have QCC overrides; independent review caught and corrected this
regression during implementation, and the preexisting all-QCC test covers it.

## Native and fixture checkpoint

Ordinary release workspace:372passed,0failed,12existing ignored across43targets.
Feature-off minimal:15passed. Clippy reports11existing library warnings and no
JPEG2000 warning in the selected library/integration target.

256fixed encodings comprise214base/geometry/rate/framing controls and42different
QCC/deep-derived controls. They include all6levels and3modes, default/rates2/4/10,
raw/JP2, variable blocks, nonzero/odd origins,1024²,256tiles and18parts. Equivalent
level0 style1, QCC controls and six Mb24 exponent-rescaled controls are separately
proved. The42variants keep entropy bodies/pass counts but deliberately change
quantization coefficients; their expected pixels are newly decoded by OpenJPEG.
36negative inputs comprise24malformed and12valid unsupported profiles, with
independent oracle outcomes and precise classifications. Fixed native tests check
pixels/dimensions, exact/short input/pixel/workspace/frame/metadata budgets,
callback/Abort order, truncation before init and subsequent scope restoration.
All256public PPM exports pass maxdifference1, maximum per-input MAE0.004329004329.
The [fixed pixel report](jpeg2000-irreversible-fixed.json) retains output hashes,
MAE, bias and differing-channel counts for every input.

Independent PLT/packet proofs preserve19,242included code-block declarations:
11,614cleanup,1,903significance and5,725refinement terminal stages. Fixtures record
actual SIZ/COD versus requested encoder tuples; a provisional probe discovered
that Pillow ignores list geometry parameters, and those invalid controls were
excluded and regenerated with tuples. The corrected probe has120valid cases;
a deep clipped/singleton OpenJPEG2.5.4 encoder assertion and11related cases were
recorded rather than silently counted/retried. Formal fixture generation has no
encoder failures. The random generator avoids that encoder limitation by keeping
tile axes at least2^levels; this is not a decoder acceptance restriction.

Independent production review is complete with no major actionable finding:
192legacy exact checks,192full midpoint identities,12exponent rescale identities
and1,728partial-pass/bound checks. Arithmetic/live-buffer proofs and frozen hashes
are saved in `jpeg2000-irreversible-native.json` and
`jpeg2000-irreversible-arithmetic-bounds.json`. That native record describes the
checkpoint before the remaining gates; the [final validation record](jpeg2000-irreversible-validation.json)
closes all selected gates against committed source
`d8c56fe5bebab9693768c6aaac4900474ce3faf3`. The final evidence commit changes only
documentation and reports; production source hashes remain identical.

## Completed runtime and artifact gates

Seed0x4952524556 adds24random geometries /144comparisons across all6levels and
Gray/RGB/ICT modes, raw/JP2, rates, blocks, nonzero origins and varied multipart
framing. All pass the same1sample oracle policy, maximum per-input MAE0.006493506494,
with0encoder failures. [Random provenance](jpeg2000-irreversible-random.json)
separates source compression loss from decoder error and retains actual geometry,
packet declarations, encoded/oracle/output hashes and metrics.

An independent debug harness exercises3,000seeded mutations over12fixtures with
bounded input/pixels/workspace/metadata. It accepts743 and rejects2,257, with
0panics and0errors after output initialization. JP2 codestream box lengths are
repaired after insert/delete mutations to reach the codestream parser. The
[corruption record](jpeg2000-irreversible-corruption.json) preserves frozen source,
harness, fixture and run-log hashes; accepted mutations are not a claim of valid
encoding or complete malformed-input coverage.

Both actual wasm32 builds (minimal and default features), run by Nodev24.19.0
with no host imports, pass1,796cases each:1,539lossless source-exact,
256irreversible oracle and1preexisting encoder-compatibility case. Each rejects
174negative fixtures before callbacks (138prior +36new). They verify every RGBA
byte/dimension, exact/short budgets, callback/Abort order, scope restoration,
truncation, framing and direct-API cumulative metadata. Both built from clean
committed d8c56fe source and match native source hashes. Final linear memory is
296,419,328bytes minimal /296,550,400default, including the embedded test corpus
and33.6MB metadata regression, not an ordinary image decode memory estimate.
[WASM provenance](jpeg2000-irreversible-wasm.json) retains module/helper/manifest
hashes and checks; the [combined archive](jpeg2000-wasm-runtime.json) preserves
earlier-stage evidence too.

Nine public API exports were displayed and inspected, covering Gray/RGB/ICT,
rates, tiny1×1level5,18unknown-total terminal parts,256tiles,1024², deep derived
quantization and differing component QCC. All match the independent oracle within
1sample. [Display sheet](jpeg2000-irreversible-display.png) and
[display hashes](jpeg2000-irreversible-display.json) retain those outputs.

Cargo verifies the unpacked minimal-feature source archive:196files,2.0MiB raw,
431.5KiB compressed (441,822bytes), SHA256
`73deeb191b46642485b790fce11276ad00bfcc672931abf36c505df313a6e999`.
The three production decoder sources are byte-identical to the tested checkout;
large JPEG2000 fixture/oracle data is excluded. Its VCS checkpoint is d8c56fe,
not the later evidence-only commit. No production dependency/default-feature/API
or WML2 0.0.35 version change is introduced.

## Paired performance

Immutable release binaries, fixed CPU affinity, B/H/H/B/H/B,3processes/version,
100warmups and no concurrent test jobs use whole-child CPU divided by calls,
including startup/export. All paired output hashes match. Recent baseline3dcd7dd
uses the same production source as its frozen pre-evidence binary; original
stable7043bf0 supplies the cumulative Gray comparison.

| Baseline | Input | CPU μs/call baseline | Current | Change |
| --- | --- | ---: | ---: | ---: |
| 3dcd7dd | Gray16 | 50.162 | 52.571 | +4.80% |
| 3dcd7dd | Gray65 | 1220.729 | 1224.631 | +0.32% |
| 3dcd7dd | RGB16 | 322.658 | 320.356 | -0.71% |
| 3dcd7dd | RGB65 | 4033.350 | 4105.326 | +1.78% |
| 3dcd7dd | Gray256tiles | 1083.610 | 1263.402 | +16.59% |
| 3dcd7dd | RGB9tiles | 137314.796 | 132177.204 | -3.74% |
| 3dcd7dd | RGB2oddtiles | 273.345 | 247.996 | -9.27% |
| 7043bf0 | Gray16 | 55.333 | 66.509 | +20.20% |
| 7043bf0 | Gray65 | 1276.869 | 1244.583 | -2.53% |
| 7043bf0 | GrayDeep65 | 1492.060 | 1480.850 | -0.75% |
| 7043bf0 | GrayDeep257 | 13787.181 | 13171.733 | -4.46% |

Two initial increases also show large within-version timing variation. Targeted
repeats retain the same affinity/order and3processes/version with3×timed iterations:
Gray256tiles changes820.420→832.494μs (+1.47%); original stable Gray16 changes
69.791→54.539μs (−21.85%). The initial +16.59%/+20.20% increases do not reproduce.
Every initial/outlier row is retained. These samples show no consistent material
regression, while shared-host variation limits their precision and prevents a
cross-host guarantee. [Recent suite](jpeg2000-irreversible-benchmark.json),
[original stable suite](jpeg2000-irreversible-stable-benchmark.json),
[Gray256 repeat](jpeg2000-irreversible-benchmark-repeat.json) and
[stable Gray16 repeat](jpeg2000-irreversible-stable-benchmark-repeat.json) retain
raw timings and source/binary/input/output hashes. Original stable cannot decode
RGB or the new irreversible inputs.

The new-path current-only reference uses3processes and100warmups on the same
fixed CPU: GrayD1rate4: 63.860μs/call; ICTD5rate2: 904.204μs/call; ICT1024rate4: 658930.886μs/call.
The previous source cannot decode these inputs, so these are future comparison
references, not a before/after performance claim. [New-path timing](jpeg2000-irreversible-new-benchmark.json)
preserves all9rows and hashes.

All selected PR review gates are complete, with no remaining blocker for this
bounded unsigned8bit profile. Independent actionable findings are resolved.


## Reproduction and remaining scope

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_irreversible.py
python3 wml2/tests/fixtures/review/generate_jpeg2000_irreversible_variants.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_irreversible_random.py /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
python3 wml2/tests/fixtures/review/benchmark_jpeg2000.py /absolute/path/to/immutable-baseline /absolute/path/to/immutable-current --baseline-git BASELINE_SHA --output .test-jpeg2000-benchmark.json
# Add --stable-gray for the original7043bf0 Gray baseline.
cargo package -p wml2 --offline --no-default-features --features jpeg2000
```

Multiple quality layers/pass continuation across layers, other progressions,
explicit precincts/SOP/EPH, tile-header overrides/interleaving/nonraster order,
ROI, ICC/palette, alpha, signed/high-depth/subsampled components remain unsupported.
High sample depth requires the [ImageBufferEx receiver plan](jpeg2000-high-depth-plan.md)
first; this stage changes no such API. Tests do not establish general full Part1
conformance, browser UI, other OS or every optional AVIF matrix. Performance is
a shared-host sample rather than a cross-host guarantee. No PR, merge or publish
is included in this task.
