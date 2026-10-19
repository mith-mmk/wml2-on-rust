# Bounded JPEG XL internal layers

The pinned `jxl0.7.4` coalescing renderer already supports cropped layers,
reference storage and alpha blending. The adapter formerly set `sample_limit`
from the displayed canvas area. This confused an output quota with the size
of internal reference layers and rejected valid still images before rendering.

The fixed official conformance input `sunset_logo` is218bytes, from revision
`b1d0f990b03e57bf6d137c365cd5dc8b470b9191`, SHA256
`6617480923e1fdef555e165a1e7df9ca648068dd0bdbc41a22c0e4213392d834`.
Its unoriented canvas is1386×924, coded colour/alpha depth10, orientation7.
Two2048×1024 regular layers have origin(-662,-100). The first is invisible,
Replace, saved to reference1. The second uses Blend with reference1 and is the
single visible output. The codec clips and blends in canvas coordinates and
then applies orientation, producing924×1386. The adapter does not crop each
layer independently, discard reference pixels, resize the canvas or apply
orientation a second time.

## Admission before pixel decoding

A header/TOC pass uses the pinned codec's public parsers, with no second pixel,
crop or blend implementation. It walks checked TOC extents to every internal
frame, including invisible reference and LF frames. Frame headers/TOCs have
64KiB slices,4096TOC-entry caps and a64internal-layer cap independent of the
one-visible-frame quota. Internal dimensions, padded/upsampled storage and
signed rectangle extents are checked; the existing16384-dimension and4Mi
padded-pixel caps remain. Negative origins are allowed. Signed extent/negation
checks also cover32-bit targets.

The working estimate retains the previous128bytes per maximum possible
internal pixel and adds conservative cumulative f32 colour/extra-channel planes
for every scanned layer, even when the codec can free or replace references.
Permutation entropy receives an additional8MiB allowance before parsing.
Embedded ICC size/entropy admission still runs first; advancing its coded
entropy stream does not reconstruct an expanded profile a second time. The
main decoder validates/finalizes that profile before any pixels. Output samples
and CMS behavior are unchanged.

The actual largest scanned layer bound is passed to the codec's `sample_limit`,
including its minimum-width padding, hard-coded three colour channels, extras
and exclusive comparison. It is never disabled. Canvas pixel/byte quotas remain
independent. Expanded workspace is checked before codec pixel allocation, then
retained metadata is checked again. This is the existing conservative workspace
contract, not a process-RSS or arbitrary codec-internal allocation guarantee.
Animations, previews and multiple visible outputs remain unsupported.

## Independent evidence

The source and license/provenance are in
`wml2/tests/fixtures/review/jxl-sunset-logo.json`. The source is CC0; the pinned
conformance testcases README permits reproducing bitstreams and decoded test
references. Independent libjxl0.11.2 C API output uses normalized straight
RGBA16. Ordinary output rounds that reference toRGBA8; native checks compare
all rounded pixels and4096spatial full-u16 probes. Managed output uses the
actual libjxl reconstruction ICC, LittleCMS2.16 relative intent/NOOPTIMIZE with
alpha excluded, then final8-bit rounding. The official float reference contains
values outside[0,1]; integer output is compared after the integer API's clamping,
with maximum error7.66e-6 against that reference.

The local maximum differences are0/255 ordinary,1/65535 native probes and1/255
managed. The three paths decode the complete oriented canvas, not a thumbnail.
Tests also reject a256MiB budget that fits the former canvas-only estimate but
not the actual layers, every truncated prefix of the218-byte input, and more
than64internal layers even with caller `unlimited()`. Admission failures occur
before initialization. The existing positive/Abort/error/metadata tests remain.

Reproduce only this independent oracle without regenerating unrelated inputs:

```console
python3 wml2-test/scripts/generate_jpegxl_additional_fixtures.py --tools /path/to/libjxl-tools/usr --work .test-jxl-references --layer-only
cargo test -p wml2 --locked --no-default-features --features jpeg-xl,image-buffer-ex,color-management --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers
python3 wml2-test/scripts/verify_jpegxl_browser.py --output .test-jxl-browser
```

The actual Chromium harness now covers14cases, including this full-size managed
layered image. Its fixed output staging capacity is checked before copying.
Node retains its61existing source/native/limit cases. Source fixtures and
independent references are test/development data; no new runtime dependency or
codec version is introduced.

Windows verification reported by session01a14e00-f0d4-7060-865c-7e99da5ec0fc
covers ICCcc4e4a1 (132passed,17native frame transforms) and JXL8303737
(116passed,0failed;31ordinary/EX hashes unchanged from83c1800). This cloud
workspace did not execute that Windows-local25-image corpus. Performance and
layer changes after those revisions require Windows delta verification.

## Existing-path cost

Paired release comparisons use unchanged8110aa6 and the layer preflight in the
same executable, alternating order per iteration,3processes/case onCPU0, with
no concurrent local builds/tests. Small7×5 inputs use1100iterations/first100
omitted (3000samples/variant); the606×500 oriented RGB uses110/first10omitted
(300samples/variant). Pixel hashes match; additional bounded header/TOC work
adds a small fixed allocation/setup cost. The first prototype unnecessarily
constructed a second expanded ICC. The final version reuses the admitted ICC
entropy histogram/reader and avoids that profile construction.

| Case | Before µs | After µs | Paired change | Allocations before/after |
| --- | ---: | ---: | ---: | ---: |
| ordinary-rgba16 | 84.30 | 85.33 | +1.36% | 388/392 |
| ordinary-oriented | 18367.89 | 18500.96 | +0.55% | 966/967 |
| EX-gray16 | 31.37 | 32.13 | +2.81% | 239/240 |
| EX-ICC | 63.33 | 68.24 | +7.67% | 370/374 |
| managed-ICC | 184.71 | 191.86 | +3.53% | 382/386 |

The EX ICC setup case adds about5µs and7.7% in this run; managed ICC adds
about7µs/3.5%. These are measured costs of the additional resource preflight,
not a claim of unchanged speed. This does not undo exact adjacent-colour reuse
or alter transform precision. The layered image had no successful old baseline;
its decode is reported as a new supported workload, not a before/after speedup.

Final local checks pass53extended and41minimal tests (overlapping), scoped
Clippy with11existing unrelated warnings and formatting. Independent layer-only
reference regeneration reproduces the saved hashes. Node61 and Chromium14cases
pass; Chromium reports58,720,256bytes of WASM linear memory and no host imports.
