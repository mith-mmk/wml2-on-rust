#![cfg(all(feature = "dds", not(feature = "noretoro")))]

use wml2::draw::{image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;

// 64 deterministic arbitrary blocks for each single-subset mode, generated
// without a WML2 encoder. The expected RGBA is Pillow 12.3.0's BC7 decode.
// Seed 0xBC7456, Python random.Random.randbytes(16), overwrite low mode+1
// bits in byte 0 with 1<<mode; modes 4, 5, 6 in that order. Each block is a
// 4x4 tile in a 4x768 DX10/BC7_UNORM image, no additional surfaces/mips.
const EXTERNAL_DDS: &[u8] = include_bytes!("fixtures/dds-bc7-single-subset.dds");
const EXTERNAL_RGBA: &[u8] = include_bytes!("fixtures/dds-bc7-single-subset.rgba");

#[test]
fn all_single_subset_modes_match_independent_pillow_decoder() {
    let mut rotations_selectors = [[false; 2]; 4];
    for block in EXTERNAL_DDS[148..148 + 64 * 16].chunks_exact(16) {
        rotations_selectors[usize::from((block[0] >> 5) & 3)][usize::from(block[0] >> 7)] = true;
    }
    assert!(rotations_selectors.into_iter().flatten().all(|v| v));
    for dxgi in [98u32, 99] {
        let mut bytes = EXTERNAL_DDS.to_vec();
        bytes[128..132].copy_from_slice(&dxgi.to_le_bytes());
        let image = image_load(&bytes).unwrap();
        assert_eq!((image.width, image.height), (4, 768));
        assert_eq!(image.buffer.unwrap(), EXTERNAL_RGBA);
    }
}

#[test]
fn mode6_pbits_anchor_and_all_interpolation_indices_are_bit_exact() {
    // E0 RGBA=0 with pbit0=0, E1 RGBA=255 with pbit1=1; anchor
    // index0=7 uses only three bits, following indices are 1 through 15.
    // Literal packed block and expected values verified independently by Pillow.
    let block = [
        64, 192, 31, 240, 7, 252, 1, 127, 31, 50, 84, 118, 152, 186, 220, 254,
    ];
    let values = [
        120, 16, 36, 52, 68, 84, 104, 120, 135, 151, 171, 187, 203, 219, 239, 255,
    ];
    let expected: Vec<u8> = values.into_iter().flat_map(|v| [v; 4]).collect();
    assert_eq!(
        image_load(&one_block(block, 4, 4)).unwrap().buffer.unwrap(),
        expected
    );
}

fn one_block(block: [u8; 16], width: u32, height: u32) -> Vec<u8> {
    let mut bytes = EXTERNAL_DDS[..148].to_vec();
    bytes[12..16].copy_from_slice(&height.to_le_bytes());
    bytes[16..20].copy_from_slice(&width.to_le_bytes());
    bytes[20..24].copy_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&block);
    bytes
}

#[test]
fn partitioned_modes_fail_explicitly_until_implemented() {
    for mode in [0, 1, 2, 3, 7] {
        let mut block = [0; 16];
        block[0] = 1 << mode;
        let error = image_load(&one_block(block, 4, 4)).err().unwrap();
        assert!(
            error.to_string().contains("BC7 partitioned modes"),
            "{error}"
        );
    }
}

#[test]
fn reserved_mode_returns_zero_rgba_and_partial_tiles_are_clipped() {
    assert_eq!(
        image_load(&one_block([0; 16], 3, 2))
            .unwrap()
            .buffer
            .unwrap(),
        vec![0; 24]
    );
    let mut block = [0; 16];
    block.copy_from_slice(&EXTERNAL_DDS[148..164]);
    let image = image_load(&one_block(block, 3, 2)).unwrap();
    let expected: Vec<u8> = [0, 1]
        .into_iter()
        .flat_map(|y| EXTERNAL_RGBA[y * 16..y * 16 + 12].iter().copied())
        .collect();
    assert_eq!(image.buffer.unwrap(), expected);
}

#[test]
fn truncated_bc7_payload_is_rejected() {
    for length in 0..16 {
        let mut bytes = one_block([64; 16], 4, 4);
        bytes.truncate(148 + length);
        assert!(image_load(&bytes).is_err());
    }
}

#[test]
fn bc7_checks_pixel_and_expanded_byte_limits_before_allocating() {
    let bytes = one_block(EXTERNAL_DDS[148..164].try_into().unwrap(), 4, 4);
    for (pixels, expanded_bytes) in [(15, 64), (16, 63)] {
        let limits = DecodeLimits {
            pixels,
            expanded_bytes,
            ..DecodeLimits::unlimited()
        };
        let error = image_from_with_limits(&bytes, limits).err().unwrap();
        assert!(error.to_string().contains("limit"), "{error}");
    }
}
