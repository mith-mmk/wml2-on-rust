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
