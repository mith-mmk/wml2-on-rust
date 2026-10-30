# JPEG XL float/HDR output and tone-mapping boundaries (#46)

Assessment: 2026-10-30, main `6a48db924b338c5ae7095bea6105fb4dbd436e95`,
published `jxl = 0.7.4`, dependency defaults disabled. No production code,
manifest, API, acceptance boundary or resource cap changes are made here.
This is a bounded feasibility/evidence result, not completed HDR support.

## Decision

Do not remove the float/PQ/HLG/tone-metadata refusals. A useful F32 preservation
path exists in the pinned core for the tested original-profile Modular files,
but it does not fit the integer EX callback contract. Typed highres F32 storage
alone does not supply a JXL decoder, source/sample-domain metadata, an
extended-range CMS or an explicit display operator. Additive native output and
SDR rendering need separate reviewed APIs and admission work before integration.

Direct F16 output is not precision-qualified: the minimum half subnormal is
flushed in these probes. `desired_intensity_target` is declared/defaulted but
has no consumer in the inspected pinned core; setting it to 80 changes none of
the 45 paired core outputs. It cannot establish a tone-mapping contract.

## Distinct contracts

| Concept | Current state | Required interpretation |
| --- | --- | --- |
| Source sample encoding | JXL header records integer or floating-point depth/exponent bits; WML2 preflight refuses float | A float source may be SDR; integer PQ/HLG can be HDR. Storage type does not determine transfer function |
| F32 output storage | Core supports F32/F16 output; highres `PixelBuffer::F32` exists | F32 can hold encoded PQ, encoded HLG or linear values. Never relabel it linear merely because it is float |
| Source and output profiles | Original-profile Modular core output retains its source profile in this experiment | XYB can select another reconstruction profile depending on output format; source identity and output coordinates must be separate |
| EX / legacy output | EX exposes checked integer Gray/RGBA byte formats; legacy is RGBA8 | Integer range scaling and rounding are not tone mapping; no float-to-EX or float-to-RGBA8 fallback |
| ICC CMS | highres keeps alpha outside CMS; current engine defaults to clamping normalized device input | F32 CMS is not extended-range/HDR processing. `clamp=false` rejects out-of-range device samples rather than preserving them |
| Tone mapping | No explicit WML2 HDR display API or operator contract | Transfer decoding, luminance scaling, gamut conversion, dynamic-range compression and output encoding are separate operations |

PQ has absolute-luminance semantics; HLG needs a declared scene/display and
viewing/system-gamma interpretation. Preserve source transfer/primaries/white
point and JXL intensity target, minimum luminance, relative-to-max-display and
linear-below signaling. These metadata fields are not commands to silently
render to a particular SDR display. [ITU-R BT.2100](https://www.itu.int/rec/r-rec-bt.2100)
provides the signal-domain context; no new transfer/tone operator is implemented.

## Reproducible executed evidence

```sh
python3 wml2-test/scripts/evaluate_jxl_hdr.py --output .test-hdr
```

Requires Linux, Rust 1.91+, `cc` and installed `libjxl.so.0.11`. Tested with
Debian `libjxl0.11 0.11.2-0.1~deb13u2`; the oracle reports version 11002.
The script downloads only official API headers at libjxl v0.11.2 commit
`332feb17d17311c748445f7ee75c4fb55cc38530`, records hashes and uses local
build-generated export/version-header stubs without redefining ABI structs.
The C helper creates original synthetic CC0 5x3 lossless fixtures with libjxl,
then independently decodes them. No candidate self-oracle is used. The isolated
Cargo workspace pins 0.7.4, records/reuses its lockfile and leaves shipping Cargo
files untouched. It calls the core directly only on trusted generated fixtures;
this intentionally bypasses production preflight for research, not general input.

[Execution record](jpeg-xl-hdr-float-evaluation.json) contains lockfile, header
hashes, fixture/output hashes, errors, profile checks, ranges and every comparison.
Generated source/oracle/core buffers remain in `.test-hdr`. Each native child has
512 MiB address-space/20-second CPU containment and a 120-second wall timeout;
these are research containment, not proven backend resource limits.

15 fixtures x F32/F16/U16 x default/80-nit requested target = **90 comparisons**.
72 are byte-exact. The 18 differences are retained as findings, not counted as
successful HDR support:

| Fixture set | Independent result | Scope/limitation |
| --- | --- | --- |
| Float16/Float32 Gray/RGB/RGBA linear, six files | All 12 F32 results match libjxl exactly; source float32 also matches original bytes | Negative values, signed zero, values >1 up to 65504, half minimum subnormal, float32 subnormals, and straight alpha tested |
| Same six files, F16 output | All 12 differ at the half minimum subnormal, error `2^-24` | Core converts that value to zero; F32 output retains it. No F16-native preservation promise |
| Integer16 Rec.2020 PQ/HLG RGB/RGBA, four files | All 12 default-target format results match libjxl; normalized code values and profiles preserved | Encoded sample preservation is not linear-light or display rendering |
| Structured PQ, requested target 80 | Six core/reference differences; F32 maximum absolute RGB difference ~0.717609, U16 up to 47028 | Core ignores requested target; libjxl changes its encoded RGB. Alpha remains identical. This reference setting alone is not an sRGB tone-map oracle |
| Structured HLG, requested target 80 | Both cores leave these source-coordinate outputs unchanged | Does not establish HLG-to-SDR correctness or define an OOTF/viewing condition |
| Float32 gamma-2 ICC; integer PQ/HLG ICC, three files | Source and output ICC bytes match original exactly in both decoders, all 18 format/target results match | Embedded ICC identity is not color conversion; arbitrary-ICC target requests here do not change samples |
| SDR sRGB with non-default intensity metadata; ordinary SDR control | Core/libjxl comparisons exact | WML2 refuses non-default tone metadata; accepts ordinary SDR control |

All 14 non-control files are rejected by each current WML2 entry point: ordinary,
EX RGBA16 and explicit sRGB display. The two ICC CICP PQ/HLG inputs specifically
return `ICC PQ/HLG requires explicit HDR processing`; their ICC comes from
libjxl's structured-profile generation, not fabricated contradictory metadata.
The SDR control succeeds through all three routes. No refusal is relaxed.

The typed-storage/CMS probe additionally executes these assertions:

- `[-0.25, 0.5, 1.5, 0.25]` is valid F32 straight RGBA storage.
- NaN, positive/negative infinity and alpha below 0 or above 1 are rejected.
- Existing same-profile highres CMS changes that pixel to
  `[0.0, 0.5, 1.0, 0.25]` with default options; alpha is unchanged.
- `clamp=false` returns an out-of-normalized-domain error. This verifies that
  toggling the option is not an extended-range preservation solution.

The production regression command was also run:

```sh
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,image-buffer-ex,color-management \
  --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers \
  --test highres_typed --test highres_icc
```

Result: **44 passed, 0 failed, 1 existing ignored benchmark**. This includes
integer low bits, orientation/ICC identity, callback/refusal limits, internal
layers, typed validation and CMS alpha behavior. Runtime diff against main is
empty, so no decoder-image-display or performance-regression claim is made.

## Concrete acceptance gates for future APIs

1. **Native F32 preservation first.** Use a distinct opt-in operation/result,
   with F32 Gray/RGB/straight RGBA, sample-unit typed strides and checked byte
   accounting. Keep source depth/exponent bits separate from F32 storage depth.
   Return an explicit sample domain (encoded transfer or linear light), source
   profile and actual reconstruction/output profile, primaries/white point,
   orientation and all luminance metadata. Source ICC and source-header copies
   must not masquerade as the output profile. Do not change common EX callbacks
   to integer fallbacks or duplicate #42's mixed-alpha work.
2. **Precision and finite policy.** Preserve finite negative/>1 samples with no
   clipping, transfer application or CMS on the native route. Reject non-finite
   decoded active samples before publishing a frame; enforce straight-alpha
   `[0,1]` independently, never applying transfer/tone operators to alpha.
   Include color under alpha zero. Start with F32 output, defer F16 until its
   subnormal/rounding issue is resolved and independently retested. Define error
   budgets for lossy/XYB samples independently of lossless byte identity.
3. **Explicit SDR rendering.** Require an operator identifier/version, source
   signal domain and luminance units, source/target peak/black/diffuse-white,
   HLG system gamma/viewing assumptions if applicable, gamut mapping and target
   transfer/profile. Reject absent/unsupported choices instead of applying a
   default clip. The stages must be separately testable: transfer decoding,
   luminance normalization, gamut/CMS processing, tone operator and final
   quantization. The present normalized-domain CMS needs an explicitly reviewed
   extended-range route or strict admission; copying F32 into it is insufficient.
4. **Output/workspace admission before allocation.** Retain the current 64 MiB
   input, 4,194,304-pixel and 16,384-axis ceilings, plus caller limits and #37.
   Checked output bytes are `pixels * channels * 4` for packed F32 (Gray 4,
   RGB 12, RGBA 16 bytes/pixel), distinct from an optional explicitly requested
   SDR result. Budget row padding, separate planes, simultaneously retained
   source/output buffers, profiles and transform scratch. Current integer canvas
   accounting cannot be reused unchanged: RGBA F32 doubles RGBA16 output bytes.
   All allocations and internal-layer/ICC work must fit the existing expanded
   budget; do not enlarge caps to make fixtures pass. Preserve #31 cancellation
   and prove operator work can stop, including failure before callback init.
5. **Independent adoption matrix.** Add original-profile and lossy/XYB fixtures
   across depths/exponents, negative/overrange/tiny values, all non-finite policies,
   PQ/HLG with structured/ICC signaling, alpha and arbitrary profiles. Verify
   source and output profile identity separately. Validate native sample
   preservation separately from explicitly selected SDR tone-map output using
   known luminance/neutral/primary ramps and an independently specified operator;
   core agreement alone is insufficient. Add layout/overflow/caps/cancellation
   tests and Linux/macOS/Windows plus actual WASM execution before advertising.

No API signatures or production subset are finalized here. Arbitrary lossy XYB,
premultiplied/mixed-depth float alpha, non-finite encoded bitstreams, tone-map
quality, output/workspace peaks and cross-platform/WASM execution are unverified
by this small experiment. #40 migration, #42 extra channels and #37 resource
policy remain independent work. PR #60/#61 are unchanged.

Primary implementation evidence: [pinned core source](https://docs.rs/crate/jxl/0.7.4/source/src/),
[WML2 preflight/output](../wml2/src/jpegxl.rs),
[typed F32 validation](../wml2/src/highres/types.rs),
[highres CMS](../wml2/src/highres/icc.rs),
[EX contract](image-buffer-ex.md),
[ICC engine normalized-domain implementation](https://docs.rs/crate/icc-profile/0.0.7/source/src/transform/compile.rs),
[libjxl decoder declarations](https://github.com/libjxl/libjxl/blob/332feb17d17311c748445f7ee75c4fb55cc38530/lib/include/jxl/decode.h).
