# Native JPEG2000 resolution reduction (unreleased)

This continuation starts at memory PR #17 head
`bc6c1182c7478c385b10296af71c996c4f4f3ab7` on
`codex/jpeg2000-reduced-decode-20261017`. WML2 remains 0.0.36.

Existing entry points and `DecodeOptions` / `DecodeLimits` struct literals are
unchanged. Three codec-specific opt-in functions accept a discarded DWT level
count and explicit limits:

- `jpeg2000::decode_with_reduction(reader, callback_options, reduce, limits)`
  returns ordinary RGBA8 for the reduced 8-bit profile.
- `jpeg2000::decode_ex_with_reduction(...)` retains the native high-depth EX
  source declaration and bytes.
- `jpeg2000::image_from_ex_with_reduction(bytes, storage, precision, reduce,
  limits)` returns owned EX storage, moving a matching native canvas or using
  guarded Exact/Round conversion for another format.

Explicit zero delegates the established full-resolution dispatch, including
its broader legacy acceptance. Existing calls with no reduction remain full
resolution. Nonzero reduction discards the finest `reduce` DWT levels; it does
not decode a full image and resize it.

## Reduced profile and geometry

Nonzero reduction accepts uniform unsigned full-resolution Gray/RGB/explicitly
declared straight RGBA at 8 bits (ordinary RGBA8), or 12/16 bits (Gray12/16 or
RGBA12/16 packed little-endian EX bytes). Classic reversible 5/3 and bounded
9/7/ICT are supported; existing 9/7 maximum six levels remains. Colour and alpha
are never premultiplied. RGB adds opaque alpha at the source bit depth.

Signed/mixed precision, subsampling, HT, encoded maxshift ROI, palette/CMAP,
sYCC/ICC and implicit/premultiplied/per-colour alpha are outside this reduced
profile and are explicitly rejected. Multi-tile-part PPT is also explicitly
unsupported: the existing full-resolution per-part parser is preserved. Single-
tile-part PPT and multi-part PPM/inline headers are independently covered.
The existing strict EX channel declaration
rules also apply to nonzero 8-bit reduction. MCT requires equal colour-component
DWT level counts. Each used tile/component must have at least `reduce` levels;
values above 32 and an empty global result are rejected before callbacks.

For scale `f = 2^reduce`, output bounds are
`[ceil(x0/f), ceil(x1/f)) × [ceil(y0/f), ceil(y1/f))`.
The width is not generally `ceil((x1-x0)/f)`. Each component/tile uses the same
endpoint rule; a tile whose reduced output collapses still has all its packets
consumed and checked. Packet/block ordering uses original reference-grid
geometry. Reduced coefficient planes retain only the required low resolutions,
original quantization entries and `levels-reduce` inverse lifting stages, with
reduced absolute origins. DC shift, colour transform, final rounding/clamping
and source-depth packing then follow the established sample rules.

## Validation and resource contract

Every packet header, tag tree, pass/segment state, progression change and
payload extent is processed, including discarded resolutions. Tile-part and
packet completeness still precede callbacks. Discarded high-resolution entropy
is not retained or Tier-1 decoded, so the new reduced entry points do not claim
to verify that entropy. Retained entropy and all structural/length errors remain
checked. Existing full-resolution validation remains unchanged.

Original source pixels, input bytes, frame and metadata budgets are retained;
reduction cannot bypass the source pixel limit. Expanded budgets cover reduced
canvas/component/working storage plus full-resolution block/header/progression
state. Retained segment bytes are charged before allocation with a conservative
capacity allowance. Discarded payload is checked for extent without charging
it as an owned copy. Header state can dominate a deeply reduced result; a
smaller output does not guarantee acceptance under a tight expanded budget.
These logical budgets are not a process RSS bound. All input and the output
canvas remain in memory; spatial ROI and streaming are separate work.

The independent corpus contains 1,309 OpenJPEG 2.5.3 `opj_decompress -r`
PGX outputs, including all supported reductions from the existing high-depth
fixtures and the five-progression, multi-layer 8-bit corpus. Reversible samples
are exact. Current reduced 9/7 differences peak at 1/1/3 source samples for
8/12/16 bits; full-resolution reference tolerances are unchanged. This corpus
includes odd origins, singleton outputs and collapsed output tiles. OpenJPEG's
9/7 encoder asserts on nine nonzero one-pixel edge-tile profiles; these are
excluded from successful encoding counts. Nine separately documented zero-
entropy derived controls independently decode all 27 reduced empty-tile outputs.

The fixture manifest, packed reference blob and compressed per-case provenance
are in `wml2/tests/fixtures/review/jpeg2000-reduced-*`. Regenerate with
`generate_jpeg2000_reduced.py --compress /path/to/opj_compress --decompress
/path/to/opj_decompress --work-dir .test-reduced-references`.

`jpeg2000_reduced_verify` exports actual native-resolution bytes and times
complete public-API decodes. Example:

```sh
cargo run -p wml2 --release --no-default-features \
  --features jpeg2000,image-buffer-ex --example jpeg2000_reduced_verify \
  -- INPUT.jp2 OUTPUT.u16le 2 16 4
```

The input bit depth/component count select the example output; the library
validates actual declarations. Benchmarks and complete platform evidence are
recorded separately from these initial numerical/contract checks.
