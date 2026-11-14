# CMYK integration validation and ICC 0.1.0 feedback

## Scope and provenance

This branch is a validation snapshot, not a release or merge. The order remains:
finish ICC 0.1.0 first, then update WML2's ICC dependency and validate its published
package. Cargo manifests and the committed lockfile remain unchanged here.

Base: WML2 `9a46e05d82792033db09507b58a9bf2c6fdead80`.
The CMYK implementation was inherited as uncommitted work from
`feat/cmyk-core-20261114`. Its original checkout was not modified. A separate copy
was reviewed and tested, then saved on `feat/cmyk-core-validated-20261114`.
This follow-up adds strict backend-aware tests, opt-in real-image/stage benchmarks,
formatting, and this report. It does not modify ICC itself.

ICC candidate: `5d145f60365c731b18c39469a182825f4d4808cd`, whose manifest still
identifies it as 0.0.8. It was selected only with command-line Cargo patch settings.
The registry 0.0.8 does not support CMYK transforms: warning plus the existing
codec-specific approximation is the intended behavior, not proof of ICC conversion.

Host: macOS arm64, rustc/cargo 1.97.0. These are not MSRV or cross-platform results.

## Verification

- Published ICC and candidate: `cargo test -p wml2 --lib --tests --features
  'color-management psd' --offline --no-fail-fast`, 373 passed each, zero failures.
  The published run had seven ignored tests; the candidate run had eight because
  the opt-in external-image target was added between runs. A second stage-timing
  opt-in test was added afterward. Optional codecs not enabled by this feature
  set are outside this full-test result.
- After formatting, each backend passed 128 focused lib/JPEG/PSD/TIFF tests,
  with the one manual JPEG visual benchmark ignored in the regular invocation.
- Color-management disabled: 74 lib + 4 JPEG + 11 PSD + 27 TIFF tests passed.
- Candidate with multithread enabled: 80 lib + 6 CMYK JPEG + 4 JPEG restart +
  9 scaled JPEG tests passed, including worker-error handling.
- `cargo check -p wml2 --no-default-features --offline` passed.
- Manual JPEG visual/benchmark test was explicitly run successfully. Real-image
  decode and timing tests were explicitly run successfully, not counted above.
- Layout errors, wrong device models, profile parse/compiled resource rejection,
  separate alpha, native 16-bit values, TIFF 6/8/16/32-bit samples, byte orders,
  invalid APP2 associations, Adobe and unmarked input, YCCK, progressive scans,
  and no-profile arithmetic are covered by the focused tests.

Previously, five new tests unconditionally assumed CMYK transform availability,
so they failed against the actual published dependency. They now assert fallback
pixels/warnings on the known published-backend rejection, and real conversion
on the candidate. Set `WML2_TEST_CMYK_ICC=fallback` or `transform` to pin the expected
backend; an unexpected capability or any other preparation error fails the test.
This avoids silently accepting a candidate regression as a fallback.

## Real images and compatibility

Five local inputs were decoded to PNG in baseline, registry, and candidate runs:
two real CMYK JPEGs (256x256 and 500x333), a 157x151 CMYK TIFF, a 640x426 real PSD,
and a 625x1 CMYK TIFF containing a real SWOP mft2 profile. The real PSD checks
ordinary PSD display; its CMYK-specific coverage comes from synthetic 8/16-bit
fixtures. Source images/profiles are not added to the repository.

JPEG/TIFF/PSD output PNGs were visually inspected. Four published-backend outputs
were byte-identical PNGs to the original WML2. The 256x256 CMYK text JPEG differs:
the inherited implementation corrects MCU component-block addressing, removing
visible broken edges in the old decode. The corrected text layout also agrees
with independent display of the original JPEG. Therefore compatibility means
preserved arithmetic/metadata contracts, not blanket byte identity for existing
incorrect block addressing. The nonuniform synthetic block tests cover this fix.

Candidate managed JPEG and SWOP TIFF decode without CMYK fallback warnings.
Synthetic analytic RGB expectations and direct transform comparisons validate
normalization/precision. Real-image appearance is not a comprehensive colorimetric
oracle or coverage of all ICC profile kinds, rendering intents, or BPC settings.

## Release-build timing and ICC feedback

Each image timing is the median of 11 decodes, including preparation but excluding
PNG encoding. Times are microseconds; one host and run, not a CI performance gate.

| Input | Original WML2 | New WML2 + registry fallback | New WML2 + candidate |
| --- | ---: | ---: | ---: |
| CMYK text JPEG 256x256 | 1805 | 1935 | 1912 |
| ICC JPEG 500x333 | 5349 | 5577 | 606373 |
| CMYK TIFF 157x151 | 2155 | 2077 | 2107 |
| ordinary PSD 640x426 | 2038 | 1991 | 1969 |
| SWOP TIFF 625x1 | 56 | 63 | 1160 |

Fallback paths stay close at this scale (text JPEG +7%, tagged JPEG +4%; not a
statistically controlled regression verdict). New managed conversion is a different
operation, but its ~606ms cost on a small image is a material viewer-readiness concern.

A seven-run median diagnostic separated stages for the 557168-byte profile from
the 500x333 JPEG, using 166500 varied CMYK samples and the same destination profile:

- Profile parse: 17 us. Transform construction: 59 us.
- `transform_u8`, whole input: 596613 us; 256-pixel calls: 596731 us.
- `transform_f32`, whole input: 298232 us; 256-pixel calls: 298269 us.
- Same destination matrix/white, but TRCs replaced with identity **for diagnosis
  only**: f32 14461 us. This changes output semantics and is NOT an optimization
  applied to rendering.
- Reported transform memory: 558676 profile bytes + 83520 compiled bytes = 642196
  resident bytes. This is ICC's accounting, not measured process peak RSS.

Observed source explains the results: `src/transform/worker.rs` performs integer
conversion twice to preserve output-on-error semantics. `src/transform/curve.rs`
implements inverse parametric curves with 32 iterations for each of two bounds;
WML2's sRGB profile contains three type-3 parametric TRCs. The identity diagnostic
isolates these output curves as the dominant cost, while preparation and chunk
boundaries are negligible. This is stronger evidence than blaming 4D interpolation
or repeated profile compilation.

Recommended ICC 0.1.0 preparation work:

1. Prioritize a faster inverse-parametric path, preserving all accepted curve
   domains, increasing/decreasing direction, plateau policy, endpoints, precision,
   and errors. Analytic inversion for validated supported functions or a compiled
   inverse representation are candidates, not fixes proven by this report.
2. Preserve the integer API's atomic-output contract. A separate one-pass API or
   caller-owned private scratch could avoid duplicate evaluation where appropriate;
   do not silently weaken the existing API. WML2 already has private chunk scratch,
   so a later adapter optimization may use f32 with equivalent finite checks and
   final quantization after correctness/performance verification.
3. Re-run the stage probe, real-image benchmark, analytic CMYK tests, and independent
   CMS comparisons after ICC changes. No ICC correctness failure was demonstrated
   here, but real profile/intent coverage remains limited.
4. Once ICC 0.1.0 is actually available, update the WML2 requirement and lockfile,
   verify Cargo selects the published 0.1.0 artifact, and rerun both regression and
   managed-conversion checks. A 0.0.8 requirement will not select 0.1.0 automatically.

## Reproduction

Use separate target directories for the original WML2 comparison and this checkout.
Do not share Cargo build artifacts between distinct same-name/version workspaces.
Use a fresh or verified clean candidate checkout at the exact SHA above.

```sh
WML2_TEST_CMYK_ICC=fallback CARGO_TARGET_DIR=.test-target \
  cargo test -p wml2 --lib --tests --features 'color-management psd' --offline
WML2_TEST_CMYK_ICC=transform CARGO_TARGET_DIR=.test-target \
  cargo test -p wml2 --lib --tests --features 'color-management psd' --offline \
  --config 'patch.crates-io.icc-profile.path="/absolute/path/to/.test-icc"'
```

The ignored `cmyk_external_validation` tests use explicit environment inputs:
`WML2_CMYK_SAMPLE_FILES` (semicolon-separated paths), `WML2_CMYK_OUTPUT_DIR`
(use a `.test*` folder), and optional `WML2_CMYK_BENCH_REPEATS=11`.
For stage profiling set `WML2_CMYK_PROFILE_JPEG` to a JPEG with a supported embedded
CMYK profile, select the candidate via patch, and run `--release` with
`cmyk_profile_stage_timings -- --ignored --nocapture`.

## Remaining limits

No package publication, merge, dependency upgrade, ICC source edit, GUI integration,
Windows/Linux/wasm run, MSRV run, exhaustive fuzzing, injected allocation failure,
or complete peak-memory measurement was performed. Full optional-codec and all-ICC
intent matrices remain future checks. JPEG retains its older generic plane-4
warning even when managed conversion succeeds. Unneeded build directories and
comparison source copies are moved to Trash; small local logs and visual outputs
are retained for review, not committed as third-party fixtures.
