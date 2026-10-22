# CI selection and external tools

Pull requests run `checks` once. Branch pushes run it only on `main`; feature
branches without a PR do not automatically run CI. `workflow_dispatch` remains
available for a full run. The release workflow's reusable call also runs the
full suite. No workflow-level path filter is used.

## Conservative selection

`select_ci.py` compares the PR head to its merge base with the target commit.
It does not use rename detection, so both old and new paths must be safe.
Missing history, malformed manifests, empty/unknown diffs and non-PR events
select the full suite.

Only isolated AVIF, JPEG2000 and JPEG XL implementation/test paths currently
have narrow selections. Multiple codec changes take the union. Shared drawing,
limits, metadata, colour, highres core, feature definitions, submodules,
fixtures, scripts/workflows, other codecs and unknown paths select everything.
Documentation-only changes keep the common checks. Executable documentation
scripts are not treated as prose.

An AVIF dependency-only update is recognized by parsing both Cargo manifests
and lockfiles, not merely by their paths. Only the existing registry AVIF
version requirements may differ in `wml2/Cargo.toml`. In `Cargo.lock`, only the
two AVIF packages' versions/checksums may differ; package sets, sources,
dependency edges, shared buffer and transitive package changes force a full
run. Optional flags, feature declarations, Git/path dependencies and other
manifest edits likewise force full coverage.

Both Rust 1.91 and stable remain in every selected feature configuration.
Default/minimal, EX, high-bit-depth and colour-management baseline tests always
run, along with feature-compatibility compilation, Windows/macOS basic tests,
WASM/i686 compilation, formatting and release metadata checks. Codec-specific
OS steps and TIFF/JPEG XL jobs follow the same selection. JPEG2000 tests are
not removed or ignored: they run on JPEG2000/shared/unknown changes and full
runs. The compatible-feature check still compiles all codecs.

For the existing PR #23 diff, the plan contains 24 Linux test combinations,
12 requiring AVIF tools, and 33 executing jobs overall (including selector and
aggregate). A full run keeps all original 46 Linux combinations and has 58
jobs including the two new control jobs. The old PR + feature-branch push
combination launched 112 jobs. These counts are not runtime guarantees.

## FFmpeg

Previously every Linux test job installed FFmpeg and generated AVIF oracles,
even without the AVIF feature. The selector follows the manifest feature graph
(including defaults/transitive activation) to mark AVIF jobs. Only those jobs
prepare FFmpeg and bootstrap the existing oracle fixtures.

`check_ffmpeg.py` tests actual libaom AVIF encode, ffprobe and decode, not just
binary existence. A working runner installation is reused. Otherwise apt
installation has bounded time/retries and is followed by the same functional
check. A failed install/check/bootstrap fails the job; it is not converted to
a skipped oracle or a successful test. Existing oracle generation is retained.
No runner image, secret or permission setting is changed.

## Required-check contract

`Required checks` always evaluates prerequisite results. Selected jobs must
succeed; failure, cancellation, missing selection output, and unexpected skips
fail the aggregate. Only explicitly unselected TIFF/JPEG XL jobs may skip.
The selector itself must succeed. Existing matrix check names are preserved.

Branch protection is not edited. The previously observed main configuration
had no required checks. If maintainers later enable protection, use the stable
`Required checks` context rather than individual dynamically absent matrix
entries. Existing externally configured matrix requirements would need an
explicit administrator migration; adding the aggregate does not silently
rewrite those settings. A previous management API request returned 403; no
attempt was made to bypass it.

## Local validation

```sh
python3 -B -m unittest discover -s .github/scripts -p 'test_*.py'
python3 -B .github/scripts/check_ffmpeg.py
python3 -B .github/scripts/select_ci.py --event pull_request --base BASE --head HEAD
actionlint .github/workflows/ci.yml .github/workflows/release.yml .github/workflows/extended-checks.yml
```

Unit tests cover shared/transitive dependency changes, source/feature changes,
missing history, malformed TOML, rename old paths, feature closure, selected
matrix coverage, and failure/cancellation/skip propagation. No codec product
code or tests were removed. This CI-change PR itself selects the full suite
because it changes workflows/scripts. Its hosted run is not awaited or polled.
