# Extra-channel acceptance audit after #92

This audit compares the original #42 acceptance conditions with the **unmerged**
production stack through #92 (`87b554fd01c258a36cbe9353894fb2681ac8c791`).
Published main0.0.41 still provides mixed-depth straight full-resolution Alpha;
the following draft work is not a released capability. Keep #42 open until its
accepted scope is integrated and the remaining policy is explicitly recorded.

| Original condition | Implemented and independently qualified in drafts |
| --- | --- |
| Associated/mixed-depth Alpha normalization | #84/#85: all supported integer color/Alpha depth pairs, straight output, colored transparency and zero-alpha policy. |
| Subsampled Alpha, odd sizes and orientation | #86/#87: declared2/4/8x straight/associated Alpha; original depth and coded geometry retained separately from reconstructed output. |
| Non-Alpha/multiple channel preservation and selection | #88–#91: source-index selection, ordered separate Depth/SelectionMask/Alpha planes, duplicate kinds and every Main5 slot0..4, including Alpha at each index. |
| Combined association/subsampling with multiple planes | #92: uniform effective1/2/4/8x reconstruction; `sample_bits_per_sample`, source association, shift and coded/output dimensions distinguish exact source codes from reconstructed normalized16. |
| ICC and Alpha separation | Existing original-profile/sRGB policies and independent C comparisons; Alpha/Depth/SelectionMask never pass through CMS. |
| Unsupported-type refusal, count and cumulative buffers | Explicit other-type/storage refusals, fixed Main5 count4, all owned plane/output/workspace boundaries, fallible reserves and cancellation. #92 adds120 independent C vectors; earlier draft manifests retain their own coverage. |

Two actual upstream constraints remain: pinned public APIs do not expose channel
names for name-based selection; fixed Main5 admits at most4 extra channels.
Neither is bypassed by private parser copies, guessing names, or a higher cap.

The qualified product subset still explicitly refuses mixed effective factors,
non-Alpha declared subsampling, multiple Alpha, float planes and other kinds
(e.g. SpotColor/CFA/Thermal), preview/XYB/HDR and cropped/multiple-layer extra
output. Mixed factors are **unqualified implementation scope**, not a proven
public-API impossibility. The representative Depth/SelectionMask contract plus
tested rejection of other kinds satisfies the original request to define the
unsupported-kind policy; it is not a promise to implement every defined kind.
Neither the issue nor this audit introduces an unlimited Cartesian product of
all sampling, types and storage combinations as a new completion requirement.

The accepted normalization/preservation/refusal work is now reviewable as one
finite stack. Additional combinations can be separately qualified if requested;
repeating small plane-count/type increments is less useful than progressing
original #43 animation. This audit does not close #42, claim all extra-channel
support, or merge the stack. See [#92 contract](jpeg-xl-extra-alpha.md) and its
[validation](jpeg-xl-extra-alpha-validation.json) for exact pixel semantics and
limits. The next independent production stage is
[buffered native animation](jpeg-xl-buffered-animation.md), with its own narrower
profile and frame contract; extra-plane support is not implied for animation.
