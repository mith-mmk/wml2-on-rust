#![cfg(all(feature = "tga", not(feature = "noretoro")))]
use wml2::draw::image_load;
fn sample(base: u8, depth: u8, palette_depth: u8, attrs: u8, rle: bool, bytes: &[u8]) -> Vec<u8> {
    let mut data = vec![0; 18];
    data[2] = base + if rle { 8 } else { 0 };
    data[12] = 1;
    data[14] = 1;
    data[16] = depth;
    data[17] = 0x20
        | if depth == 32 || depth == 16 && base == 3 || palette_depth == 32 {
            8
        } else {
            1
        };
    if base == 1 {
        data[1] = 1;
        data[5] = 1;
        data[7] = palette_depth;
        data.extend_from_slice(bytes);
        if rle {
            data.push(0);
        }
        data.push(0);
    } else {
        if rle {
            data.push(0);
        }
        data.extend_from_slice(bytes);
    }
    let offset = data.len() as u32;
    let mut ext = vec![0; 495];
    ext[..2].copy_from_slice(&495u16.to_le_bytes());
    ext[494] = attrs;
    data.extend_from_slice(&ext);
    data.extend_from_slice(&offset.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"TRUEVISION-XFILE.\0");
    data
}
#[test]
fn useful_and_ignored_alpha_preserve_correct_colors() {
    for rle in [false, true] {
        for attrs in [0, 1, 3, 4] {
            let data = sample(2, 32, 0, attrs, rle, &[10, 20, 30, 0]);
            let want = if attrs < 2 {
                vec![30, 20, 10, 255]
            } else {
                vec![0, 0, 0, 0]
            };
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), want);
            let data = sample(1, 8, 32, attrs, rle, &[10, 20, 30, 0]);
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), want);
            let data = sample(3, 16, 0, attrs, rle, &[23, 0]);
            assert_eq!(
                image_load(&data).unwrap().buffer.unwrap(),
                if attrs < 2 {
                    vec![23, 23, 23, 255]
                } else {
                    vec![0; 4]
                }
            );
        }
    }
}
#[test]
fn explicit_alpha_bypasses_legacy_16bit_all_zero_fallback() {
    for attrs in [3, 4] {
        for base in [1, 2] {
            let data = sample(
                base,
                if base == 1 { 8 } else { 16 },
                if base == 1 { 16 } else { 0 },
                attrs,
                false,
                &[0, 0x7c],
            );
            assert_eq!(image_load(&data).unwrap().buffer.unwrap(), vec![0; 4]);
        }
    }
}
#[test]
fn premultiplied_alpha_is_unassociated_with_rounding_and_saturation() {
    for base in [1, 2] {
        for rle in [false, true] {
            let data = sample(
                base,
                if base == 1 { 8 } else { 32 },
                if base == 1 { 32 } else { 0 },
                4,
                rle,
                &[25, 50, 100, 128],
            );
            assert_eq!(
                image_load(&data).unwrap().buffer.unwrap(),
                vec![199, 100, 50, 128]
            );
        }
    }
    let data = sample(3, 16, 0, 4, false, &[64, 128]);
    assert_eq!(
        image_load(&data).unwrap().buffer.unwrap(),
        vec![128, 128, 128, 128]
    );
    let data = sample(2, 32, 0, 4, false, &[255, 255, 255, 1]);
    assert_eq!(
        image_load(&data).unwrap().buffer.unwrap(),
        vec![255, 255, 255, 1]
    );
}
#[test]
fn undefined_alpha_has_no_silent_rgba_interpretation() {
    for attrs in [2, 5, 127, 128, 255] {
        assert!(image_load(&sample(2, 32, 0, attrs, false, &[1, 2, 3, 4])).is_err());
    }
    let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 4]);
    data[17] = 0x20;
    assert!(image_load(&data).is_err());
}
#[test]
fn footer_and_extension_cannot_supply_missing_image_pixels() {
    let mut data = sample(2, 32, 0, 3, false, &[]);
    assert!(image_load(&data).is_err());
    data = sample(2, 32, 0, 3, true, &[]);
    assert!(image_load(&data).is_err());
    // Even a footer with no extension is excluded from the pixel payload.
    let mut data = vec![0; 18];
    data[2] = 2;
    data[12] = 1;
    data[14] = 1;
    data[16] = 32;
    data[17] = 0x28;
    data.extend_from_slice(&[0; 8]);
    data.extend_from_slice(b"TRUEVISION-XFILE.\0");
    assert!(image_load(&data).is_err());
    for offset in [1u32, u32::MAX] {
        let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
        let footer = data.len() - 26;
        data[footer..footer + 4].copy_from_slice(&offset.to_le_bytes());
        assert!(image_load(&data).is_err());
    }
    let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
    data[22..24].copy_from_slice(&494u16.to_le_bytes());
    assert!(image_load(&data).is_err());
}

#[test]
fn developer_and_ancillary_ranges_bound_pixels_without_decoding_metadata() {
    // Unknown developer tags are accepted when their referenced bytes fit.
    let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
    let footer = data.split_off(data.len() - 26);
    let developer = data.len();
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&123u16.to_le_bytes());
    data.extend_from_slice(&((developer + 12) as u32).to_le_bytes());
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(&[7, 8, 9]);
    data.extend_from_slice(&footer);
    let f = data.len() - 26;
    data[f + 4..f + 8].copy_from_slice(&(developer as u32).to_le_bytes());
    assert_eq!(
        image_load(&data).unwrap().buffer.unwrap(),
        vec![3, 2, 1, 255]
    );
    let mut bad = data.clone();
    bad[developer + 4..developer + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(image_load(&bad).is_err());
    let mut bad = data.clone();
    bad[developer + 4..developer + 8].copy_from_slice(&18u32.to_le_bytes());
    assert!(image_load(&bad).is_err());
    // A scanline table can precede the extension; it must not supply pixels.
    let mut data = sample(2, 32, 0, 3, false, &[]);
    let offset = 18usize;
    data[18 + 490..18 + 494].copy_from_slice(&(offset as u32).to_le_bytes());
    assert!(image_load(&data).is_err());
    for field in [482usize, 486, 490] {
        let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
        data[22 + field..22 + field + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(image_load(&data).is_err());
    }
    // Truncated image ID and color map cannot read into metadata either.
    let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
    data[0] = 20;
    assert!(image_load(&data).is_err());
    let mut data = sample(1, 8, 32, 3, false, &[1, 2, 3, 255]);
    data[5] = 2;
    assert!(image_load(&data).is_err());
}
#[test]
fn future_extension_size_and_legacy_alpha_are_deliberate() {
    let mut data = sample(2, 32, 0, 3, false, &[1, 2, 3, 255]);
    let f = data.len() - 26;
    data.insert(f, 0);
    data[22..24].copy_from_slice(&496u16.to_le_bytes());
    assert!(image_load(&data).is_ok());
    let mut legacy = sample(2, 16, 0, 3, false, &[0, 0x7c]);
    legacy.truncate(20);
    assert_eq!(
        image_load(&legacy).unwrap().buffer.unwrap(),
        vec![255, 0, 0, 255]
    );
}
#[test]
#[ignore = "writes TGA2 alpha fixtures and RGBA expectations; set WML2_TGA_OUTPUT to existing directory"]
fn tga2_external_artifacts() {
    let p = std::path::PathBuf::from(std::env::var("WML2_TGA_OUTPUT").unwrap());
    for attrs in [0, 1, 3, 4] {
        let mut data = sample(2, 32, 0, attrs, false, &[25, 50, 100, 128]);
        data[12] = 2;
        let extension = 22usize;
        data.splice(extension..extension, [0, 0, 0, 0]);
        let f = data.len() - 26;
        data[f..f + 4].copy_from_slice(&26u32.to_le_bytes());
        std::fs::write(p.join(format!("v2-alpha-{attrs}.tga")), &data).unwrap();
        let image = image_load(&data).unwrap();
        std::fs::write(
            p.join(format!("v2-alpha-{attrs}.rgba")),
            image.buffer.unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn footer_must_not_overlap_header_image_id_or_palette() {
    for length in 26..44 {
        let mut data = vec![0; length];
        data[2] = 2;
        data[12] = 1;
        data[14] = 1;
        data[16] = 32;
        let footer = length - 26;
        data[footer..footer + 8].fill(0);
        data[footer + 8..].copy_from_slice(b"TRUEVISION-XFILE.\0");
        assert!(std::panic::catch_unwind(|| image_load(&data)).is_ok());
        assert!(image_load(&data).is_err());
    }
    let mut data = vec![0; 18];
    data[2] = 2;
    data[12] = 1;
    data[14] = 1;
    data[16] = 32;
    data[0] = 1;
    data.extend_from_slice(&[0; 8]);
    data.extend_from_slice(b"TRUEVISION-XFILE.\0");
    assert!(image_load(&data).is_err());
    let mut data = vec![0; 18];
    data[1] = 1;
    data[2] = 1;
    data[5] = 1;
    data[7] = 32;
    data[12] = 1;
    data[14] = 1;
    data[16] = 8;
    data.extend_from_slice(&[0; 8]);
    data.extend_from_slice(b"TRUEVISION-XFILE.\0");
    assert!(image_load(&data).is_err());
}
