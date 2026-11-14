use super::decoder::{cmyk_to_rgb, packed_cmyk, ycck_to_rgb};
use super::header::Component;

fn components(h: usize, v: usize) -> Vec<Component> {
    (0..4)
        .map(|i| Component {
            c: 10 + i,
            h: if i == 0 { h } else { 1 },
            v: if i == 0 { v } else { 1 },
            tq: 0,
        })
        .collect()
}

#[test]
fn cmyk_block_offsets_are_row_major_not_accumulated() {
    for (h, v) in [(2, 2), (3, 1)] {
        let c = components(h, v);
        let mut units: Vec<_> = (0..h * v).map(|b| vec![100 + b as u8 * 20; 64]).collect();
        units.extend([vec![200; 64], vec![210; 64], vec![250; 64]]);
        let out = cmyk_to_rgb(&units, &c, (h, v));
        for y in 0..v * 8 {
            for x in 0..h * 8 {
                let p = (y * h * 8 + x) * 4;
                assert_eq!(
                    &out[p..p + 4],
                    &[95 + (y / 8 * h + x / 8) as u8 * 20, 195, 205, 255]
                );
            }
        }
    }
}

#[test]
fn ycck_samples_each_luma_block_and_preserves_legacy_rounding() {
    let c = components(2, 2);
    let units = vec![
        vec![20; 64],
        vec![40; 64],
        vec![60; 64],
        vec![80; 64],
        vec![127; 64],
        vec![127; 64],
        vec![200; 64],
    ];
    let out = ycck_to_rgb(&units, &c, (2, 2));
    for y in 0..16 {
        for x in 0..16 {
            let expected = 180 - (y / 8 * 2 + x / 8) as u8 * 20;
            assert_eq!(
                &out[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4],
                &[expected, expected, expected, 255]
            );
        }
    }
}

#[test]
fn canonical_cmyk_distinguishes_unmarked_and_adobe_complements() {
    let c = components(1, 1);
    let units = [12, 34, 56, 78].map(|v| vec![v; 64]);
    let mut out = Vec::new();
    packed_cmyk(&units, &c, (1, 1), false, false, &mut out);
    assert_eq!(&out[..4], &[12, 34, 56, 78]);
    packed_cmyk(&units, &c, (1, 1), false, true, &mut out);
    assert_eq!(&out[..4], &[243, 221, 199, 177]);
    let units = [40, 128, 128, 200].map(|v| vec![v; 64]);
    packed_cmyk(&units, &c, (1, 1), true, true, &mut out);
    assert_eq!(&out[..4], &[40, 40, 40, 55]);
}
