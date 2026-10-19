# JPEG 2000 ICC acquisition and sample identity

JP2 `colr` method2 now selects structurally valid Gray/RGB ICC device coordinates
through the own `icc-profile`0.0.7 container parser. This does not compile or
apply an ICC transform. Direction-specific unsupported tags remain retainable;
malformed containers and other device models reject before image callbacks.
The `jpeg2000` feature enables the already declared own ICC dependency, with no
new third-party runtime dependency.

The existing first-understood colour-box policy remains. Later boxes cannot
overwrite selected ICC or enumerated colour. Unregistered JP2 Part1 methods,
including method3, are ignored rather than guessed from their payload. This
matches the [OpenJPEG2.5.3 JP2 parser](https://github.com/uclouvain/openjpeg/blob/v2.5.3/src/lib/openjp2/jp2.c#L1365).
JPX/unrestricted colour support is not claimed.

Selected bytes are always `Source ICC Profile`. A compatible `ICC Profile`
describes RGB/palette-expanded ordinary output or native Gray/RGB EX samples.
Gray replicated into RGBA ordinary output remains source-only. Shared EX
storage conversion demotes native Gray ICC when converting to RGBA; numeric
precision conversion alone does not apply ICC. The same-format owned shortcut
and native resolution-reduction APIs retain the same metadata as callbacks.

Source copies/keys and active copies/keys are charged cumulatively before
allocation/callback publication. Structural parsing uses carrier-bounded tag
storage, the own parser's4096-tag/64MiB caps, metadata limits and checked
temporary workspace. Retention never requires a transformable TRC/LUT route.
Abort at either new profile callback stops before initialization and terminates
once; caller errors propagate unchanged. Existing codec pixel/alpha/geometry
support restrictions remain independently enforced.

## Validation

Five new integration tests cover Gray/RGB and alpha, both MCT choices, opaque
unsupported transform tags, first-colr selection, unknown methods, malformed and
mismatched profiles, aggregate metadata boundaries, direct/guarded callback
Abort/errors, owned/callback EX parity, storage conversion and reduced output.
All5pass. Existing JPEG2000,EX,EX-colour and reduced targets pass67tests.
Counts refer to Rust test functions; they include the existing independent
OpenJPEG/PGX corpora and all400reduced EX inputs.

Five OpenJPEG-encoded inputs with colr-only ICC insertion preserve every
codestream byte. Ordinary output exactly matches the unprofiled baseline and
independent Pillow/OpenJPEG source-coordinate pixels, including alpha. Source
profile bytes and active applicability match. The paired rasters were displayed
without corruption. Native12/16-bit and reduced samples remain exactly equal
to the existing independently referenced controls. No new WML2-encoded input
or WML2-generated colour reference is used.

## Local performance evidence

Initial separate-process timing was unstable (including large changes in both
directions for unchanged controls). The final comparison links both saved
release libraries into one executable and alternates their order each iteration
on the same source buffer. Each case uses3processes pinned toCPU0 and1100pairs,
with first100omitted:3000samples per variant. No concurrent builds/tests run.
All allocation counts, allocated bytes and pixel hashes match between variants.

| Existing path | Before | After | Median paired change |
| --- | ---: | ---: | ---: |
| gray8 | 61.34µs | 61.09µs | -0.16% |
| rgb8-mct | 230.68µs | 227.22µs | -2.53% |
| ex-gray16 | 49.03µs | 48.87µs | +0.40% |
| ex-rgb16 | 86.59µs | 85.10µs | -2.09% |

The new ICC route has no successful pre-change counterpart. Absolute local
medians (300warm samples/case) are below; these are small-fixture setup/decode
costs, not cross-machine throughput guarantees. Separate-process sensitivity
and all first-pass CSVs are retained alongside the final paired measurements.

| ICC input | Median |
| --- | ---: |
| gray8 | 48.75µs |
| rgb8-mct0 | 138.96µs |
| rgb8-mct1 | 145.11µs |
| rgba8-general | 273.02µs |
| grayalpha8-general | 184.54µs |
