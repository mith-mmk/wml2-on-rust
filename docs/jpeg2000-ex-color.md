# JPEG2000 EX unsigned RGB/RGBA12/16 (unreleased)

This continuation starts at PR #14 head
`f8cdef98f4cab2060248a7ab58dffd383a572209` on a separate branch.
WML2 remains 0.0.36. PR #13, PR #14 and main are not rewritten or merged;
this work does not post a new PR, publish, tag or release.

## Accepted profile and output

Enable `jpeg2000,image-buffer-ex` and call `image_decoder_ex` or
`image_from_ex`. Every component, including alpha, must be unsigned, uniformly
12 or 16 bits, full resolution, classic reversible 5/3, with no scalar
quantization or maxshift ROI. Existing tile/packet/layer assembly is reused.

| Input | Declaration | EX source/output bytes |
| --- | --- | --- |
| One component | Raw J2K or JP2 Gray, existing profile | Gray12/16, unchanged |
| Three components | Raw J2K RGB convention or JP2 RGB enum 16 | RGBA12/16 with opaque alpha |
| Four components | JP2 RGB enum 16 and complete straight-alpha CDEF | RGBA12/16 with preserved alpha |

The RGB output uses the existing RGBA formats, adding alpha 4095 or 65535.
No public RGB format, signed format or implicit precision conversion is added.
Colour samples are explicitly LE bytes in R,G,B,A order, eight bytes per pixel; source
stride is packed (`None`). Exact/Round storage conversion remains explicit and
scales the entire normalized range, including alpha. RGB at alpha zero is
retained. Alpha remains straight and no colour-profile transform is applied.

An optional reversible component transform applies to the first three coded
components. Signed centred integer coefficients stay intact through inverse
5/3, then inverse RCT uses i64 intermediates and floor division for negative
chroma sums. DC shift and sample-domain clamping follow inverse RCT. The alpha
component is independently reconstructed. No f64 reconstruction or legacy u8
component store is used for EX. Gray retains its dedicated LE packing loop.

## CDEF and alpha boundary

Three-component RGB may omit CDEF, using R,G,B component order. When present,
CDEF must completely and uniquely map component indices 0..2 to colour
associations 1..3. These colour associations may permute RGB order, and CDEF
records may occur in any order. Component mapping/palettes are not supported.

RGBA requires exactly four definitions: colour components 0..2 form the same
bijection, and component 3 is type 1, association 0 (straight whole-image
opacity). This is the canonical opacity declaration written by
[OpenJPEG 2.5.3](https://github.com/uclouvain/openjpeg/blob/v2.5.3/src/lib/openjp2/jp2.c#L1904-L1917).
The independent reader applies CDEF colour order before exporting PGX samples.

Raw four-component codestreams and four-component JP2 without that complete
declaration are rejected: a fourth component alone does not declare straight
alpha. Empty, partial, duplicate or ambiguous CDEF, unknown channels,
premultiplied type 2, per-colour opacity, and alpha in another component slot
remain unsupported. They are rejected before initialization, rather than being
silently reinterpreted or rounded. General alpha placement is follow-up work.

Mixed precision (including lower-depth alpha), signed samples, 9/7 high depth,
other coded depths, subsampling, HT, palette/CMAP, sYCC and effective ICC remain
unsupported. The existing first-understood-colour policy is retained; ignored
later colour declarations are not applied. No animation, streaming, decoder ROI
or general colour-management API is introduced.

## Compatibility and resource contracts

Existing Gray12/16 EX pixels, precision declarations, callbacks and large-origin
index arithmetic remain covered. Ordinary JPEG2000 APIs retain existing 1..8-bit
RGBA8 acceptance and reject high depth. Their established palette, CDEF and
premultiplied-alpha handling is unchanged.

The EX canvas budget is two bytes per Gray pixel or eight per RGBA pixel.
Conservative component reconstruction, segment/contribution state and packet
ordering allocations are included with the canvas, using checked arithmetic.
Input/metadata have separate budgets; workspace bounds are not total RSS limits.
Relative coordinates are formed before canvas row/column offsets, including
small canvases with large absolute origins on 32-bit targets.

All validation and reconstruction finish before init_ex. Abort terminates once;
other callback errors preserve the existing propagation contract. Unsupported,
malformed or over-budget input emits no callback.

## Fixtures and reproducible checks

104 positive streams cover both precisions, RGB/RGBA, RCT on/off, raw/JP2 where
supported, colour permutations, low bytes/extremes, transparent RGB, odd sizes,
origins, tiles, singleton edges, all five progressions and multiple layers/parts.
Seeded original U16 values are encoded using OpenJPEG RAWL with explicit coded
precision, then independently decoded into per-component PGX and compared exactly.
Six readable signed/irreversible/10-bit RGB/RGBA streams are negative cases.
Unsupported CDEF, mixed precision and precision-conversion policies have focused
tests. The earlier two RGB16 negative entries are removed because uniform RGB16
is now accepted; existing Gray positives remain unchanged.

Native and actual wasm32/Node share pixel, odd byte-stride/subrectangle,
callback/resource-boundary and large-origin checks. WASM runs explicitly enable
release overflow checks and require no host imports. The feature CI matrix runs
the new integration target, and the Windows i686 command explicitly includes it.
Fixtures/tests remain development material and are excluded from the package.

```sh
python3 -B wml2/tests/fixtures/review/generate_jpeg2000_ex_color.py \
  --compress /path/to/opj_compress --decompress /path/to/opj_decompress \
  --work-dir .test-jpeg2000-color-oracles
cargo test -p wml2 --locked --release --no-default-features \
  --features jpeg2000,image-buffer-ex --test jpeg2000_ex --test jpeg2000_ex_color
python3 -B wml2-test/scripts/verify_jpeg2000_ex_color_wasm.py \
  --overflow-checks --work-dir .test-jpeg2000-color-wasm
python3 -B wml2-test/scripts/verify_jpeg2000_ex_color_wasm.py \
  --default-features --overflow-checks --work-dir .test-jpeg2000-color-wasm-default
```

Detailed source hashes, checks and local performance/visual evidence are retained
in [the validation record](jpeg2000-ex-color-validation.json). Exact-head remote
CI is reported separately when the branch is pushed. Independent review is not
rerun as part of this feature implementation.

The release workspace passed 510 tests with 14 existing ignored; the narrow
native matrix passed 133 and high-bit-depth focused checks passed 44. Actual
minimal/default WASM each passed 104 colour fixtures and six profile rejections.
The retained eight-pair legacy measurements range from -5.10% to +2.89%; Gray EX
changes are +0.18%/-0.22%. All output hashes match the fixed base. On this host,
33x29 RGB12/16 decode medians are 1.51/1.84 ms and RGBA12/16 are 2.05/2.39 ms.
These observations do not guarantee performance on other hosts or inputs.
Builds used two jobs; sampled cgroup peak was about 3.67 GiB of 16 GiB, with no
OOM event. [Decoded/reference colour, low-byte and alpha panels](jpeg2000-ex-color-display.png)
were visually checked. [Raw logs and measurement harness](jpeg2000-ex-color-raw.json.gz)
are retained. Base/current benchmark builds use separate target directories.
