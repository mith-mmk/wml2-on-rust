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
The first214public PPM exports pass maxdifference1, maximum per-input MAE0.00433.

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
`jpeg2000-irreversible-arithmetic-bounds.json`. Final seeded random/native
source-export verification, actual minimal/default WASM runs, display, paired
performance and cargo archive verification are pending. Results and immutable-source provenance will be appended when complete.

## Reproduction and remaining scope

```sh
python3 wml2/tests/fixtures/review/generate_jpeg2000_irreversible.py
python3 wml2/tests/fixtures/review/generate_jpeg2000_irreversible_variants.py
cargo test --workspace --release --features wml2/jpeg2000
cargo build -p wml2 --release --no-default-features --features jpeg2000 --example jpeg2000_verify
python3 wml2/tests/fixtures/review/check_jpeg2000_irreversible_random.py /absolute/path/to/jpeg2000_verify --cases 24
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

Multiple quality layers/pass continuation across layers, other progressions,
explicit precincts/SOP/EPH, tile-header overrides/interleaving/nonraster order,
ROI, ICC/palette, alpha, signed/high-depth/subsampled components remain unsupported.
High sample depth requires the [ImageBufferEx receiver plan](jpeg2000-high-depth-plan.md)
first; this stage changes no such API. Tests do not establish general full Part1
conformance, browser UI, other OS or every optional AVIF matrix. Performance is
a shared-host sample rather than a cross-host guarantee. No PR, merge or publish
is included in this task.
