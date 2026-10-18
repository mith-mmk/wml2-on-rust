# GIF callback cancellation regression

PR #13's callback guard remembered a returned Abort without converting it to
its internal successful-cancellation signal for GIF. GIF emits Continue after
its first frame, then Next before later frames. A callback aborting at the first
boundary could therefore receive later verbose callbacks or encounter
`next before successful initialization or after Abort` at the next boundary.
This occurred with `image-buffer-ex` disabled too.

## Direct reproduction and fix

Identical public-API probes with the same three-frame 1x1 GIF return:

| Source | Result | Next callbacks | Draw callbacks | Finalization |
| --- | --- | ---: | ---: | ---: |
| main 5b12608a | Ok(None) | 2 | 1 | 0 |
| PR #13 a1bcf033 | Err(next before successful initialization or after Abort) | 1 | 1 | 0 |
| Corrected | Ok(None) | 1 | 1 | 1 |

The guard now routes GIF87a/89a Abort responses through its existing typed
cancellation path. Cancellation stops immediately, performs no later input or
callback work, and finalizes once. Genuine initialization/draw/next/metadata/
verbose errors, malformed input before cancellation and finalization errors
remain errors. An ordinary decode retains all animation pixels. The direct
low-level GIF decoder keeps its existing callback policy; this corrects guarded
public dispatch, without expanding EX animation support.

The self-contained fixture in `wml2/tests/gif_callback_abort.rs` contains explicit
LZW clear/index/end streams, not output from WML2's encoder. It covers default
and explicit-limit public entry points, debug off/on, first and later frame
boundaries, GIF87a, a missing trailer/malformed suffix after cancellation and
real errors. `callback_abort` additionally exercises every callback using a
larger encoded animation. CI explicitly runs both GIF-only and GIF+EX cases.

Commands:

```sh
cargo test -p wml2 --locked --no-default-features --features gif --lib --tests
cargo test -p wml2 --locked --no-default-features --features gif,image-buffer-ex --lib --tests
```

Main/before/fixed use separate Cargo target directories. An initial shared
cache comparison interfered between checkouts; that run was discarded and the
isolated runs above establish the reported source-specific outcomes.

## Validation

Isolated minimal:28passed; GIF-only:40passed; GIF+EX:75passed. Release workspace
(default codecs, AVIF encoder, EX and high-bit-depth):461passed, including
15doctests, with14existing ignored tests. GIF-only/GIF+EX/workspace Clippy,
formatting, six release-metadata tests and version-note validation pass. Existing
Clippy warnings remain; the added GIF tests/predicate produce none. Counts across
configurations overlap. Peak sampled cgroup memory for supervised checks was
3.34GiB, with zero OOM events and a two-job Cargo limit.

## Independent pixels, display and performance

Pillow-generated 1x1, 64x64 and 512x512 GIFs contain three frames each. Every raw
RGBA byte of the base canvas and all stored animation frames matches Pillow's
independent decode before and after the correction, across all 108 observations.
The [display comparison](gif-abort-display.png) shows reference/before/fixed
frames; it was visually inspected.

Release binaries use the same harness and GIF feature, 12 balanced blocks,
rotating original/fixed/identical-original-control order and one pinned CPU.
Timings cover ordinary full animation decoding, including allocations; pixel
hashing and file output are outside the timed loop. No other build or timing
job ran concurrently.

| Size | Loops/process | Fixed vs original median | Identical-binary control vs original |
| --- | ---: | ---: | ---: |
| 1x1 | 100000 | +0.039% | +0.523% |
| 64x64 | 1000 | -2.392% | -1.258% |
| 512x512 | 60 | -1.153% | +0.676% |

No material regression was observed in these cases. One host/CPU and ordinary
runtime variability do not establish a universal speed or no-regression claim.
The [validation record](gif-abort-validation.json) retains all 108 raw samples,
source hashes, exact counts/commands, the harness source and measurement limits.
Builds use two Cargo jobs; supervised checks stop their own process group above
12 GiB of the 16 GiB cgroup budget. AVIS and separate iOS work remain outside this
correction.

## Shared stopped-state follow-up: BMP

An additional direct probe found the same readiness classification regression
in public BMP decoding: after Abort on the first of two scanlines, main5b12608a
returned Ok(None), two draws and one termination; the GIF-corrected cb7a000
returned Err(draw before successful initialization or after Abort), one draw and
no termination. The corrected guard returns Ok(None), one draw and one termination.

Only the existing invalid-readiness branches change: a remembered callback Abort
returns the private cancellation marker. Never-initialized and failed
initializations retain their real error. Unit checks distinguish both error
classes with handle_abort false/true; readiness, format, bounds and accounting
checks remain in force. The ordinary successful draw/next path gains no new
condition. This is not a blanket conversion of decoder errors into success.

The public BMP regression independently builds valid24-bit rows at heights1/2/8,
checks normal full-image pixels, default/explicit-limit APIs, debug0/1 and real
termination failures (including Abort on the only/last row). BMP's existing
post-draw metadata delivery is retained. The immediate GIF cancellation promise
above is not generalized to every legacy decoder. CI runs BMP with EX off/on.

All108paired BMP64/BMP512/GIF64 observations match Pillow pixels byte-for-byte.
Fixed-versus-before medians are+0.310%/-0.845%/-1.799%; identical-binary control
shifts are-2.802%/-0.520%/+0.079%. These are observations on one host/CPU, not a
universal throughput promise. [Raw evidence and exact commands](callback-stop-validation.json)
and the [actual decoded comparison](callback-stop-display.png) are retained;
the display was visually inspected.

Follow-up checks pass: minimal28, BMP29, GIF40, BMP+GIF+EX76; release workspace462
(including15doctests), with14existing ignored tests. BMP-only/BMP+EX/workspace
Clippy, fmt, six release metadata tests and current-version notes pass. These
configuration counts overlap. The stronger initialization tests require a real
error after failed initialization and a private cancellation only after Abort.
