# ICC metadata and image lifecycle

`ICC Profile: DataMap::ICCProfile` describes the current declared samples.
`Source ICC Profile` retains original bytes without asserting that a numeric
sample expansion applied color management. Opaque retention does not validate a
profile or compile a transform.

| Boundary | Result |
| --- | --- |
| Ordinary RGB/palette PNG or three-component JPEG | Keep existing `ICC Profile`; RGB reconstruction/alpha insertion alone does not apply ICC |
| Ordinary PSD RGB/indexed output with a declared RGB profile | Retain `ICC Profile`; source-only declarations are not promoted |
| PSD Gray/CMYK, short or incompatible ICC | Preserve opaque `Source ICC Profile`; numerical expansion applies no CMS |
| Ordinary Gray/GrayAlpha PNG or Gray/CMYK/YCCK JPEG expanded to RGBA | Keep original bytes as `Source ICC Profile` |
| Native EX Gray storage and Gray callback ICC | Keep Gray samples and `ICC Profile`, including channel-preserving depth conversion |
| Gray callback into RGBA EX storage | Preserve source declaration and demote callback ICC to `Source ICC Profile` |
| Native Gray EX exported through `to_rgba8` or `LegacyPickAdapterEx` | Replicate Gray numerically, keep original buffer unchanged, expose ICC as source metadata |
| Explicit highres `FrameTransform` | Gray↔RGB builds separate planar output; typed active descriptor and any existing generic `ICC Profile` use destination bytes; `FrameMetadata.source_color` and `Source ICC Profile` remain source provenance |

EX PNG/TIFF/AVIF retain their existing source-only naming. Numeric precision or
channel conversion is never a CMS. Conflicting distinct generic/source profiles
at Gray-to-RGBA demotion return an error requiring the caller to resolve them;
neither profile is silently overwritten. Native RGB and Gray retention do not
require the optional `color-management` feature or a supported transform route.

EX AVIF encoding uses `icc-profile`'s checked container parser to reject a
known ICC model incompatible with requested Gray/RGB samples. It also rejects
a known incompatible source ICC unless encoder options explicitly supply a
compatible output ICC. Callers must perform the actual color conversion
themselves; supplying a profile only signals caller-prepared samples. A native
Gray output remains available. The established opaque explicit-output ICC API
is retained: unparsable bytes are opaque, not validated CMS declarations.
Source-only metadata is never promoted or applied automatically.

The ordinary AVIF encoder has no output ICC signaling contract. Known non-RGB
source/active ICC therefore rejects there; use the EX encoder with native Gray,
or explicitly managed RGB and compatible output metadata. This does not add
ordinary ICC signaling, an AVIF decoder correction, or CMYK color management.

## Reused targets

`DrawCallback::begin_image()` is a default no-op for source compatibility.
The public `image_decoder[_ex][_with_limits]` entrypoints call it once before
input validation and metadata. Direct guarded JPEG/PNG decode entrypoints also
call it. An embedded guarded codec does not restart its outer image lifecycle.
Other format-specific legacy decode functions do not promise this boundary;
use the common entrypoints or explicitly call `begin_image` when reusing those
targets directly.

Built-in `ImageBuffer` clears per-image metadata there. `ImageBufferEx` also
invalidates the prior canvas/source declaration while retaining the configured
storage format and precision policy. Metadata delivered before `init` belongs
to the new image and survives initialization. Neither `init` nor animation
`next` clears metadata. Custom collecting wrappers that support reuse should
reset their own metadata/state in `begin_image`, forwarding it to owned targets
as needed. Wrappers using the default keep their existing behavior.

Abort from `begin_image` stops before further callbacks/input inspection and
calls `terminate` once; a real callback error remains an error. Existing format
init/draw/metadata/next Abort contracts remain in force. `terminate` alone does
not prove metadata completeness or successful decoding. After an error or
Abort, callers must check the decode result before using the target.

Regressions are in `jpeg_png_icc_metadata.rs`, `image_buffer_ex_icc.rs`,
`avif_icc_output.rs`,
`image_metadata_lifecycle.rs` and `icc_registry_lut.rs`. The frozen Lab-PCS
fixture compares the registry CMS through WML2 to an independent LittleCMS
reference. Progressive CMYK JPEG raster decoding remains a pre-existing
unsupported case in this fixture matrix; its metadata routing is tested at
the parsed-header boundary only.
