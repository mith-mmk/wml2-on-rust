# Exact adjacent-colour reuse in the shared display adapter

The AVIF/JXL adapter already parses each profile and builds one own-library
transform per image, outside the pixel loop. It uses the f32 API once per
scratch chunk; the integer API's duplicate validation transform was removed
in the earlier boundary correction. There is no per-pixel profile parsing or
transform reconstruction to remove. The expensive general output parametric
TRC inverse in icc-profile0.0.7 uses two32-step binary searches, with repeated
power evaluations. This change does not replace those colour operations.

The remaining exact redundancy addressed here is adjacent identical source
colours. Within each256-pixel chunk, the immutable own-library transform is
evaluated once per exact run of participating u16 colour samples. Alpha is
excluded from equality and is preserved individually. Distinct colours keep
their order. No precision reduction, interpolation, approximate transfer curve,
identity-profile shortcut, ICC substitution or ICC bypass is used. Scratch
remains fixed-size; one256-entry usize mapping adds2KiB on64-bit/1KiB on32-bit
stack storage and no heap allocations. Compiled-profile admission is unchanged.

## Separate comparisons

The earlier approximately9–10x number compared the corrected ICC boundary with
the former incorrect SDK display path. It was not an ICC-versus-no-ICC cost.
The following experiment compares the correct baselinecc4e4a1 with exact run
reuse, and independently compares each revision's ICC and no-ICC paths.

Independent libavif1.2.1/libaom inputs use RGB8, lossless YUV444 identity, full
range, BT709 primaries, sRGB transfer, speed10 and one encoder thread. Each pair
comes from the same native planes; its mdat bytes are identical with/without the
frozen standard sRGB ICC. WML2 native input data, profile bytes and output pixels
are not used to construct the expected independent image references. All5pairs
have exactly equal ordinary RGBA8 output across ICC/no-ICC and before/after.

Both unchanged baseline and current release libraries are linked into one
executable (baseline rustc metadata disambiguator changes symbols only). Each
iteration balances the order of4variants. Three processes run pinned toCPU0,
with no concurrent local builds/tests.32²:110iterations/process, first10omitted,
300samples/variant.128²:20iterations/process, first5omitted,45samples/variant.
Timings include ordinary decode, metadata, output allocation/drop and the same
pixel hash. Inputs are read outside timing. Ratios are medians of paired ratios;
table absolute times are pooled medians and therefore need not divide to the
displayed paired ratio. Rust1.91.0,opt-level3,no-default-features,
`avif,image-buffer-ex,png`.

| Pattern | Plain before/after µs | ICC before/after µs | ICC change | ICC/plain before→after |
| --- | ---: | ---: | ---: | ---: |
| flat-32 | 82.20/85.92 | 2693.58/131.57 | -95.12% | 33.03→1.58 |
| runs-32 | 258.93/262.84 | 2887.04/638.96 | -77.79% | 11.19→2.46 |
| varying-32 | 529.73/526.47 | 3421.11/3424.09 | -0.07% | 6.51→6.56 |
| flat-128 | 467.39/529.37 | 42230.42/923.89 | -97.81% | 89.71→1.76 |
| varying-128 | 5009.52/5028.50 | 48864.16/49115.40 | +0.61% | 9.83→9.78 |

The runs pattern has8identical adjacent pixels; the varying pattern changes
every pixel, even when colours recur later. Before/after hashes, heap allocation
counts and allocated bytes match for all cases. The unchanged no-ICC control
varies by-2.20%to+14.13%, so small timing changes are not precise regression or
speedup estimates. Large same-colour savings are clear; general varying-colour
ICC still costs roughly6.6–9.8times its equivalent no-ICC path here.

The original non-sRGB ICC fixtures are measured separately with alternating
before/after order in one process,3processes onCPU0,110iterations/first10omitted:

- gradient: 4148.17→4127.59µs, paired-0.67%,300samples/variant; exact pixels/heap counts/bytes.
- prem-gradient: 4238.28→4178.29µs, paired-0.22%,300samples/variant; exact pixels/heap counts/bytes.
- gray-limited12: 980.92→960.38µs, paired-1.66%,300samples/variant; exact pixels/heap counts/bytes.

These original gradient/Gray fixtures have few or no adjacent repeats. This
optimization does not eliminate their general TRC inverse cost. Improving that
engine requires a separately verified own-library change and dependency rollout;
neither a library release nor a new dependency source is introduced here.

## Validation and reproduction

The scoped AVIF targets pass65tests with1existing ignored. A regression compares
Gray/RGB output bit-for-bit with the full unbatched own-library operation across
long runs, distinct colours, scratch boundaries and changing alpha. All9 existing
independent libavifRGBA16/LittleCMS16/finalRGBA8 references remain exact; the5new
lossless ICC/plain pairs match independent Pillow/libavif pixels. Their rasters
were displayed side by side without corruption. Scoped Clippy and format pass.

Developer tools only, not runtime dependencies:

```console
cc -O2 -DWML2_AVIF_HEADER='"/path/to/avif.h"' wml2-test/scripts/avif_icc_bench_inputs.c /path/to/libavif.so.16 -o .test-tools/inputs
.test-tools/inputs wml2/src/srgb.icc .test-inputs
# Build unchanged baseline/current with the same locked features into separate rlibs.
# Add -C metadata=avif_exact_runs_before only to the baseline library if needed.
rustc --edition 2024 -C opt-level=3 wml2-test/scripts/avif_icc_pair_bench.rs --extern wml_before=/path/before/libwml2.rlib --extern wml_after=/path/after/libwml2.rlib -L dependency=/path/deps -o .test-tools/pair
taskset -c 0 .test-tools/pair .test-inputs/flat-32-plain.avif .test-inputs/flat-32-icc.avif 110
```

Create the named .test directories first. Save CSV output; discard the stated
warm-up rows and group variants by iteration for ratios. The local evidence
includes input hashes/mdat equality, per-process CSVs and benchmark JSON.
Windows25-image corpus validation remains unexecuted in this cloud environment.
