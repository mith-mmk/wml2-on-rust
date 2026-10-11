# JPEG2000 continuation: review, WASM execution and next profile

Resume point: `d1e128f01812d591e67f9811d8dab0c5c4eb85fb` on
`codex/jpeg2000-rgb-reimplementation-20261011`. Normal Git remote read/fetch
matched that SHA; stable `jpeg2000` remained at `7043bf083203234c1193cf1c22a70ab61d6dc3df`.
Repository AGENTS.md was read; the workspace `.agents` directory was empty and
there were no repository `.agents/skills` instructions. Working folders use
`.test*`. No old ZIP investigation or GitHub app approval flow was resumed.

## RGB/API integration review

An independent correctness review found no concrete defect in the supported
RGB subset or its integration into the existing memory/callback APIs. Three
fixed regression gaps were filled: exact expected width/height assertions;
neutral, monochrome and flat RGB images (empty packets and zero RCT chroma);
and active component packets with distinct QCC maximum bit-plane precision.

The RGB generator now has 46 sources / 184 encodings. Three additional 1×1
QCC fixtures increase Mb from 9 to 10 for component 0, 1 or 2, inserting a zero
into the corresponding unary zero-plane tag to keep the encoded depth/passes
unchanged. All three decode to RGB (31,83,197) with OpenJPEG and Rust; ignoring
QCC fails the full-pass check. `generate_jpeg2000_qcc.py` reproduces them, and
`jpeg2000-qcc-manifest.txt` records their hashes. OpenJPEG remains only a fixture
encoder/pixel oracle, with no production implementation dependency.

The JPEG2000-only native unit/integration run passed (19 + 17 tests); the new
distinct-QCC integration test passed separately. The independent review's
unchanged baseline run also passed all 36 unit/integration tests. Ordinary
release workspace regression with default codecs plus JPEG2000 passed 346
tests, 0 failed, 12 existing ignored; formatting and diff checks passed.

## Actual WASM execution

`wml2-test/scripts/verify_jpeg2000_wasm.py` builds a disposable cdylib against
this repository's wml2 crate, embedding the existing synthetic source/encoding
bytes. It instantiates and executes the actual wasm32-unknown-unknown module
with Node.js v24.19.0 using no host imports. It does not change production API,
crate types, output format, dependencies, or add unsafe decoder code.

Each fixture checks public-API output dimensions and every RGBA byte; exact and
one-below input/pixel/expanded/frame/metadata limits; finite limit scope
restoration; callback output and init/draw Abort; and truncated input rejection
before callbacks. Minimal-feature execution passed all original 330 encodings;
default-codec-plus-JPEG2000 execution passed 366 encodings, including the added
neutral/monochrome/flat cases. Native QCC cases were added after those executions.
Module hashes, memory sizes and checks are in `jpeg2000-wasm-runtime.json`.
This is WASM execution in Node, not a browser UI or graphics integration test.

```sh
# With Rust 1.91+, the wasm32 target installed, and Node on PATH:
python3 wml2-test/scripts/verify_jpeg2000_wasm.py
python3 wml2-test/scripts/verify_jpeg2000_wasm.py --default-features
```

## Next useful profile step

Code-block geometry is the next dependency to remove: COD, packet partitioning
and the otherwise dimension-independent Tier-1 loops currently force 16×16,
which rejects common default 64×64 OpenJPEG encodings. The next implementation
will read validated block dimensions from COD, use them in partition/tag-tree
geometry, and retain the legacy 16×16 fast path and existing budgets. Tile/layer,
wavelet, progression, precision and color constraints remain independently
checked. Multi-layer partial-pass continuation, 9/7, multiple tiles, other
progressions, component subsampling, signed/high-bit-depth output and general
JP2 color/palette/ICC handling remain later work.
