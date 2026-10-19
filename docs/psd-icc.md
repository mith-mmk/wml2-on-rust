# PSD source and output ICC identity

PSD resource1039 remains opaque profile data. Ordinary output is RGBA8:
Gray replication and the legacy inverted-CMYK numeric helper do not apply ICC.
Those profiles are now retained as `Source ICC Profile`, as are short or
unknown/incompatible model declarations. RGB and indexed palette output retain
`ICC Profile` only when a complete128-byte header declares RGB. This identifies
sample compatibility; it does not validate the profile or compile a transform.

A repeated1039 resource keeps the established last-record semantics. Replacing
an active record with a source-only record clears the stale active declaration.
Retained ICC bytes/keys are admitted to the shared cumulative metadata ledger
before fallible allocation. Direct legacy decode establishes a ledger while
guarded/container calls preserve their existing scope. No automatic sRGB output
or additional runtime dependency is introduced.

Two new integration regressions cover all4source modes, matching/mismatching/
short profiles, unchanged pixels, limits and repeated resources. PSD-only tests
pass, including the established raw/RLE/ZIP, layer and corruption cases.
Nine manually generated raw PSD inputs are independently decoded by Pillow;
all pixels match both Pillow and the unmodified baseline exactly. The CMYK
profile header in this applicability test is an opaque model declaration,
not an independently validated or transformable CMYK profile. The paired
images were displayed without corruption, and all retained profile bytes match.

## Local benchmark evidence

Release baselinefbec877 (PSD source identical to b792e4a),5alternating process
pairs pinned toCPU0,same-length executable aliases,1100iterations/process with
first100omitted,5000samples per variant, no concurrent local builds/tests.
Pixel hashes are unchanged. The earlier3-pair results are retained to show
measurement sensitivity; tiny-case medians are unstable across runs.

| Case | Final before | Final after | Final change | Earlier change |
| --- | ---: | ---: | ---: | ---: |
| psd-gray-plain | 16.55µs | 19.05µs | +15.06% | +0.13% |
| psd-gray-icc | 19.83µs | 19.99µs | +0.81% | -12.32% |
| psd-indexed-plain | 18.94µs | 19.05µs | +0.58% | +21.26% |
| psd-indexed-icc | 23.36µs | 24.28µs | +3.90% | +4.99% |
| psd-rgb-plain | 34.64µs | 29.18µs | -15.76% | +0.23% |
| psd-rgb-icc | 29.79µs | 29.80µs | +0.03% | +5.32% |
| psd-cmyk-declaration-plain | 45.61µs | 46.42µs | +1.78% | -13.33% |
| psd-cmyk-declaration-icc | 40.32µs | 49.08µs | +21.74% | +0.00% |
| psd-large-rgb-icc | 967.91µs | 976.01µs | +0.84% | +1.90% |

The256×128 RGB case is+0.84% in the final run (+1.90% initially). Several
32×32 controls swing by about15–22% across runs in either direction, including
unchanged unprofiled controls; these runs cannot establish a persistent tiny-image
performance change. No speedup or universal unchanged-performance claim is made.
ICC storage remains one retained copy; source naming changes key storage by7bytes.
Detailed medians, allocation counts and all paired CSVs are saved in local evidence.
