# Associated integer alpha depth combinations

This unmerged follow-up to #84 implements the next part of #42: independently
supported8/9/10/12/16-bit color and alpha depths on full-resolution associated
Gray/RGB SDR stills. The original acceptance condition explicitly asks for depth
combinations. Published0.8.0 already normalizes both channels and exposes the
same unpremultiplied output used by #84; no upstream change or user display
policy is needed. This is production admission plus independent qualification,
not another backend evaluation.

The product change removes only the matching8/16 restriction. Both color and
alpha still pass the existing finite integer-depth whitelist. Source association
is cross-checked with public BasicInfo. Original profile, no preview, one coded
layer, one full-resolution alpha, SDR, and all existing input/ICC/TOC/padding/
workspace caps remain. Associated subsampled, non-alpha/multiple extras, float/HDR/XYB,
preview and multi-layer associated inputs remain unsupported. No dependency,
public API or global resource ceiling changes.

## Output and acceptance mapping

All associated inputs still normalize to native straight RGBA16. Ordinary output
uses existing deterministic RGBA8 rounding; EX Exact requires Rgba16, while lower
precision requires explicit Round. Source color and, when different, alpha bit
depth stay distinct metadata. ICC/CMS sees straight RGB and never transforms alpha.
The zero-alpha convention is the already qualified #84 behavior; no original
pre-association or hidden-color recovery is promised.

| Original #42 acceptance | Current evidence / remaining scope |
| --- | --- |
| Partial/zero/colored-transparent alpha | Existing #84 vectors retained; matrix adds minimum positive/half/near-opaque/opaque for each depth. |
| Depth combinations | All25 color×alpha pairs, independently generated for Gray and RGB; native RGB within1u16, alpha exact against C. |
| Odd sizes and orientation | All matrix inputs7×5; orientations1–8 across each color model, independently oriented C output. |
| ICC, alpha outside transform | Two new9/16 and16/10 embedded RGB ICC cases plus #84 cases; source/reconstruction ICC identity and independent C→LittleCMS output checked. |
| Channel and cumulative buffer limits | One full-resolution alpha; unchanged native16 buffers/ledgers, exact workspace boundary and limit-before-callback tests on new mixed input. |
| Subsampled alpha and general extra planes | Subsequent [declared straight subsampling](jpeg-xl-subsampled-alpha.md) implements a subset; general extra-plane API remains unimplemented. |

#42 stays open. The completed integer-depth portion exists only on this draft
stack, not main0.0.41. No broader completion condition is introduced.

## Reproduction and measured validation

Build existing `jpegxl_alpha_fixture_encoder.c` and `jpegxl_alpha_reference.c`
against installed libjxl0.11.2 with `-Wall -Wextra -Werror`, as described in the
[#84 report](jpeg-xl-associated-alpha.md). Run
`python3 -B wml2-test/scripts/generate_jpegxl_associated_depths.py --work .test-oracle`.
The generator checks C header depths/association and writes50 encoded inputs,
normalized RGBA16 references, two LittleCMS2.16 managed references, the generated
Rust case list and [hash manifest](jpeg-xl-associated-depth-oracle.json). It also
re-decodes the historical16/8 refusal sample and verifies its existing committed
straight oracle before using it as positive coverage. Candidate output never
constructs expected pixels. Existing #84 fixtures and its recorded evidence are
retained.

The shared production tests now compare66 independent associated cases. New
metadata checks preserve both depths and verify successful preview absence for
these admitted stills. CMS compares four depth combinations using embedded RGB ICC within2u8
RGB with exact alpha and exact source/reconstruction ICC identity. Cancellation
is injected at all18 checkpoints on the previous matching input and all18 on a
new mixed ICC input, with scoped zero net retained heap. Workspace/output/input/
metadata/pixel/frame limits, every truncated prefix and callback Abort are
exercised on new mixed inputs. Feature-disabled compilation and unchanged HDR,
F16/F32, preview, reference-layer and resource regressions remain covered.

[Local execution and timings](jpeg-xl-associated-depth-validation.json) record
139 expanded and34 minimal passing tests; actual Node24 WASM361 checks without
host imports, including every new matrix case at native/legacy precision. The
WASM harness asserts dimensions, source depths, exact alpha, pixel limits and
EX precision refusal. Its observed final memory size is not a global codec heap
or worst-case peak bound. macOS/Windows/Windows32 execution belongs to CI rather
than local Linux. No new streaming, latency or full heap guarantee is claimed.

An oriented mixed-depth ICC image is rendered over a checkerboard using both
independent C+LittleCMS and WML2 output and visually compared. Benchmarks pair the
same previously supported matching16-bit ICC input/build/features on CPU0 in six
alternating process pairs. Newly admitted16/10 timings are separate because the
parent intentionally refuses that input. Full numbers and variability are stored
without interpreting overlap as proof of no regression.

The matching16-bit control median changes ordinary74.899→74.401µs (ratio0.9934)
and managed155.198→152.824µs (ratio0.9847). Process-median ranges overlap:
ordinary73.579–75.602µs before /73.789–75.607µs after; managed151.745–162.316µs
before /149.827–153.162µs after. No speedup is inferred from these differences.
Three new16/10 process medians span ordinary73.569–75.182µs and managed
159.402–176.232µs. Configuration, per-block numbers and common control hash/size
are in the validation JSON; the harness is `jpegxl_alpha_bench.rs`.
