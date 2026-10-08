These four Gray8/Gray16 TIFF fixtures are generated test assets, not copies
of the external codec corpus. They were created with Pillow 12.3.0 and
libtiff 4.7.1, using TIFF LZW and PackBits with Predictor 1. WML2's encoders
are not involved. Pillow independently reopens every file and compares the
decoded samples to the original input before the generator completes.

Dimensions are 257 by 3. For pixel `(x, y)` the original integer sample is:

- Gray8: `(x * 37 + y * 83 + (x / 7) * 11) % 256`
- Gray16: `(x * 257 + y * 4660 + (x / 17) * 31) % 65536`

Division is integer floor division. The first three values are 0, 37, 74
and 0, 257, 514 respectively. EX expectations are these original values,
with Gray16 represented as little-endian words; they are not inferred from
another WML2 decode. The LZW files use real dictionary compression in
addition to the independent literal/Clear fixture streams in the tests.

Regenerate into the existing fixture directory:

```sh
python3 wml2-test/scripts/generate_ex_tiff_compression_samples.py --dest wml2/tests/fixtures
```
