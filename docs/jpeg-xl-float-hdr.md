# F16/F32 encoded HDR preservation

This unmerged #46 implementation builds on #97
(`052e30262f4bf84952986e95482acc0123d4b681`), production through #95.
#96's experimental codec patch remains isolated. No product dependency, public
API signature, fixed cap, version, merge or release changes.

## Selected scope and ordering

The remaining heap/preflight dependency integration (#37/#40) needs public APIs
or a distribution decision; HDR display needs white/exposure/tone/gamut and HLG
display conditions. Neither decision is needed to preserve already encoded
normalized float HDR values. That is the selected independent #46 stage.
#42 has a finite acceptance/refusal audit, not an instruction to implement every
combination; #47's production incremental admission remains a larger independent
scope; #48/#49 retain their backend/resource blockers. Existing evaluations were
read rather than rerun as new product support.

## Public behavior and precision

`image_hdr_encoded_from_with_resource_limits_and_cancel` retains integer16
acceptance and now accepts original-profile Modular **F16/5 or F32/8** PQ/HLG
Rec.2020/D65 RGB, optionally one matching-depth full-resolution straight Alpha.
Source/output structured profile and bit-depth projections are checked against
the bounded independent preflight. Float input requires Modular frames before
pixel allocation. Non-HDR/native SDR and ordinary/EX/sRGB acceptance is unchanged.

The existing `JxlEncodedHdrImage` returns F32 encoded RGB coordinates. F16 source
values are promoted exactly; F32 storage is not a declaration of source precision.
The carrier does not expose source depth or fabricate ICC bytes. RGB and Alpha
must be finite in0..=1; negative zero is retained. Negative/above-one/nonfinite
RGB, negative/nonfinite/out-of-range Alpha, associated/subsampled/mismatched Alpha,
embedded ICC, custom primaries and other extras remain explicit errors. This
finite encoded-HDR subset differs from native SDR float's extended-range RGB.
There is no silent clamp, normalization by the source intensity hint, EOTF/OOTF,
CMS, tone mapping or SDR output. Alpha never passes through a color transform.

The existing explicit linearization consumes this carrier if requested: PQ uses
absolute nits and HLG uses relative scene units. It still does not select HLG
system gamma/display conditions or HDR-to-SDR policy. RGB subnormals survive
**encoded** decoding; subsequent transfer math may underflow by its documented
F32 output contract.

All old preflight/input/header/layer/padding/TOC/ICC/Main5 guards remain. F32 output
bytes and signal metadata share the same cumulative workspace allowance, and a
fallible output reserve happens before publication. No new image copy/scratch is
introduced. Fifteen pixels need240 output bytes;239 is refused. Error/cancellation
returns no image. This remains conservative codec admission plus adapter accounting,
not codec-wide fallible allocation, peak RSS coverage or large-image enablement.

## Independent evidence and validation

The [oracle manifest](jpeg-xl-float-hdr-oracle.json) pins C helper/generator,
input hashes, authored-source hashes, actual C metadata and RGBA hashes. C
libjxl0.11.2 independently encodes and decodes eight cases: PQ/HLG ×F16/F32 ×RGB/
RGBA, orientations1/6, transparent/partial/opaque Alpha, signed zero, minimum
subnormal and minimum normal. C ORIGINAL and DATA structured encodings agree;
kept-orientation C bytes exactly equal authored values. Product output exactly
matches C's independently oriented F32 bytes, including Alpha and sign bits.

Nine additional C-readable inputs exercise negative/above-one RGB, NaN/Infinity,
negative/NaN Alpha, F16 NaN, associated Alpha and P3. WML2 rejects each according
to its narrower contract. Every proper prefix of every positive file is rejected.
Tests retain old native/ordinary refusal behavior, output/resource boundaries,
pre-cancel precedence and each observed cancellation checkpoint for all four
RGBA float cases, with warmed scoped owned-byte cleanup. A test-only hook injects
actual allocator-null failure at the240-byte fallible output request for F16 and
F32 and verifies typed failure plus recovery. This is not global OOM recovery.

[Local validation](jpeg-xl-float-hdr-validation.json) records Linux x86_64 /
Rust1.91: minimal50 tests, expanded133 tests (ordinary, resources, native F16/F32,
encoded HDR, linearization and library), disabled target0 tests. Actual wasm32 /
Node24.19 passes101 checks with31 combined cancellation probes, no host imports
and enforced32 MiB linear-memory ceiling. Native32-bit execution and local
Windows/macOS were not run; existing CI platform jobs include the changed test
file, and the existing HDR WASM job replays the extended harness. CI completion
is not awaited.

A magnified encoded-coordinate/Alpha checkerboard diagnostic of the oriented
float result and C oracle was inspected and matches. It is a diagnostic of
stored code values, not a product HDR display or tone mapper.

Release compatibility benchmarks use identical features/graphs/harnesses,
three alternating paired blocks of100 timed decodes and three warmups. Initial
integer-HDR/ordinary/managed median ratios are1.024/0.993/0.866; the managed
baseline is visibly noisy. A bounded six-pair1000-sample follow-up onCPU0 gives
integer-HDR ratio0.997 with overlapping block ranges. No repeatable integer-HDR
slowdown is established here; neither a speedup nor general throughput guarantee
is claimed. All hashes match. No accepted-before float-HDR benchmark exists.

```sh
cc -O2 -Wall -Wextra -Werror -I <libjxl-0.11.2>/lib/include \
  wml2-test/scripts/jpegxl_hdr_encoded_reference.c \
  -Wl,-l:libjxl.so.0.11 -o .test-oracle/reference
python3 -B wml2-test/scripts/generate_jpegxl_float_hdr.py \
  --reference .test-oracle/reference --work .test-oracle
cargo test -p wml2 --locked --no-default-features --features jpeg-xl --lib \
  --test jpeg_xl_encoded_hdr
python3 -B wml2-test/scripts/verify_jpegxl_wasm.py \
  --hdr-encoded --output .test-float-hdr-wasm
```

#46 remains open for display policy and unmerged implementation. No new
completion conditions, dependency fork, upstream submission, approval or release
are introduced.
