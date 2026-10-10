#![cfg(all(feature = "pcx", not(feature = "noretoro")))]
use wml2::draw::image_load;

#[test]
fn four_plane_rows_preserve_alpha_and_ignore_each_plane_padding() {
    let mut bytes = vec![0u8; 128];
    bytes[..4].copy_from_slice(&[10, 5, 1, 8]);
    bytes[8..10].copy_from_slice(&2u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
    bytes[65] = 4;
    bytes[66..68].copy_from_slice(&4u16.to_le_bytes());
    let expected: Vec<u8> = (0..6)
        .flat_map(|i| [i * 7, i * 11, i * 13, i * 31])
        .collect();
    for row in expected.chunks_exact(12) {
        for channel in 0..4 {
            for pixel in row.chunks_exact(4) {
                let value = pixel[channel];
                if value >= 192 {
                    bytes.push(193);
                }
                bytes.push(value);
            }
            bytes.extend_from_slice(&[193, 239]);
        }
    }
    assert_eq!(image_load(&bytes).unwrap().buffer.unwrap(), expected);
}
