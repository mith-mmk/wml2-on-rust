# JPEG XL incremental/progressive evaluation (#47)

Decision: **hold the production resumable API**. The pinned jxl-rs 0.7.4 core
successfully resumes these progressive streams and renders changing provisional
pixels. Whole-input WML2 already accepts them. This does not establish bounded
incremental admission, provisional publication or end-of-input validation for an
untrusted streaming API. No production source, public API, dependency, limit or
version changes in this batch. #47 remains open.

This branch is independent of main `6a48db924b338c5ae7095bea6105fb4dbd436e95`.
It does not incorporate animation #67, preview #62, cancellation #32, migration
#54 or resource-profile changes. The separately tested #32+#62 composition is
explicitly identified below; their public branches are unchanged.

## Reproduction and independently established pixels

```sh
python3 wml2-test/scripts/evaluate_jpegxl_progressive.py \
  --output .test-progressive-evaluation --target-dir .test-progressive-target \
  --include /path/to/matching-libjxl-0.11.2/include
cargo test -p wml2 --locked --release --no-default-features \
  --features jpeg-xl --test jpeg_xl_additional \
  progressive_core_evaluation::frozen_incremental_boundaries_match_independent_c_pixels
```

The first command requires Rust/Cargo, a C compiler, matching C libjxl **0.11.2**
headers/runtime and the usual platform linker. The script checks both C versions.
Default execution verifies frozen input hashes and replays independent C decoding;
`--write-fixtures` explicitly regenerates inputs, oracle bytes and manifest.
No network fetch or dependency update is performed by the evaluator. Use the
repository's Rust environment when invoking it. The actual local run used
Rust1.91.0, C libjxl0.11.2 and jxl-rs0.7.4 with default features disabled.

The development C encoder creates deterministic 32×24 RGB8 VarDCT streams at
quality distance1/effort3, with progressive AC and quantized AC, with DC0 or DC1.
The pinned header walk confirms **two actual AC passes** in each visible frame.
The DC1 case additionally has an invisible LF frame, also declaring two passes.
These are progressive codestream updates, not embedded previews or animation.

Each C stream has raw, finite `jxlc`, three indexed `jxlp` boxes and a finite
`jxlc` plus trailing XML wrapper: **8 small inputs**, 2 RGBA16 independent C
oracles. The wrapper splits the raw stream after byte7 and halfway through the
remaining stream, with sequential part indices and the last-part bit. The XML
is inert retained metadata; no parser/network access is involved here. The
[manifest](../wml2/tests/fixtures/review/jxl-progressive-fixtures.json) contains
construction details and SHA256 for every input and oracle. Neither WML2 nor
another invocation of the Rust core generates expected pixels.

C reconstruction explicitly requests relative sRGB and normalized little-endian
RGBA16. The Rust core requests RGBA16 at high precision, full little-endian16.
Actual native maxima are **1 u16 for AC /2 u16 for DC**; the test permits3 u16
for implementation rounding. Final WML2 `image_from_srgb` differs by at most
**1 u8** after independent C16-to8 deterministic rounding (test allows2); alpha
is exact opaque255. This batch does not establish ICC, HDR, transparency,
orientation or animation streaming behavior. Existing tests retain the existing
all-eight-orientation and profile contracts. The core's whole-versus-resumed
exact equality supplements, rather than replaces, the independent C oracle.

## What the actual 0.7.4 API does

Installed published sources, not a libjxl analogy:

- `src/api/input.rs`: `JxlBitstreamInput` provides available-byte estimate,
  vectored read and skip. No sealed/EOF flag. The slice implementation consumes
  its mutable slice; the caller must preserve **unconsumed** input. A zero read
  means currently no input, not an application-level EOF declaration. The
  provided BufReader implementation requires Read+Seek and seeks to the end
  for its estimate; it is not a nonseekable socket interface.
- `src/api/decoder.rs`: processing returns typed `NeedsMoreInput { size_hint,
  fallback }`. Keep the fallback decoder at its exact typestate and supply more
  bytes; the hint is an estimate, not a requested allocation size or finality
  signal. Image/header and frame states both expose `flush_pixels`.
- `src/api/inner/process.rs:183–206`: flush maps `Ok` **or OutOfBounds** to the
  dirty-pixel flag. `frame_info::do_flush` attempts available LF/HF rendering.
  `true` means new pixels were written; it does not mean complete coverage,
  validated entropy/tail, a numbered quality pass or an application final image.
  There is no public valid rectangle/mask or quality-level contract here.
- `WithImageInfo::file_length` is authoritative because the core may over-read.
  `process_trailing_data` is a separate call once all main frames finish. Both
  boundary checking and container completion matter; frame completion alone is
  insufficient. Errors consume the typestate operation and must terminate a
  future wrapper session rather than silently reset it.

WML2's `draw::image_reader<R: BufRead+Seek>` and BinaryReader callback entrypoints
operate on a complete seekable image. `jpegxl::decode` reads and unpacks that
image, performs ICC/frame/TOC preflight, then processes and draws completed
pixels. Its `complete()` turns NeedsMoreInput into UnexpectedEof for that
whole-input contract. `DrawCallback::draw` has no provisional/quality/finality or
revocation event. Reusing it without a new contract would publish ambiguous
success. Whole-input progressive support is already present; this issue concerns
a new resumable/publication contract.

## Executed boundary and ownership experiments

The [trusted helper](../wml2/tests/common/jpegxl_progressive.rs) exposes a growing
window through the actual input trait, retains the fallback typestate and uses
initialized aligned u16 output. It permanently poisons **only consumed input**
after every return. All unconsumed bytes remain intact. This tests the observed
codec-owned state rather than replaying a retained chunk by accident. It caps
fixtures at64KiB, the canvas at32×24, and saved snapshots at16. These fixture
checks are **not a production resource contract**: all original input is cloned
and snapshots are retained for comparison; codec ICC/entropy/workspace is not
independently metered or bounded by this helper.

| Input | Two-chunk boundaries | Bytewise provisional updates | Frame completes before end |
| --- | ---: | ---: | ---: |
| AC raw | 1247 | 2 | 0 |
| AC jxlc | 1287 | 2 | 0 |
| AC jxlp | 1315 | 2 | 0 |
| AC trailing XML | 1350 | 2 | 63 |
| DC raw | 1280 | 4 | 0 |
| DC jxlc | 1320 | 4 | 0 |
| DC jxlp | 1348 | 4 | 0 |
| DC trailing XML | 1383 | 4 | 63 |

All **10,530 two-chunk boundaries**, including empty first chunk and empty last
chunk, produce exactly the same final core pixels and file boundary as whole
input. Every one-byte input schedule also matches. An immediate empty retry
preserves input position, pixels and frame-complete status. The same initial
sentinel buffer is reused: no tested provisional update retains a sentinel, but
that observation is not a universal coverage guarantee. Every tested provisional
update differs from final; successive updates differ too. Four flush updates in
the DC case are not four visible frames or a public four-level quality promise.
A contact sheet of actual LF/AC updates and final output was visually inspected:
blurred low-frequency output converges to the independent final pattern.

Before sealing, waiting is just waiting. The evaluator separately invokes the
whole-input API on every strict prefix. Raw/jxlc/jxlp strict prefixes reject.
For a file with trailing XML, exactly one strict prefix—the complete jxlc box
before XML—already constitutes another valid complete file. A future session
must wait for caller `finish()`; it cannot treat a temporarily exhausted prefix
as the final original file or promise that every shorter prefix is malformed.

For each split, the development exposure gate refuses to expose more bytes
while paused and verifies that input position/exposure, snapshots and pixels
remain unchanged; then normal processing resumes. A separate actual decoder is
created, progressed to that boundary and dropped to model Abort **between
calls**. These are caller-policy/ownership experiments. They do not implement a
shipping backpressure/Abort API, interrupt a process call, prove allocator peak
or qualify a queue of arbitrary consumer-held snapshots.

## EOF and late-error observations

Fourteen deliberate envelope negatives cover raw/boxed truncation, appended raw
junk, an initial jxlp index1 instead of0, incomplete trailing box header and
trailing payload length exceeding available bytes. The current WML2 whole API
rejects all14; the negative Rust probe creates **zero pixel output files**. Actual
callback regression tests also verify absent canvas/pixels/metadata on malformed
envelopes and input/pixel/workspace/metadata/frame-limit errors.

The C oracle rejects the six truncations, but accepts appended raw junk,
misindexed parts and the four incomplete trailing-box cases at its image-only
subscription level. This is recorded, not used to certify strict container
admission. C may write reference pixels for those accepted cases; those files
are separate from the negative Rust output prefix. The known negative constructions and WML2's stricter envelope
contract are explicit; C final pixels are the independent pixel oracle.

In the malformed-trailer cases the Rust core has **already completed a frame**
and reports the earlier jxlc boundary. Its explicit trailing-data call then
returns NeedsMoreInput with hint1 after consuming available tail bytes. With a
valid XML tail it completes, while `file_length` still identifies the earlier
main-image boundary. With raw junk it can report trailing-data complete even
though the authoritative raw codestream length is shorter. Therefore neither
"main frame complete" nor a standalone trailing-data completion substitutes for
sealed, structurally validated input and authoritative raw boundary checks.
This is an API-contract observation, not a newly claimed production bypass.

## Concrete additive contract proposed, not exported

Introduce an explicit owned `JxlSession` companion, leaving current still/EX/CMS
entrypoints and callbacks unchanged. Start with a single still image and the
existing admitted channel/profile/depth/orientation restrictions. No animation,
embedded-preview extraction, HDR or new encoder capability is implied.

- `feed(bytes)` returns accepted-byte count and a state/event; the caller owns
  the unaccepted suffix. A paused consumer causes zero further admission or
  decode work. Size hints never override budgets. Empty feed is not EOF.
- `poll()` yields NeedInput, an optional **owned provisional snapshot** with
  generation and explicit provisional status, or terminal error. Initially
  publish complete initialized canvases, not invented validity rectangles or
  pass numbers. Keep at most one pending snapshot; cap consumer-held ownership
  by a quota/acknowledgement contract before allowing more. Failed/cancelled
  generations are invalidated; they never become successful final images.
- `finish()` seals input exactly once. Resolve pending NeedsMoreInput as
  truncation, validate all container parts/metadata/trailers and authoritative
  codestream boundaries, finish color conversion, then commit exactly one owned
  final result. Subsequent feed/finish after seal/error/Abort rejects. Frame
  completion before finish remains provisional. A late error revokes the
  provisional generation and returns no final image.
- Reuse #32's cancellation/error type and fallible runner, checking at admission,
  resume, snapshot/CMS chunks and before final commit. Abort drops owned state
  and forbids later events. Cancellation stays distinct from malformed input,
  unsupported capability and limit errors. No wall-clock bound is claimed:
  serial header/ICC/TOC parsing, transform creation and a running codec task
  remain noninterruptible until their next supported safe point.

Cumulative accepted input includes bytes already consumed/discarded; never
replenish that budget as a pending queue shrinks. Use checked arithmetic and
admit before allocation for raw/expanded/aggregate metadata, indexed parts,
headers/ICC, each coded LF/layer/TOC and conservative codec/reference/scratch
storage. Fixed limits from #37 stay conservative; no caps increase here.
A live ledger must include pending input, retained parser/assembled bytes,
codec planes/reference/HF/LF scratch, retained metadata/profiles, aligned output,
one pending owned snapshot, acknowledged-but-retained snapshots, CMS working
space and the final collector. For this small RGBA16 example each full canvas
alone is6144 bytes, final RGBA8 is3072; those are not decoder peak estimates.
One-copy assumptions are insufficient. Drop the core to release it on Abort;
reset/reuse needs its own retained-capacity accounting.

**Admission is the present blocker.** WML2's existing preflight needs the full
assembled codestream for bounded ICC expansion and all-coded-frame geometry/TOC
planning before entering the codec. Feeding untrusted chunks directly into the
trusted helper bypasses that foundation. Either develop a bounded resumable
preflight that admits each section before the core can allocate it, or use a
bounded whole-input buffering facade and accurately label it as such. The latter
alone does not satisfy #47's partial-output requirement. Do not expose this
helper as a safe API or remove security caps to claim streaming support.

## Upstream0.8, qualification and remaining acceptance

The installed published0.8.0 API has `Event::NeedMoreInput`, BasicInfo,
FrameHeader, **FrameComplete** and separate **Complete** for frames plus trailing
boxes (`src/api/mod.rs:35–47`). Its decoder docs say reset retains internal
buffers/scratch; dropping releases them. These source facts make the distinction
clearer but do not solve admission. The #40 [migration evaluation](https://github.com/mith-mmk/wml2-on-rust/pull/54)
remains the authority on unavailable private preflight APIs and migration hold.
This batch runs0.7.4 and does not claim a0.8 streaming qualification or bump the
pin.

Required before production integration:

1. Incremental bounded container/ICC/frame/TOC planning and aggregate live ledger
   including split extended/open-ended boxes, indexed-part ordering, brob/Exif/XMP,
   huge/overflow/truncated headers, excessive fragments/metadata/quality work and
   repeated feeds whose current queue is small but cumulative input exceeds cap.
2. Feed/finish/error state machine with no final publication before sealed
   validation, retained unconsumed bytes, no-input retries, adversarial malformed
   entropy and late tails, no output resurrection after Abort/error/cancel.
3. Owned snapshots with initialized/validity semantics, generation invalidation,
   CMS profile/alpha/orientation fidelity, queue acknowledgements and hard
   consumer-held-memory bounds. Pause/resume must stop actual input admission,
   decode and allocation; test slow/aborting consumers, not just this gate.
4. Actual incremental cancellation at all meaningful supported codec/CMS stages,
   distinct limit/cancel precedence, no cancelled final result; qualify precise
   noninterruptible sections without importing libjxl interruption assumptions.
5. Extend this independent small oracle matrix to lossless/responsive modular,
   ICC Gray/RGB, alpha/high precision, all orientations and larger/permuted groups,
   every byte split and repeated random schedules; final pixels match both the
   existing whole API and independent reconstruction/CMS. Qualify actual WASM
   and32-bit runtime retention/overflow behavior for the new session itself.

Executed local/CI evidence is recorded in
[jpeg-xl-progressive-evaluation.json](jpeg-xl-progressive-evaluation.json).
The optimized Cargo test repeats every split against frozen independent pixels
without C SDK dependencies; a dedicated CI step runs it. Debug platform/feature
matrices sample16 header/box/tail splits per input plus every-byte feeding.
Native/full, minimal and disabled regression results are recorded alongside the
independent replay. No new streaming WASM runtime claim: existing shipping JXL
WASM checks remain separate. There are no production decoder changes and no
production speedup, regression benchmark or peak-RSS claim.

The isolated #32+#62 composition at
`e02f5c7c9df5e43ef2a434267ccc5a90bd9b0538` successfully decodes all8 progressive
files to independent sRGB references and rejects cancellation at **220
deterministic checkpoints**, with8 input-limit/cancel precedence cases.
[The checker](../wml2-test/scripts/jpegxl_progressive_cancel_check.rs) is compiled
against that combination's actual wml2 rlib. This verifies buffered cancellation
and unchanged preview composition only. It does not add incremental cancellation,
qualify latency, or rewrite either public PR branch. No merge, approval, release
or issue closure is performed.
