# Release maintenance

`docs/releases/<version>.md` is the canonical release body from 0.0.33 onward.
README history links to it; do not maintain a second summary there. Older README
entries are retained as historical records. GitHub Release bodies are copies of
these files. Write user-visible summaries for direct commits as well as PRs.

1. Validate changes in the affected lower-level libraries first. Describe purpose
   and acceptance conditions in an Issue; record user impact, compatibility,
   commands/results and related Issues in the PR template.
2. Update `wml2/Cargo.toml` and `docs/releases/<version>.md` together before a new
   version is tagged. Include `Changes`, `Compatibility`, and `Pull requests`
   sections with meaningful bullets. List merged PR links in the actual previous
   tag..new tag range; explicitly state when there are none. Review direct commits
   and dependency updates too. Do not use TODO/TBD placeholders in release bodies.
3. Add a version link to both README histories. Run:
   `python -B -m unittest discover -s .github/scripts -p 'test_*.py'` and
   `python -B .github/scripts/release_notes.py`. Inspect `git log` and `git diff`
   between the exact tags, including submodule pins. Main and feature branches
   may contain unreleased work; never copy that into an older version's notes.
   Run `cargo publish -p wml2 --dry-run --locked` and inspect the actual package:
   it must contain a root `README.md`, and its normalized `Cargo.toml` must
   select that file. Compare the packaged README with the repository source;
   a README elsewhere in the repository or test fixtures is not sufficient.
4. After review and separate publication authorization, merge and tag the intended
   commit as `v<version>`. The tag must contain these workflows, script and notes.
   The release workflow calls the same CI at that commit (all existing matrices,
   platform, compile, format and TIFF oracle jobs), validates tag/SHA/version/body,
   builds examples on three OSes, and only then creates the Release from the notes
   artifact. Only the final creation job has `contents: write`; PR CI has read
   permission. Scheduled/manual extended checks remain supplementary.
5. Do not move release tags. The publish job rechecks the remote tag SHA, but an
   external tag move after that check cannot be made atomic with Release creation.
   An existing Release makes creation fail; editing it requires a separate,
   explicitly authorized operation. Failed CI or missing notes/assets prevents
   publication. This workflow creates GitHub Releases, not crates.io publications.
6. Adopt the validated library in affected viewers afterward. Record downstream
   checks when relevant, not for every documentation/workflow change. Viewer
   version display is `<viewer version>-lib<Lib version>`.

## Existing 0.0.33 draft

The `0.0.33.md` body is a reviewed local draft based on
`v0.0.32...v0.0.33`; it does not describe later main commits. The existing v0.0.33
tag predates these workflows and notes. Merging this change does not rerun or
retroactively gate that release. Do not retag or blindly dispatch a release from
main (which contains later AVIF changes). After separate approval, validate the
existing tag's source in an isolated checkout and manually apply the reviewed body
to the existing Release. Viewer adoption remains separate work.

The script requires Python 3.11+ (CI pins 3.12); it rejects missing/empty notes,
wrong version headings, unfinished sections, malformed tags, Cargo/tag mismatch,
and checkout/tag/event SHA mismatch. Human review still establishes factual
accuracy and completeness of the PR list.
