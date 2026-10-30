# Runtime PR composition evidence (2026-10-30)

This is an isolated local integration experiment, not a merge or approval of any
GitHub PR. Original public branches and `main` are unchanged. No production
regression requiring a separate fix was found in the exercised combinations.
The individual issues retain their documented partial scopes.

The tested code/test commit is `d7a6121b64596a3e8ce83391d7b9cb71b65cdadf`, tree
`3759a5efa8e62066639b2d5c3e8e24679eb7290c`. The subsequent evidence commit only
adds this report, its data, and a default `.test*` Cargo target for the reproduction
script. See [machine-readable results](runtime-composition.json) for full input
heads, commands, measurements and WASM hashes. Toolchain: Rust 1.91.0 on x86_64
Linux; actual wasm32-unknown-unknown execution with Node 24.19.0.

## Inputs and mechanical resolutions

Starting from `main@6a48db924b338c5ae7095bea6105fb4dbd436e95`, exact reviewed
heads were composed in this order:

| PR | Head | Purpose |
|---|---|---|
| #57 | `81178d7c7359` | Typed geometry/header-only information |
| #32 | `da4f45764995` | Managed cooperative cancellation |
| #66 | `5f08606d4413` | Borrowed slice readers/pre-copy input admission |
| #56 | `b3c52a50962c` | Bounded compressed metadata |
| #58 | `7b693f09786f` | Structured EXIF/XMP; based on #56 |
| #62 | `a69a54f2bfad` | Bounded preview skip, main-only output |
| #64 | `d45a4f8feefa` | Intrinsic dimensions; based on #57 |
| #65 | `4eb32d9856e8` | Straight mixed-depth integer alpha |
| #53 | `b9c9644dbb0d` | PNG EX save |
| #55 | `dd45d8cd252f` | TIFF16 EX save; based on #53 |
| #59 | `695d0de97a98` | Baseline JPEG reduced IDCT; based on #57 |
| #52 | `b2196d9e7ec2` | AVIF encoder capability reporting |

All twelve heads are ancestors of the integration commit. Local merge commits
retain their history. The resolutions were:

- Combine #62/#64's shared preflight predicate: reject animation, retain both
  bounded-preview and intrinsic-size support. Neither complete branch predicate
  alone represents the combined contract.
- Keep #58's structured-metadata admission before #32's cancellable transition
  into frame processing; retain the cancellation runner, error mapping and check.
- Retain both cancellation and brob unit-test additions, preview/intrinsic test
  modules and all WASM case groups.
- Union runtime/oracle dependencies in Cargo.lock, retaining version-qualified
  miniz entries, `roxmltree` and TIFF/PNG oracle dependencies.
- Combine geometry documentation: header-only output is conditional ordinary
  output, reduced JPEG reports its selected canvas, and JXL output/display axes
  are oriented. Intrinsic metadata does not request resampling.
- Union CI test targets and EXIF feature settings. Keep Cargo's actual miniz
  artifact linkage for WASM. Existing fixed counts become 226 union cases plus
  three new composition groups, asserted as 229 actual WASM cases.
- Reconcile the support/metadata guide without restoring stale refusals of
  preview, intrinsic dimensions, brob or mixed-depth straight alpha.

No limits were raised and no evaluation-only future API was enabled.

## Focused composition results

The added fixture combines an embedded preview, 19x11 intrinsic dimensions,
orientation 6, a 7x5 coded main canvas, 16-bit alpha and an embedded ICC profile.
An independent C libjxl 0.11.2 decoder decoded the preview and main separately;
its main RGBA16 equals the frozen independent reference. Header BasicInfo
independently reports a 5x7 oriented output and stored 19x11 intrinsic axes.

New native tests verify:

- Header query: coded 7x5, conditional output 5x7, oriented intrinsic display
  11x19. Ordinary and EX pixels match independent references; the preview is
  never substituted. A bounded truncated prefix can answer geometry while full
  decoding refuses it.
- Add the frozen independently qualified compressed EXIF/XMP boxes to this input
  and to an 8-bit-color/16-bit-alpha input. Raw/decoded metadata, XMP, optional
  typed EXIF, normalized EXIF orientation and source alpha depth survive.
- Search each combined input's admission threshold and refuse one byte below it
  before any init/draw/metadata/end callback, on both ordinary and EX routes.
  The boxed inputs require more metadata/workspace than their unboxed sources.
  Native threshold observations are 18,329/9,600,260 bytes for preview/intrinsic
  metadata/workspace, and 17,146/1,206,396 for mixed-alpha metadata/workspace.
  These are observed adapter admission thresholds, not actual heap guarantees.
- Promoted native alpha storage is charged before initialization: a 279-byte
  canvas budget rejects the 35-pixel RGBA16 canvas. Native Exact values and
  Rgba8 Exact refusal survive metadata composition.
- Managed preview/intrinsic+metadata and mixed-alpha+metadata each pass a
  non-cancelling probe and typed cancellation at all 17 observed checkpoints.
  Cancelled private images are never returned.
- Pre-cancel beats malformed/oversized input and zero limits with zero measured
  allocations. Non-cancelled oversized input refuses with 3 allocation requests,
  82 total requested bytes, largest 42 bytes. The independent mixed-alpha CMS
  reference still matches RGB within its existing tolerance and alpha exactly.

## Executed matrix

| Profile | Passed test executions | Other result |
|---|---:|---|
| JXL disabled | 2 | success |
| Ordinary JXL | 84 | success |
| JXL + EX, EXIF off | 96 | success |
| JXL + EX + EXIF + own CMS | 118 | success |
| PNG/TIFF/JPEG/GIF/PSD + JXL/EX/CMS | 144 | 6 existing ignored tests |
| JPEG AAN / slower IDCT | 9 / 9 | success; LLM is covered above |
| AVIF encoder capability/encoding | 8 | success |
| Additional encoder-disabled tests | 1 | EX targets compile to zero tests |
| All compatible codecs / noretoro | 54 / 54 | both all-targets checks pass |
| CI selection | 22 Python tests | success |
| WASM EXIF on / off | 229 / 229 | no host imports, both pass |

The intentional all-features JPEG IDCT conflict is asserted by the compatible
feature checker. Legacy PNG/TIFF without EX also compiles. Final focused native
checks, Cargo formatting and diff whitespace checks pass.

Two of the six ignored tests above are manual EX encoder display/benchmarks;
they were separately executed in release mode and passed independent PNG/TIFF
sample round trips. Their 512x384 RGBA16 medians were PNG 18.890 ms; TIFF none
0.149 ms, LZW 116.347 ms, Deflate 12.093 ms (20 timed, 3 warmups). The PNG display
artifact and composed JXL output were opened for visual checking. Three external
TIFF corpus cases and the manual header-query benchmark were not executed.

## Performance observations

The existing requested-allocation benchmark was built against verified main and
the combined tree in separate release targets, using the same locked registry
dependencies. Two batches alternate baseline/candidate order, each with three
warmups and seven timed iterations pinned to CPU 0. Pixel hashes match per route.
The host was not a dedicated performance laboratory.

| Input / route | Main median range | Combined median range |
|---|---:|---:|
| 7x5 RGBA8 / ordinary | 51.798–56.715 us | 53.261–53.671 us |
| 606x500 JPEG-reconstructed / ordinary | 18.145–18.710 ms | 16.204–18.942 ms |
| Same / own CMS | 1076.243–1108.014 ms | 1072.971–1077.288 ms |

There is no consistent timing regression in these observations. Small-image
requested peak rises 69,085→69,325 bytes; the larger source route drops
6,178,973→5,994,712 bytes and CMS drops 7,391,595→7,207,334 bytes. These counters
measure requested live allocation bytes, excluding allocator overhead, codec
internal allocator behavior and RSS; they are not a full peak-memory bound or a
general speed guarantee.

## Reproduce

Checkout this evidence branch into a fresh `.test*` worktree. Preserve the
committed lockfile and use Rust 1.91.0. No merges of public PRs are necessary:

```sh
export CARGO_TARGET_DIR="$PWD/.test-composition-target"
export PYTHONDONTWRITEBYTECODE=1
python3 wml2-test/scripts/check_runtime_composition.py --output .test-matrix
rustup target add wasm32-unknown-unknown
python3 wml2-test/scripts/verify_jpegxl_wasm.py --exif --output .test-wasm-exif
python3 wml2-test/scripts/verify_jpegxl_wasm.py --output .test-wasm-noexif
cargo test -p wml2 --locked --no-default-features \
  --features jpeg-xl,exif,image-buffer-ex,color-management \
  --test jpeg_xl --test jpeg_xl_slice_admission composition -- --nocapture
```

Fixture regeneration additionally needs libjxl 0.11.2 headers/library and a C
compiler; existing source/oracle fixture files remain the pinned inputs:

```sh
python3 wml2-test/scripts/generate_jpegxl_composition_fixtures.py \
  --include /path/to/include --work .test-fixture-oracle
```

For benchmark reproduction, follow the separate-target build/helper commands in
[jpeg-xl-input-admission.md](jpeg-xl-input-admission.md#existing-valid-path-timing-and-requested-heap), use this
tree and `6a48db924b338c5ae7095bea6105fb4dbd436e95`, and run `ordinary`/`cms`
with the frozen `jxl-rgba8.jxl` and `jxl-jbrd-bench.jxl` inputs. Raw stdout is in
[runtime-composition.json](runtime-composition.json). The manual encoder probes
are `cargo test --release` with `png,tiff,image-buffer-ex`, selecting
`display_and_benchmark -- --ignored --nocapture` in both EX encoder targets.

This experiment does not qualify Windows/macOS, another Rust toolchain, every
possible cross-product of JXL features, hard heap bounds or cancellation latency.
Evaluation-only PRs #54/#60/#61/#63/#67/#68/#69 are not included as runtime changes.
No animation, float/HDR, resumable API, encoder, reconstruction API or dependency
migration is enabled by the composition.
