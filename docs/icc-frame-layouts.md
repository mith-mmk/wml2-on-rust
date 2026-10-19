# Explicit native Gray/RGB ICC output layouts

The `color-management` feature's `FrameTransform` and existing `transform_frame`
functions now support Gray→RGB and RGB→Gray through the own `icc-profile`0.0.7
engine. Source and destination ICC remain explicit caller arguments. The adapter
validates source/model agreement and accepts only Gray/RGB device models;
arbitrary3CLR does not become RGB by matching a channel count.

A model change creates full-resolution packed planar colour planes and, when
present, a separate alpha plane. The storage type stays U8/U16/F32. Colour
meaningful precision is the maximum of the source colour planes; alpha keeps
its own original meaningful precision and exact sample values. Source strides,
channel offsets and padding are respected on input. Same-model conversion
preserves its existing layouts, padding and per-plane precision.

Input and output channel spans are distinct, each bounded to256pixels. Alpha
never enters the CMS. The immutable input frame, `FrameMetadata.source_color`,
source tags, geometry metadata and timing remain unchanged. The active typed
ICC and any existing generic `ICC Profile` use destination bytes. A failed
transform publishes no partial frame.

```rust
let transform = wml2::highres::icc::FrameTransform::new(
    &source_icc, &destination_icc, Default::default(),
)?;
let converted = transform.transform_with_output_limit(&frame, 16 * 1024 * 1024)?;
```

The optional cap covers output sample-buffer bytes, including preserved padding
for same-model output. Model-changing extents use checked arithmetic and
fallible data allocation before conversion. Metadata clones are outside this
sample cap; profile parsing/compilation keeps the own engine's separate limits.
The existing `transform` method uses usize::MAX as its sample cap. This does not
change decoder `DecodeLimits` or silently impose a new native-frame size limit.

Premultiplied alpha, limited-range samples, subsampled colour and unreconstructed
YCbCr still require explicit preparation before ICC. This is SDR colour-model
conversion, not tone mapping or a CMYK/native4D-CLUT implementation. No new
runtime dependency or duplicated ICC/CMS engine is added.

## Independent validation

Four new integration regressions cover both directions, U8/U16/F32,8/10/12/16
meaningful integer precision,513-pixel chunk and row boundaries, unusual role
ordering, padding, mixed colour/alpha precision, source immutability, exact alpha,
metadata/timing identity, exact output-byte boundaries, invalid coordinates and
non-RGB three-colourant models. Seventy-five frozen LittleCMS2.16 relative-intent
NOOPTIMIZE float reference rows are generated independently by
`generate_icc_cross_model_references.py`; integer output is quantized last.
In the CSV,bits32 denotes normalized F32, not a32-bit integer storage type.

The focused ICC/EX targets pass17tests with1existing benchmark ignored. Counts
include6existing frame tests,4new layout tests, the published Lab-PCS LUT control
and6EX metadata bridge tests. Scoped Clippy and formatting pass with existing
unrelated diagnostics only. Two native image patterns independently checked by
LittleCMS16 differ by at most1/65535 and have exactly equal final RGBA8 pixels;
alpha is exact. Their paired native-coordinate diagnostic rasters were displayed
without corruption.

## Local existing-path performance

Both release libraries are linked into one executable and alternate call order
each iteration onCPU0,3processes per case, with no concurrent builds/tests.
The old source is identical to b792e4a/15e26b8. A metadata-only crate disambiguator
allows both versions to link; no baseline source is edited. Small16×16 cases use
1100pairs/first100omitted (3000samples per variant);256×256 cases use40/first5
(105samples per variant). All existing-path output hashes match.

| Existing route | Before | After | Median paired change | Allocations before/after |
| --- | ---: | ---: | ---: | --- |
| gray16 | 210.67µs | 211.58µs | +0.28% | 12 / 12 |
| rgb16 | 624.40µs | 626.52µs | +0.36% | 20 / 20 |
| gray256 | 71153.57µs | 70760.04µs | -0.31% | 12 / 12 |
| rgb256 | 215017.22µs | 215285.88µs | +0.28% | 20 / 20 |

New model-changing routes have no previously successful adapter counterpart;
these local measurements do not claim a speedup or general throughput bound.
Detailed byte counts, paired CSVs and image/reference evidence are local artifacts.
