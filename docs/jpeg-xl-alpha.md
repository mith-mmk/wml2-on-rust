# JPEG XL alpha qualification

This addresses the mixed-depth subset of #42 on verified main
`6a48db924b338c5ae7095bea6105fb4dbd436e95`. The runtime remains pinned to jxl
0.7.4. Associated/subsampled alpha and general extra planes are deferred, not
silently accepted, dropped or mixed into RGB. No animation, HDR, tone mapping,
streaming, intrinsic or preview behavior is added.

## Qualified contract

Accept at most one full-resolution, unassociated integer alpha channel. Colour
and alpha can independently have depths in {8,9,10,12,16}. The pinned render
pipeline converts colour using its colour depth and each extra channel using
that channel's own depth (`frame/render.rs`, lines602–611).

A mixed-depth image has one normalized Rgba16 native EX source layout for all
four channels. Original depths are independently recorded as `JPEG XL Source
Bit Depth` (colour) and `JPEG XL Source Alpha Bit Depth` (alpha, added only for
mixed-depth images), both `DataMap::UInt`. `JPEG XL Output Bit Depth` still
describes delivered samples. Source header/profile/orientation metadata remains.
Same-depth images retain the previous Gray/RGBA8/12/16 policy;9/10-bit sources
already normalize to16. These changes do not relabel alpha as colour precision.

For example colour8/alpha16 returns normalized Rgba16 through native EX, keeping
alpha low bits and RGB beneath zero alpha. Exact Rgba8/Rgba12 storage requests
reject this source layout; Round converts all channels under the existing
policy. Colour12/alpha8 also normalizes to16, so callers should use the separate
source-depth metadata rather than infer encoded depths from EX storage alone.
Integer normalization may differ from independent C output by one u16 unit;
tests additionally recover every original alpha code exactly. Ordinary RGBA8
and explicit sRGB output retain deterministic rounding. CMS transforms RGB only;
alpha is copied/rounded without a colour transform. Managed alpha matches the
independent oracle exactly at8 bits in the qualification matrix.

## Admission

The input, metadata, pixel, padded/internal-layer, frame, ICC and expanded-workspace
guards and hard caps remain. There are still at most four reconstruction/output
channels; new mixed inputs introduce no extra planes or channel-count growth.
The existing conservative workspace estimate already covers these16-bit buffers.

Canvas admission now uses the selected common native depth: mixed colour8 /
alpha16 requires8 bytes/pixel, just like an existing16-bit RGBA source. This is
checked before decoder initialization/output allocation, including the ordinary
route. A7×5 fixture passes with280 canvas bytes and rejects279 before callbacks.
Metadata scalar names/values remain charged to existing metadata/workspace
estimates. No caller `unlimited()` removes the channel/profile/fixed runtime caps.
These are known-buffer estimates, not bounds on total process RSS or allocator
and caller-owned memory.

## Explicit deferrals and executable evidence

| Input | Current policy and evidence |
| --- | --- |
| Associated alpha | Reject. In the actual0.7.4, `premultiply_output=false` preserves already-associated samples; it does not produce straight output. The trusted core probe exactly matches C's as-is output, but differs from C's explicit unpremultiplication by up to65522/65535. A wrapper would need a separately qualified high-precision unassociation, zero-alpha/transparent-RGB and ICC-order contract; it is not claimed impossible or implemented here. |
| Subsampled alpha (`dim_shift>0`) | Reject before codec work. C libjxl0.11.2 encodes/decodes the odd7×5, orientation6, dim_shift1 fixture. The direct0.7.4 core returns SectionTooShort;0.8.0 AsIs produces all C pixels/alpha exactly. This characterizes an upstream capability gap, not a new corruption/security finding in the guarded adapter. |
| Float alpha / other depths | Reject. The independently C-decodable half-float-alpha and4-bit-alpha cases verify the supported integer-depth subset is explicit. Colour float/HDR remains separately rejected. |
| Multiple alpha, alpha+depth, depth-only, spot colour | Reject before publication, even with unlimited caller limits. C independently encodes/decodes each file and exposes its channel type/depth/exponent/dim_shift/association; the manifest records them. Spot colours are never rendered by the probe or adapter. No separate-plane API or silent discard is introduced. |

The [official0.8.0 release](https://github.com/libjxl/jxl-rs/releases/tag/v0.8.0)
identifies the dim_shift fix and AlphaMode/unpremultiplication addition. Source
inspection confirms that0.7.4 conditionally incorporates dim_shift only when
colour upsampling>1, while0.8.0 incorporates it unconditionally. Both direct
cores default to as-is associated samples; the0.8 Unpremultiplied option was
not exercised in this evaluation. The runtime cannot be upgraded by changing
one version: [PR #54](https://github.com/mith-mmk/wml2-on-rust/pull/54) / #40
records private-header/ICC/internal-layer admission blockers. Those guards are
not weakened here.

The0.7 public BasicInfo extra-channel descriptors expose type and association,
but omit source depth/dim_shift. Its pinned public header reader exposes those
fields;0.8 restricts that parser. A future general-plane API must retain type,
source precision, coded/output plane geometry, association and selection, with
bounded channel counts/aggregate retained buffers and explicit unsupported-type
errors. Gray/RGBA callback storage is insufficient. The present rejection policy
avoids claiming that general-plane retention is implemented.

`jpeg-xl-alpha-core-evaluation.json` records the direct trusted-core comparisons,
exact probe/lock identity and source locations. The development probe comes from
PR #54's `jxl08_probe.rs` at commit7c81dea; it is not a production admission API.
Build that PR's evaluator with an isolated target directory, then run its
`probe <jxl-alpha-refused-associated.jxl|...subsampled.jxl> <07|08> 1 <.test-output>`.
Compare normalized output to the C oracle. No one-run core timing or requested
allocation data is used as performance/RSS evidence here.

## Fixtures and verification

```sh
python3 wml2-test/scripts/generate_jpegxl_alpha_fixtures.py \
  --include <libjxl-0.11.2-headers> --work .test-jxl-alpha
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,image-buffer-ex,color-management \
  --lib --test jpeg_xl --test jpeg_xl_additional --test jpeg_xl_layers
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-jxl-alpha-wasm
```

Matching C libjxl0.11.2 headers/runtime and LittleCMS2.16 are development-only
oracle dependencies. Fifty C-generated files cover GrayAlpha/RGBA ×5 colour
×5 alpha depths on odd7×5 images, with orientations1–8 distributed across the
matrix (not every depth combination crossed with every orientation). Ten are
same-depth controls;40 qualify new mixed inputs. Each uses zero, minimum positive,
half, maximum-minus-one and maximum alpha, with nonzero transparent colour.
The generator verifies C signaling and recovery of every original colour/alpha
source code, and retains full independent RGBA16 output. Two additional RGB ICC
files cover colour8/alpha16/orientation6 and colour16/alpha8/orientation8; the C
output ICC feeds LittleCMS, with alpha copied and final8-bit quantization.
Expected pixels are never generated by WML2 or compared only to another WML2 call.

Tests cover native/ordinary/CMS, Exact/Round, source metadata, original mixed
fixture compatibility, six caller budgets, promoted-canvas admission before
callbacks and eight explicit refusal cases. The existing integration target
includes the new cases, so the established platform/feature CI runs execute them.
Actual Node WASM adds109 qualification cases to the base61. See
[jpeg-xl-alpha-validation.json](jpeg-xl-alpha-validation.json) for exact revisions,
counts, hashes, display checks and paired release measurements.
