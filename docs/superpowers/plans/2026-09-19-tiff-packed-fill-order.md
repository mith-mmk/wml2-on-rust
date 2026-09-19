# TIFF Packed FillOrder Interoperability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Correct packed 1/2/4-bit grayscale TIFF decoding with `FillOrder=2` so sample order and row padding match the existing palette path and external TIFF behavior, without changing the public WML2 API.

**Architecture:** Keep packed-bit extraction in `wml2/src/tiff/sample.rs` as the single internal contract for `FillOrder=1` and `FillOrder=2`. Replace the grayscale decoder's handwritten 1/2/4-bit branches with that helper, using a row-local sample index so padding bits never become pixels. Extend the existing review fixture and converter/metadata oracle rather than adding a new runtime dependency or changing orientation behavior.

**Tech Stack:** Rust, Cargo, WML2 TIFF decoder, generated Classic TIFF fixtures, Python/Pillow, ImageMagick/LibTIFF oracle scripts, GitHub Actions.

**Spec:** `docs/tiff-extend.md`, `docs/tiff-extend-review-fixes.md`, and `docs/tiff-extend-alpha-lzw-review.md`.

## Global Constraints

- Preserve the `wml2` 0.0.31 public API and the existing RGBA8/native TIFF contracts; implementation changes are internal.
- `FillOrder=1` remains MSB-first and `FillOrder=2` applies only to packed sample bit positions; it must not select or alter LZW code order.
- Byte-aligned samples (8/16/32-bit) are unaffected by `FillOrder`.
- Each strip/tile row starts its packed sample index at zero; trailing padding bits are never decoded as pixels.
- Packed planar TIFF remains rejected by the current validation contract; do not broaden it in this change.
- TIFF Orientation 2–8 continues to be retained as metadata without automatic rotation.
- No native `libtiff`, ImageMagick, or FFmpeg runtime dependency may be added; external tools remain validation-only.
- Keep generated samples and reports under `.test*` or the existing temporary scratch area; do not add external binaries to Git.

## Review Focus

- A 4-bit `FillOrder=2` row must preserve sample order (`1, 2`), not reverse the two nibbles; pin this in the decoder integration test in Task 1.
- 1-bit and 2-bit `FillOrder=2` extraction must preserve the bit value and cross-byte position; pin the helper contract in Task 1.
- A row whose width is not byte-aligned must restart at the next row's first sample; pin mixed-width two-row fixtures in Task 3.
- Palette decoding and LZW code order must remain independent of packed sample `FillOrder`; pin both direct and LZW-backed cases in Task 3.
- Byte-aligned decoding and packed planar rejection must remain unchanged; pin regression commands and assertions in Task 4.

## Current Baseline

- Parent checkout is clean at `5f26664` / tag `v0.0.31`.
- The known remaining TIFF review boundary is the main-derived Gray4/`FillOrder=2` pixel-order issue. The existing palette regression `r5_fill_order_two_four_bit_palette_matches_libtiff_order` already demonstrates the intended `0x48 -> [1, 2]` interpretation.
- Existing CI already runs `tiff_review_regressions`, `tiff_extend`, converter/metadata, and the TIFF oracle manifest, so the new fixture can use those gates without a new workflow job.

## Files and Responsibilities

- Modify `wml2/src/tiff/sample.rs`: retain and test the canonical packed-sample reader used by all packed callers.
- Modify `wml2/src/tiff/decoder/mod.rs`: route grayscale 1/2/4-bit decoding through the canonical reader with a row-local index.
- Modify `wml2/tests/tiff_review_regressions.rs`: add an in-repository Gray4 `FillOrder=2` failure fixture and the packed-depth/row-padding regression matrix.
- Modify `wml2-test/scripts/generate_tiff_review_samples.py`: add LE/BE converter-oracle samples for packed grayscale `FillOrder=2`.
- Modify `docs/tiff-extend-alpha-lzw-review.md`: append the final boundary, fixture names, and validation result without claiming orientation support or remote execution that was not run.

---

### Task 1: Lock the packed-sample contract and reproduce Gray4 ordering

**Files:**
- Modify: `wml2/src/tiff/sample.rs:77-111, 202-247`
- Modify: `wml2/tests/tiff_review_regressions.rs` near `r5_fill_order_two_four_bit_palette_matches_libtiff_order`

**Interfaces:**
- Consumes: existing `read_sample_with_fill_order(data, bits, endian, index, fill_order) -> Result<u32, Error>`.
- Produces: a failing regression named `r5_fill_order_two_four_bit_gray_matches_libtiff_order` and unit coverage proving the helper's 1/2/4-bit interpretation.

- [ ] **Step 1: Add the helper contract test**

Add a unit test in `wml2/src/tiff/sample.rs` that uses non-palindromic values and asserts the existing helper contract:

```rust
#[test]
fn reads_fill_order_two_packed_samples_in_stored_order() {
    assert_eq!(
        read_sample_with_fill_order(&[0x48], 4, Endian::BigEndian, 0, 2).unwrap(),
        1
    );
    assert_eq!(
        read_sample_with_fill_order(&[0x48], 4, Endian::BigEndian, 1, 2).unwrap(),
        2
    );
    assert_eq!(
        read_sample_with_fill_order(&[0b00_01_11_10], 2, Endian::BigEndian, 0, 2).unwrap(),
        1
    );
    assert_eq!(
        read_sample_with_fill_order(&[0b0000_0001], 1, Endian::BigEndian, 0, 2).unwrap(),
        1
    );
}
```

- [ ] **Step 2: Add the Gray4 integration fixture**

In `wml2/tests/tiff_review_regressions.rs`, create a two-entry grayscale color map (`17 * 257` and `34 * 257` in every RGB plane), build a `Spec` with `width=2`, `height=1`, `bits=[4]`, `samples=1`, `photo=1`, `fill_order=2`, and raw block `[0x48]`, then assert the decoded RGBA bytes are `[17,17,17,255,34,34,34,255]`.

- [ ] **Step 3: Run the new test before implementation**

Run:

```powershell
cargo test -p wml2 --test tiff_review_regressions --no-default-features --features tiff r5_fill_order_two_four_bit_gray_matches_libtiff_order -- --exact
```

Expected: the helper unit test passes, while the new integration test fails because the current grayscale branch reverses the `0x48` sample order.

- [ ] **Step 4: Commit the reproduction**

```powershell
git add wml2/src/tiff/sample.rs wml2/tests/tiff_review_regressions.rs
git commit -m "test(tiff): reproduce packed grayscale fill-order regression"
```

### Task 2: Route packed grayscale decoding through the canonical reader

**Files:**
- Modify: `wml2/src/tiff/decoder/mod.rs:650-940`

**Interfaces:**
- Consumes: `read_sample_with_fill_order` and the existing row bounds (`row_len`, `l`, `pixel`, `header.fill_order`).
- Produces: grayscale 1/2/4-bit decode that uses the same bit-order semantics as palette decode, with no changes to byte-aligned or planar paths.

- [ ] **Step 1: Replace the handwritten 4-bit extraction**

In the grayscale `4` branch, obtain the current row slice with checked `row_start`/`row_end` arithmetic and call:

```rust
let row = data
    .get(row_start..row_end)
    .ok_or_else(|| std::io::Error::other("TIFF packed grayscale row is truncated"))?;
let c = crate::tiff::sample::read_sample_with_fill_order(
    row,
    header.bitspersample,
    header.tiff_headers.endian,
    pixel,
    header.fill_order,
)? as usize;
```

Remove only the `reverse_bits()`/nibble selection from this branch. Keep palette lookup, RGBA emission, and row bounds unchanged.

- [ ] **Step 2: Apply the same path to 2-bit and 1-bit branches**

Use the same row slice and `pixel` index in the `2` and `1` branches. Do not use the global byte cursor `i` for packed samples; `i` remains the byte cursor for the byte-aligned branches and is not incremented by the packed branches.

- [ ] **Step 3: Preserve the existing explicit limits**

Do not change `page.rs` validation, packed planar rejection, `read_sample_with_fill_order` behavior for byte-aligned samples, Predictor handling, or LZW decoder selection. If a new row-slice check is needed, return the existing decode error style instead of returning a partial image.

- [ ] **Step 4: Run focused tests**

Run:

```powershell
cargo test -p wml2 --lib --no-default-features --features tiff sample::tests::reads_fill_order_two_packed_samples_in_stored_order -- --exact
cargo test -p wml2 --test tiff_review_regressions --no-default-features --features tiff r5_fill_order_two_four_bit_gray_matches_libtiff_order -- --exact
```

Expected: both tests pass, including the previously failing Gray4 case.

- [ ] **Step 5: Commit the decoder change**

```powershell
git add wml2/src/tiff/decoder/mod.rs wml2/src/tiff/sample.rs
git commit -m "fix(tiff): honor fill order for packed grayscale samples"
```

### Task 3: Expand the regression and external-oracle matrix

**Files:**
- Modify: `wml2/tests/tiff_review_regressions.rs`
- Modify: `wml2-test/scripts/generate_tiff_review_samples.py`

**Interfaces:**
- Consumes: the existing `Spec`/`build` fixture builders, TIFF review manifest schema, and `verify_tiff_review_samples.py`.
- Produces: deterministic LE/BE direct fixtures and converter/metadata oracle cases for packed grayscale `FillOrder=2`.

- [ ] **Step 1: Add a row-packing helper to the Rust regression fixture**

Add a test-only helper with this contract. It must pack one row at a time, so the caller can pass a two-dimensional value list and the helper cannot carry padding into the next row:

```rust
fn pack_rows(rows: &[&[u8]], bits: usize, fill_order: u16) -> Vec<u8> {
    assert!(matches!(bits, 1 | 2 | 4));
    assert!(matches!(fill_order, 1 | 2));
    let mask = (1u8 << bits) - 1;
    rows.iter()
        .flat_map(|row| {
            let mut packed = vec![0; (row.len() * bits).div_ceil(8)];
            for (sample, &value) in row.iter().enumerate() {
                assert_eq!(value & !mask, 0);
                for bit in 0..bits {
                    if value & (1 << bit) != 0 {
                        let stream_bit = sample * bits + bit;
                        let byte = stream_bit / 8;
                        let offset = stream_bit % 8;
                        let physical = if fill_order == 1 { 7 - offset } else { offset };
                        packed[byte] |= 1 << physical;
                    }
                }
            }
            packed
        })
        .collect()
}
```

- [ ] **Step 2: Test non-byte-aligned rows**

For each of `bits = 1, 2, 4` and each `fill_order` in `[1, 2]`, build a two-row grayscale page with `width=3`, `height=2` and values that are not all-zero/all-maximum. Use `pack_rows`, assert six decoded grayscale pixels in row-major order, and verify the second row does not consume the first row's padding bits.

- [ ] **Step 3: Test the unaffected paths**

Keep the existing palette `0x48` test and add the same packed values through the LZW block builder for standard MSB and WML2 LSB streams. Assert that changing TIFF `FillOrder` changes only packed sample extraction and never changes the LZW code-order selection. Also retain the existing byte-aligned and packed-planar assertions.

- [ ] **Step 4: Add LE/BE generated oracle fixtures**

In `generate_tiff_review_samples.py`, generate `r5_gray_4bit_fillorder2_le.tif` and `r5_gray_4bit_fillorder2_be.tif` with `width=2`, raw `[0x48]`, a 16-entry grayscale `ColorMap`, `FillOrder=2`, and expected RGBA `[17,17,17,255,34,34,34,255]`. Add a width-3 two-row case for padding and record `FillOrder=2` in `tags`.

- [ ] **Step 5: Run the external validation locally**

Build the tools and run the existing scripts in `C:\temp`:

```powershell
$scratch = 'C:\temp\wml2-tiff-packed-fill-order-20260919'
New-Item -ItemType Directory -Force $scratch | Out-Null
cargo build -p wml2-test --locked --release --example converter --example metadata
python wml2-test/scripts/generate_tiff_review_samples.py --dest "$scratch\samples"
python wml2-test/scripts/verify_tiff_review_samples.py --samples "$scratch\samples" --output "$scratch\reports" --converter '.\target\release\examples\converter.exe' --metadata '.\target\release\examples\metadata.exe' --magick convert
```

Expected: the new samples decode successfully, converter and metadata both exit successfully, expected RGBA matches, and no partial output is reported. Treat ImageMagick disagreement as diagnostic only if the manifest marks a case `oracle_policy=expected_only`.

- [ ] **Step 6: Commit the expanded fixtures**

```powershell
git add wml2/tests/tiff_review_regressions.rs wml2-test/scripts/generate_tiff_review_samples.py
git commit -m "test(tiff): cover packed grayscale fill-order fixtures"
```

### Task 4: Document the boundary and complete validation

**Files:**
- Modify: `docs/tiff-extend-alpha-lzw-review.md` in the final review/validation section

**Interfaces:**
- Consumes: focused Rust results, converter/metadata report, and the existing CI job contract.
- Produces: a reproducible handoff that distinguishes local proof, CI configuration, and unverified remote execution.

- [ ] **Step 1: Record the corrected behavior**

Document that packed grayscale 1/2/4-bit `FillOrder=2` uses row-local sample order, that palette and LZW behavior remain separated, and that Orientation 2–8 remains metadata-only. Name the new fixture files and do not claim automatic orientation support.

- [ ] **Step 2: Run the complete TIFF regression set**

Run:

```powershell
cargo fmt -p wml2 --check
cargo test -p wml2 --test tiff_extend --test tiff_review_regressions --no-default-features --features tiff,high-bit-depth
cargo test -p wml2 --test tiff_lzw_modes --no-default-features --features tiff
cargo test --workspace --locked
cargo clippy -p wml2 --locked --lib --tests --no-default-features --features tiff,high-bit-depth
git diff --check
```

Expected: all relevant tests and checks pass; pre-existing warnings may be recorded but must not be relabeled as new failures.

- [ ] **Step 3: Inspect the final scope**

Run `git status --short`, inspect `git diff 5f26664..HEAD --stat`, and verify that only the decoder, tests, oracle generator, and review documentation changed. Confirm no generated TIFF/PNG/report or `.test*` output is staged.

- [ ] **Step 4: Commit the documentation and handoff**

```powershell
git add docs/tiff-extend-alpha-lzw-review.md
git commit -m "docs(tiff): record packed fill-order interoperability fix"
```

## Acceptance Criteria

- Gray 1/2/4-bit `FillOrder=2` decodes in stored sample order for direct and LZW-backed blocks.
- Widths that leave row padding decode identically row-by-row for `FillOrder=1` and `FillOrder=2`.
- Existing palette, byte-aligned, alpha, BigTIFF, Predictor, LZW-mode, and packed-planar tests remain green.
- Converter and metadata succeed for the new LE/BE generated fixtures, with expected RGBA validation and no partial outputs.
- The docs state the exact supported boundary; Orientation is still metadata-only and remote GitHub execution is reported only if actually observed.

## Execution Handoff

This plan is intentionally limited to the documented TIFF interoperability gap. Do not combine it with orientation transforms, new compression codecs, or AVIF decoder work without a separate plan and acceptance criteria.
