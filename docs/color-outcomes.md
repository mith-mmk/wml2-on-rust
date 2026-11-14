# Typed color-processing outcomes (issue #113)

This additive API describes observed processing of decoded pixels, not ICC presence.
Existing `DataMap` variants, buffer layouts, callbacks, warnings and pixel algorithms
are unchanged. The public `color_outcome` module reads versioned `DataMap::Raw`
records through `ColorOutcome::from_metadata`. No warning text is parsed.

## Reading a result

After successful decode, use `metadata_key(output_index)` to retrieve exactly that
output's record from `ImageBuffer.metadata` (or a callback's metadata collection):

```rust
use wml2::color_outcome::{ColorOutcome, metadata_key};
let image = wml2::draw::image_from(bytes)?;
let outcome = ColorOutcome::from_metadata(
    image.metadata.as_ref().and_then(|m| m.get(&metadata_key(0)))
);
```

`conversion` distinguishes `AppliedToSrgb`, device-CMYK `Approximation(reason,
method)`, `NotApplied(reason)` and `Unknown`. `output` separately distinguishes
sRGB, preserved source/reconstruction coordinates, a device approximation and
unknown. An approximation is **not** guaranteed sRGB. Applications must not infer
conversion success from an ICC, absence of warnings, or an unreported output.
Missing records, unknown versions, malformed records and invalid state combinations
read as `Unknown`. Never inherit a previous page's record.

The v1 wire record is exactly eight bytes: `WCO`, version `1`, conversion tag,
reason tag, method tag, output-space tag. Numeric mappings are defined by the
module's explicit encoder/parser, not Rust enum discriminants. The public enum
vocabulary is non-exhaustive. The two public `ColorOutcome` fields are the stable
result layout; future wire extensions require a new version.

## Reported processing sites and output correspondence

| Route | Reporting point | Output index |
| --- | --- | --- |
| Baseline/progressive JPEG CMYK | Completed pixel output and EOI; prepared route retained separately from converter | 0 |
| TIFF generic blocked CMYK | Successful completion of all page blocks | Zero-based display-page order |
| PSD CMYK | Composite and emitted raster layers have finished conversion | 0 composite, then 1.. in emitted layer order |
| JPEG XL ordinary/EX still | Completed reconstruction, preserving reconstruction coordinates | 0 |
| JPEG XL managed still | Successful reconstruction-profile-to-sRGB transform | 0 |
| JPEG XL ordinary/EX/managed callback animation | All frame reconstruction/optional transforms completed | 0 poster, 1.. playback frames, including repeated first frame |

JPEG uses additive approximation; TIFF/PSD use multiplicative approximation.
Reasons distinguish missing ICC, invalid ICC (including malformed JPEG APP2
association), unsupported profile and feature-disabled CMS. A valid prepared ICC
route is reported applied only after successful pixel work. CMYK transform runtime
errors remain fatal, **not** a newly introduced fallback: `TransformFailed` is
reserved for a future explicit fallback producer. No current route emits it.
PSD untagged ICC is deliberately not selected as a conversion profile and reports
`ProfileIgnored`, distinct from a missing ICC.

For JXL, `JPEG XL Source ICC Profile` describes the embedded/source profile;
`JPEG XL Reconstruction ICC Profile` describes reconstructed samples before a
managed conversion; `ICC Profile` describes returned samples. The ordinary route
keeps reconstruction coordinates even when source and reconstruction differ.
The managed helper executes a transform, including when its endpoints happen to
be equivalent; it does not infer an `AlreadySrgb` shortcut.

Other codec paths, JPEG scaled decoding, TIFF low-level compression/draw wrappers,
TIFF native Gray/RGB EX, delegated JPEG-in-TIFF and native
`JxlAnimationImage` APIs do not yet produce these records. They remain unknown to
this accessor. Native JXL animation retains its existing source/output ICC fields;
this change does not add fields to that public result. Reasons such as `AlreadySrgb`,
`Disabled` and `SourcePreserved` are available to future explicitly instrumented
routes, not inferred by this implementation.

## Failure, cancellation and resource contract

Records accompany pixels; they are not independent completion certificates.
Callbacks can see completed processing followed by a later callback error/Abort.
Streaming clients must discard both pixels and records on a failed/cancelled
transaction and reset their metadata at `begin_image`. Owned managed JXL helpers
return no image on cancellation or error. A TIFF later-page recovery warning can
coexist with earlier valid pages; an unsuccessful later page has no successful
record. JPEG incomplete/malformed scans do not gain a completed record.

New record bytes/keys count against metadata budgets. JXL also includes them in
its pre-publication workspace/metadata accounting. No memory limits are relaxed.

## Validation at this change

On hadrianus (aarch64 macOS), source base `a6bdc86a`:
- Default codecs plus `psd,color-management,image-buffer-ex`: library/integration
  matrix, 481 passed and 13 intentionally ignored (before the final extra mapping
  tests; those are covered by the focused runs).
- Focused JPEG/TIFF: applied/missing/invalid/profile-disabled outcomes, progressive
  and baseline equivalence, mixed TIFF page routes, failed later-page isolation,
  and incomplete JPEG EOI behavior.
- JXL/PSD: ordinary versus managed profiles, animation poster/playback indexing,
  PSD composite/layer indexing, every cancellation probe in owned JXL decoding,
  malformed/truncated input, and existing independent pixel fixtures.
- Protocol unit tests: every reason/method round trip, missing/wrong-typed records,
  unknown versions, invalid lengths/tags and impossible output combinations.
- Minimal feature build and CMS-disabled builds preserve additive compatibility.

This is a bounded first implementation of #113, not universal color-management
coverage. Wider codec instrumentation and direct native-animation typed accessors
remain follow-up work. Do not close #113 as complete for every codec/API.

### Review follow-up

The first review found two gaps, both corrected before publication: PSD's explicit
untagged-profile flag now reports `ProfileIgnored`, and raw TIFF decoding stops
when the newly introduced outcome callback returns Abort. Regression coverage
includes raw/common TIFF and public/common JPEG, raw/common PSD, static/animated
JXL outcome callbacks, and the exact/-1-byte PSD outcome metadata budget.
After these fixes: JPEG/TIFF CMS 49 passed/1 manual ignored; PSD/JXL targeted
callback and result tests 21 passed; PSD CMS-disabled 13 passed (before adding
its two callback/quota tests); protocol reason tests 2 passed. Extended JXL
oracle/admission/cancellation suites passed 31 tests.

[Local benchmark and image validation](color-outcomes-validation-20261114.json)
records baseline/candidate pixel equality and descriptive debug timings. Six JPEG
PNG files are byte-identical to baseline; PSD/JXL pixel hashes also match. Images
were visually inspected. These shared-host measurements are not a general speed
claim, and were made before the final callback/reason-only follow-up.

## Final review corrections and rerun

Review of `31a9b2c0` identified a stale JPEG ICC exact-budget test: the
new outcome contributes 28 bytes (key plus v1 record), so the old 716-byte
expectation correctly failed at 744 bytes. The test now derives the added cost
from the public key/schema and checks both the exact budget and one byte less.
No resource limit or runtime accounting was weakened.

After this test correction, `cargo test -p wml2 --locked --lib --tests
--features 'psd,color-management,image-buffer-ex' --no-fail-fast` passed
486 tests with zero failures and 13 intentionally ignored tests on hadrianus.
The JPEG ICC target also passed all eight tests in five configurations:
no-default JPEG with IDCT llm/aan/slower, llm plus CMS, and default features
(40 executions, not 40 distinct tests). CI on the previous SHA is failed,
not evidence for this correction; the new head must pass independently.
