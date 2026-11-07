# Focused JXL integration (unmerged)

Target: published jxl 0.8 decoding and JXL animation. No encoder implementation or changes.
Main baseline: `f2151b2408e9d04f1bc7b4e9e29679e078a57209` (0.0.41).
This series does not depend on the former preview/HDR/extra-plane/resource-policy stack.

## PR disposition

The following PRs were closed at the user’s request after reviewing their individual
base-to-head diffs. Closing preserves branches, source and history. These are not
features shipped in main.

|PR|Reviewed head|Files|Reason|
|---|---|---:|---|
|#74|`d41bc21eb3fee3ac4c388f43ba1de665a42eaa7f`|11|Codec-wide heap evaluation/cache accounting; unnecessary for migration/animation|
|#76|`240f31347807799da01bb71457b238fe822f29d9`|4|Preview extraction; unnecessary for migration/animation|
|#77|`f9ef774ed5d59e36623499ca9dbdb35ef2c60d13`|10|Preview extraction; unnecessary for migration/animation|
|#78|`f3a661e31d3c260085190711c4c25157b81e2fc6`|16|Preview extraction; unnecessary for migration/animation|
|#79|`6edd0128a5668adc4cc260a7c8184e42008ca931`|7|Preview extraction; unnecessary for migration/animation|
|#80|`de163ee2f75a830dd5eb385b5ebebd67acdb5578`|33|HDR/float extension; unnecessary for migration/animation|
|#81|`d7575f365d6bff84b85cbf11520be42bc537d909`|32|HDR/float extension; unnecessary for migration/animation|
|#82|`95ff7ad01191b75a7a075f8aaec95537c1c88e40`|26|HDR/float extension; unnecessary for migration/animation|
|#83|`cc0d54535db3209317a5e0820fc3a9c3074f053b`|16|HDR/float extension; unnecessary for migration/animation|
|#84|`aa6593ae8dc178cc908cd42738ef352704677ad0`|45|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#85|`d979935ba4313034cd2147da2699985d307aa3e8`|115|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#86|`1543251aebe9047b1b5dbb9e26d4bdefdf821014`|116|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#87|`133be97de10c4cd0e9defb29dd26dd77db793ee4`|115|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#88|`64c7a7c65cfab08d35d6be9fa6c2452e7e0f4d3c`|167|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#89|`cb41c10084c22d738735edfac4a5d0263e6f9aa3`|169|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#90|`1e2a27f0dc9a94cb70f8991c2b44b01cf7c6d868`|238|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#91|`9efd75638e13132c09cac6ab834db730193e2ecd`|283|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#92|`87b554fd01c258a36cbe9353894fb2681ac8c791`|612|Associated/subsampled alpha and general extra planes; unnecessary for migration/animation|
|#95|`721025a0ccbd19faeaaf3b68a5bcc8d7d4132e26`|12|Codec-wide heap evaluation/cache accounting; unnecessary for migration/animation|
|#96|`b5e216d991b1293dd20640422da6d184ee8d5c49`|10|Codec-wide heap evaluation/cache accounting; unnecessary for migration/animation|
|#97|`052e30262f4bf84952986e95482acc0123d4b681`|3|Integration guidance for the superseded broad stack; unnecessary for migration/animation|
|#98|`78c74be37582df7e63a1aad8618970f7663358a7`|37|HDR/float extension; unnecessary for migration/animation|

Retained goal sources: #75 (migration), #93/#94 (animation), #54/#67 (earlier evaluations).
The new series transplants necessary code rather than depending on their old bases.
Retained pending judgment: #72/#73 (resource API/fallible copy safety), #99–#101
(input/container/animation metadata safety fixes). These may contain independently
useful fixes; they were not rejected merely because their bases contain unwanted features.
Independent cases left untouched: #51 docs, #71 TIFF, #60 JPEG reconstruction,
#61 encoder evaluation, #63 resources, #68 HDR and #69 progressive evaluations.
Encoder work remains outside this task, including #61.

## Migration boundary

Pixel decoding uses published `jxl =0.8.0` exclusively. The aliased `jxl =0.7.4`
provides existing pre-allocation header/ICC/TOC readers and dimension-only inspection.
The 0.8 public API does not expose equivalent bounded parser operations. Removing
these checks or duplicating private parsers is outside this migration. No fork,
vendor patch, new resource-policy API or fixed cap increase is introduced.

The old typestate calls become explicit BasicInfo → FrameHeader → FrameComplete →
Complete events, with preflight/public-header cross-checks and authoritative final
file length. Still-image precision, metadata, source profile, orientation,
straight alpha, cancellation and opt-in sRGB conversion retain their contracts.

## Migration validation

Rust 1.91.0, no defaults, jpeg-xl/image-buffer-ex/color-management: 106 tests
passed (35 library; 41 main JXL; 6 additional; 6 brob; 3 layers; 6 metadata;
9 slice admission). Feature-off detection: 1 passed. Actual wasm32 Node24.19:
229 composed cases passed, no host imports. This is a real 32-bit execution; no
other 32-bit native platform is claimed. C libjxl/djxl lossless, lossy, orientation,
alpha and ICC oracles remain exact/within their original tolerances. Decoded
606×500 photo was rendered and visually inspected.

Five alternating release benchmark blocks against main at the same source path,
three warmups and nine timed decodes per process, identical features/settings:
RGB8 1.066×, tiny ICC 1.127×, photo 0.784×, layered sunset 0.915× baseline time.
All pixel hashes match. Small-input slowdown remains a review tradeoff; this is
not a universal speedup. Measurements on a shared cloud host are descriptive.
Raw trial medians and input hashes: [validation JSON](jpeg-xl-focused-validation.json).
CI completion is deliberately not awaited, per user instruction.
