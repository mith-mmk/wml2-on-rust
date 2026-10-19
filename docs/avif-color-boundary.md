# AVIF still-image ICC and alpha boundary

The corrected ordinary still path uses the complete associated colour set from
the pinned `avif-rust`0.1.0 native parser. An associated ICC takes priority over
nclx primaries/transfer regardless of property order; matrix/range
reconstruction remains a codec operation. This follows the
[libavif colour interpretation guide](https://github.com/AOMediaCodec/libavif/wiki/CICP).

The order is native AV1 decode, matrix/range reconstruction into encoded RGB16,
alpha unassociation when required, one `icc-profile`0.0.7 transform to the
fixed sRGB destination, then RGBA8 quantization. The dependency's legacy
`to_rgba16` is not used for this ICC path because it applies display transfer
before its independent ICC engine. `avif` now enables the already declared
own ICC dependency; no additional third-party runtime dependency is introduced.

The transform uses separate1/3-channel source and3-channel destination spans,
fixed256-pixel scratch arrays, and checked profile/compiled-memory limits.
Alpha never enters the CMS. Gray ICC is applied only to a monochrome source.
The ordinary result retains original `Source ICC Profile` and the destination
`ICC Profile`, plus original AV1/nclx signaling and bit depth. Source metadata
does not become a declaration that output samples still use that source ICC.
Native/EX preservation of opaque unsupported ICC is independent of transform
compilation; the display route reports unsupported profiles before drawing.

The588-byte destination is a frozen LittleCMS-generated standard sRGB profile,
with copyright tag `No copyright, use freely`, SHA256
`43c1bc2be9fb4cf72e27ab12f64e54fad4478cf80a02ee79dc7df37957c79c96`.
LittleCMS is reference tooling only; runtime transforms use the own library.

## Actual alpha association

The AVIF specification places [`prem` under `meta/iref`](https://aomediacodec.github.io/av1-avif/v1.2.0.html#additional-boxes),
not in `ipco`. A bounded bin-rs box walk resolves the directional reference
from the primary item to its decoded alpha item, including iref versions0/1.
Unrelated references do not label the primary, and wrong/duplicate targets or
truncated extents fail. Nonstandard `ipco/prem` does not substitute for it.

Typed native planes are unchanged and now carry the actual
`AlphaAssociation::Premultiplied`. EX requests straight normalized RGBA16, so
it unassociates encoded RGB before any colour transform, and records
`Source AVIF Alpha Premultiplied` as provenance. At alpha zero RGB becomes zero.

## EX source coordinates

EX now suppresses the converter's display-transfer/gamut stage while retaining
the original signaling. It does not apply ICC, display transfer or tone mapping.
This admits source transfer codes such as linear/PQ when the codec supports
their matrix reconstruction, without claiming rendered HDR output. The pinned
converter also omits video-range expansion for monochrome; the adapter corrects
that before a Gray ICC can be applied. Native limited-range planes remain
unchanged, with their original range metadata.

Existing EX still-profile restrictions on AVIS, geometry and unsupported
derived layouts remain. Ordinary AVIS and unprofiled/unassociated stills keep
their legacy callback/timing route. Managed AVIS, ICC/associated-alpha geometry,
and associated-alpha display transfer without a source ICC remain distinct
work; the corrected owned still route explicitly rejects the latter two.
No general HDR renderer, tone mapper or animation CMS is claimed here.

## Validation references

`avif_color_boundary.rs` checks9independent libavif/libaom/ffmpeg inputs,
RGB/Gray,8/10/12-bit linear signaling, actual prem association, and video-range
Gray. Reference RGB is reconstructed as16-bit by libavif1.2.1, transformed by
LittleCMS2.16 with relative intent, and quantized to8-bit last. This avoids
amplifying an earlier8-bit rounding step through the ICC curve. References
contain no WML2-generated pixels or WML2-encoded AVIF input.

Provenance/hashes are in `wml2/tests/fixtures/review/avif-icc-origins.json`.
Source range and ICC identity, both property orders, alpha, all samples,
metadata limits, Abort and callback errors are covered. The ordinary output
is within1/255 of the independent end-to-end reference; native source planes
remain exact. The filter-disabled control is frozen alongside the tests so
optional-AVIF regression no longer needs a generated submodule working file.

Reference regeneration (development tools, not required to run Rust tests):

```console
python3 wml2-test/scripts/generate_avif_color_references.py
```

The default verifies frozen references. `--write` refreshes them. Matching
headers and a Unix soname can be selected with `--avif-header` and
`--avif-library`. The reference C helper additionally regenerates the three
limited Gray inputs with `--gray-limited <bits> <gray.icc> <output.avif>`.
Original RGB/Gray source patterns use libavif or ffmpeg/libaom; colr-only ICC
insertion leaves the encoded AV1 payload unchanged and updates ancestor/iloc
offsets. All generated inputs and profile assets are project-owned test data.

## Local validation checkpoint (2026-10-19)

The scoped AVIF targets pass62tests with1existing ignored; AVIF-only passes29tests.
The default+EX+CMS core passes364tests with6existing ignored. Counts overlap.
Formatting and ordinary scoped Clippy pass with existing unrelated warnings only.
All9managed RGBA8 outputs exactly match the independent references in this run;
source-coordinate RGBA16 maximum difference is2/65535. Native plane identity,
source/destination profile bytes, callback Abort/errors and quota failures pass.
The reference and result rasters were displayed side by side without corruption.

Release benchmarks compare b0065c3d (AVIF source identical to b792e4a) with this change.
Each case uses3alternating process pairs pinned toCPU0,1100iterations per process,
first100omitted, with no concurrent builds/tests. These tiny16/32-pixel-side
fixtures measure setup as well as decode; they are not throughput guarantees.

| Case | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| unprofiled | 22.82µs | 23.41µs | +2.55% |
| rgb-icc-nclx | 428.10µs | 4023.19µs | +839.79% |
| rgb-icc-prem | 383.54µs | 3925.05µs | +923.38% |
| gray-icc-limited12 | 77.90µs | 801.02µs | +928.30% |
| ex-unprofiled | 23.46µs | 24.07µs | +2.60% |

Unprofiled control hashes remain identical. ICC-case hashes intentionally change
because the former display-transfer/ICC order and source-profile selection were
incorrect; the corrected outputs match the independent oracle. The own-library
matrix/TRC transform has a substantial per-pixel cost, especially its bounded
inverse parametric curves. Using its f32 API once on private scratch avoids the
integer API's extra validation transform, roughly halving the initial corrected
cost. The remaining ICC slowdown is measured and retained as an explicit performance
limitation; no claim of unchanged ICC-path speed is made. Further optimization
belongs in the own ICC engine rather than a second conversion implementation.
