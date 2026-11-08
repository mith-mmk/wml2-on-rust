# JPEG XL SDR WebAssembly and Viewer playback validation

Validated on NERO / Windows x64, 2026-11-08, Rust 1.99.0 and Node 24.22.1.
This supplements the native codec tests with actual WebAssembly execution and
actual WML2Viewer rendering. No CI/CD run was awaited.

## Source and isolation

- WML2 0.0.42, local `fix/jxl-xyb-animation` worktree, based on
  `042a941114781d4d17f7e4bbf7b5d09574847f61` plus the SDR changes in this PR.
- WML2Viewer 0.0.27 source archive from
  `909b636faa0e00103a6cf3388f21ce511330ca0b`.
- An isolated Viewer copy changed **both** its root and `wml2viewer-core`
  WML2 path dependencies to the current local PR worktree. Cargo's build output
  and lock file confirmed WML2 0.0.42 from that path. No published crate or
  unmerged remote Git dependency was substituted.
- No Viewer GUI, renderer or animation-scheduler source changes were made.
  System, Susie and FFmpeg fallback plugins were disabled in dedicated test
  configurations. Each sample occupied a separate test directory.
- All build/work roots begin with `.test`. The original Viewer source, user
  sample files and default Viewer settings were not changed. Test windows were
  closed after inspection.

The [machine-readable report](jpeg-xl-sdr-playback-validation.json) records
decoder/test source SHA256s, the WASM and Viewer executable hashes, all test
results, load timings, screenshot hashes and comparison measurements.
After the final successful run, one decoder API doc comment was clarified to
say **output** ICC; executable decoder code did not change. The JSON also
records the post-comment source hashes so this difference is explicit.

## Actual WASM execution: 13/13 passed

From the WML2 repository root:

```powershell
python wml2/tests/fixtures/review/run_jxl_sdr_wasm.py --offline
```

Prerequisites are the installed `wasm32-unknown-unknown` Rust target and Node.
Omit `--offline` if dependencies are not cached. `--work-dir` accepts an explicit
directory whose basename starts with `.test`.

The runner copies the test bodies from `jpeg_xl_xyb_animation.rs`,
`jpeg_xl_gray_animation.rs` and `jpeg_xl_xyb_icc_animation.rs` into a generated
standalone cdylib. It removes only the test-harness attribute, makes the selected
functions callable, and resolves fixture paths. It builds with `jpeg-xl`,
`image-buffer-ex` and `color-management`, then Node compiles and instantiates the
actual WASM module and calls every exported test selector. There are no imported
host decoder functions. A fresh instance is used for each test, and a trap or
failed assertion makes the runner fail. Source hashes and results are saved in
`.test-jxl-sdr-wasm/report.json`.

All 13 bodies passed, including:

- Every frame of the 48-frame XYB icos4d and 60-frame XYB spline samples against
  independent libjxl RGBA8 references, RGB tolerance 1/255 and exact alpha.
- Independent full-precision XYB RGBA16 oracle checks, native and EX output,
  RGB tolerance 1/65535 and exact alpha.
- Gray8, Gray+alpha8, Gray16 and Gray+alpha16: every native and EX sample exactly
  matches the libjxl oracle; common and managed output, profile tagging, timing,
  limits, truncation and cancellation tests also pass.
- XYB RGBA16 with distinct embedded and reconstruction ICC profiles: native,
  ordinary, EX and managed output agree with the independent reference, and
  output/source profile metadata remains distinct.

This is **WASM decoder execution in Node**, not a `cargo check` result and not a
claim about a browser's native JPEG XL implementation.

## Viewer-core and actual desktop playback

The native Viewer-core harness called `wml2viewer_core::image::decode` using the
same isolated Viewer source. It compared every decoded RGBA8 frame against the
independent checked-in libjxl references, checked the poster against frame zero,
and asserted dimensions, frame count, loop count and every frame delay.

| Fixture | Frames | Delays | Maximum raw RGBA8 error |
|---|---:|---|---|
| XYB icos4d, 128x128 | 48 | 50 ms each | `[1,1,1,0]` |
| XYB spline, 320x320 | 60 | 20 ms each | `[1,1,1,0]` |
| XYB RGBA16, 32x24 | 3 | 50/90/130 ms | `[1,1,1,0]` |
| XYB ICC16, 32x24 | 3 | 50/90/130 ms | `[1,1,1,0]` |
| Gray8, 32x24 | 3 | 50/90/130 ms | `[0,0,0,0]` |
| Gray+alpha8, 32x24 | 3 | 50/90/130 ms | `[0,0,0,0]` |
| Gray16, 32x24 | 3 | 50/90/130 ms | `[1,0,1,0]` |
| Gray+alpha16, 32x24 | 3 | 50/90/130 ms | `[1,0,1,0]` |

All eight fixtures were then opened in the actual **release** WML2Viewer GUI.
Multiple captures showed changing frames, including the small Gray and alpha
samples. The icos4d sample also successfully reloaded with F5. Its initial
release decode took 1,587 ms and reload took 1,564 ms in the Viewer log. Spline
decoded in 883 ms. These are recorded smoke-test timings, not a benchmark.

Playback was compared to meaningful references, not merely a non-black window:

- Icos4d captures match different libjxl frames 29 and 43; the reload capture
  matches frame 3. Spline captures match frames 15 and 31.
- For each 32x24 sample, the centers of 709 enlarged source pixels were compared
  against all three independent reference frames composited over the configured
  black background. The cursor region and tile edges were excluded. Gray8,
  Gray+alpha8 and Gray16 selected-frame samples match exactly. Gray+alpha16 has
  at most one RGB unit of screenshot error. XYB RGBA16 and ICC16 have mean RGB
  error below one unit and 99th-percentile error four units after JPEG capture.
- Captures identify all three frames for XYB RGBA16, ICC16, Gray8, Gray16 and
  Gray+alpha16, and two distinct frames for Gray+alpha8. Sequences such as ICC16
  `1,2,2,0` and XYB RGBA16 `2,2,0,1` demonstrate actual loop wrap.
- The raw all-frame comparisons above are the precision evidence. Resizing and
  lossy JPEG screenshots are additional visual/compositing evidence, not a
  replacement for those comparisons.

The isolated build and launch commands were:

```powershell
$env:CARGO_TARGET_DIR = "$qa/.test-viewer-target"
cargo build --offline --release --manifest-path "$qa/.test-viewer/Cargo.toml" --bin wml2viewer
# Run separately for icos4d, spline, rgba16, gray8, grayalpha8, gray16, grayalpha16, icc16:
& "$qa/.test-viewer-target/release/wml2viewer.exe" --config "$qa/config-icos4d.toml" --log "$qa/.test-icos4d/icos4d.jxl"
```

`$qa` denotes the isolated `.test-sdr-playback-20261108` work root. The local
evidence bundle contains the full native harness (`.test-core/main.rs` and
`Cargo.toml`), configurations, `viewer-core-report.json`, screenshot comparison
scripts/results, 30 playback captures, reload capture and Viewer JSONL logs.
Screenshots and machine-local source paths are not required to build the crate.

## Explicit limits and failed attempt

- The first **debug** Viewer run hit its existing five-second load timeout for
  icos4d. It was not counted as a playback pass. The subsequent unmodified
  release GUI loaded, played and reloaded successfully; the timeout was not
  disabled or lengthened.
- Every new GUI sample encodes loop count zero. Infinite looping was observed;
  finite-loop stopping was not exercised in this GUI run.
- Exact encoded ticks and Viewer-core delay values passed. The screenshot
  cadence does not measure frame-pacing accuracy, display refresh behavior or
  dropped frames; the Viewer's existing minimum-delay scheduler is unchanged.
- No native Chrome/JXL comparison or universal Chrome compatibility is claimed.
  HDR, floating-point samples, unsupported ICC profiles, auxiliary-channel
  display and exhaustive corpus validation remain outside this result.
