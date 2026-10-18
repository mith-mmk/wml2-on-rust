#![cfg(all(feature = "jpeg2000", feature = "image-buffer-ex"))]
#[path = "../../wml2-test/scripts/jpeg2000_ex_checks.rs"]
mod checks;
use checks::*;
use std::path::Path;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
fn root() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/review"
    ))
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(root().join(name)).unwrap()
}

#[test]
fn all_source_and_independent_pgx_pixels_depth_stride_and_qcc_orders() {
    let manifest = std::fs::read_to_string(root().join("jpeg2000-ex-gray-manifest.txt")).unwrap();
    let mut count = 0;
    for line in manifest.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        let bits: u8 = f[2].parse().unwrap();
        let w: usize = f[3].parse().unwrap();
        let h: usize = f[4].parse().unwrap();
        let expected = read(f[1]);
        // PGX was produced by independent OpenJPEG decode, never by WML2.
        let pgx = read(&f[1].replace(".u16le", ".pgx"));
        let start = pgx.iter().position(|&v| v == b'\n').unwrap() + 1;
        assert_eq!(
            std::str::from_utf8(&pgx[..start]).unwrap().trim(),
            format!("PG ML + {bits} {w} {h}")
        );
        let le: Vec<_> = pgx[start..]
            .chunks_exact(2)
            .flat_map(|v| u16::from_be_bytes([v[0], v[1]]).to_le_bytes())
            .collect();
        assert_eq!(le, expected);
        pixels(&read(f[0]), &expected, w, h, bits);
        count += 1;
    }
    assert_eq!(count, 80);
}

#[test]
fn callback_abort_errors_and_workspace_thresholds_are_transactional() {
    for bits in [12, 16] {
        let stem = format!("jp2_ex_gray{bits}_tiles");
        for ext in ["j2k", "jp2"] {
            callbacks_and_limits(
                &read(&format!("{stem}.{ext}")),
                &read(&format!("{stem}.u16le")),
                33,
                29,
                bits,
            );
        }
    }
}

#[test]
fn exact_round_and_legacy_callback_contracts_are_explicit() {
    for bits in [12, 16] {
        let data = read(&format!("jp2_ex_gray{bits}_edges.jp2"));
        assert!(image_from_ex(&data, PixelFormatEx::Gray8, PrecisionConversion::Exact).is_err());
        let image = image_from_ex(&data, PixelFormatEx::Gray8, PrecisionConversion::Round).unwrap();
        let maximum = (1u32 << bits) - 1;
        let source = read(&format!("jp2_ex_gray{bits}_edges.u16le"));
        let expected: Vec<_> = source
            .chunks_exact(2)
            .map(|v| {
                ((u32::from(u16::from_le_bytes([v[0], v[1]])) * 255 + maximum / 2) / maximum) as u8
            })
            .collect();
        assert_eq!(image.bytes().unwrap(), expected);
        let mut legacy = ImageBuffer::new();
        let error = image_decoder_ex(
            &mut bin_rs::reader::BytesReader::new(&data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut legacy,
            },
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("legacy callback requires packed RGBA8")
        );
        assert!(legacy.buffer.is_none());
    }
}

#[test]
fn native_and_32bit_geometry_contracts_are_checked() {
    for bits in [12, 16] {
        geometry_boundaries(
            &read(&format!("jp2_ex_gray{bits}_noise.j2k")),
            &read(&format!("jp2_ex_gray{bits}_noise.u16le")),
            33,
            29,
            bits,
        );
    }
}

#[test]
fn large_x_origins_preserve_pixels_on_native_and_32bit() {
    for bits in [12, 16] {
        large_x_origin_pixels(
            &read(&format!("jp2_ex_gray{bits}_noise.j2k")),
            &read(&format!("jp2_ex_gray{bits}_noise.u16le")),
            33,
            29,
            bits,
        );
    }
}

fn rejected(data: &[u8]) {
    let mut p = Probe::new(16);
    assert!(
        decode(
            data,
            &mut p,
            DecodeLimits {
                pixels: 1_000_000,
                expanded_bytes: 4 * 1024 * 1024,
                metadata_bytes: 65536,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(p.events.is_empty());
}
fn marker(data: &[u8], m: u8) -> usize {
    data.windows(2).position(|v| v == [255, m]).unwrap()
}
#[test]
fn unsupported_profiles_and_overflow_reject_before_callbacks() {
    for name in [
        "signed16",
        "rgb16",
        "rgba16",
        "irreversible16",
        "gray10",
        "roi16",
    ] {
        for ext in ["j2k", "jp2"] {
            let data = read(&format!("jp2_ex_unsupported_{name}.{ext}"));
            let mut p = Probe::new(16);
            let error = decode(&data, &mut p, DecodeLimits::default()).unwrap_err();
            assert!(error.to_string().contains("EX"), "{name}: {error}");
            assert!(p.events.is_empty());
        }
    }
    let data = read("jp2_ex_gray16_noise.j2k");
    let siz = marker(&data, 0x51);
    let cod = marker(&data, 0x52);
    for depth in [1u8, 8, 9, 10, 11, 13, 14, 15, 17, 20, 32, 38, 128] {
        let mut changed = data.clone();
        changed[siz + 40] = depth - 1;
        rejected(&changed);
    }
    let mut changed = data.clone();
    changed[siz + 40] |= 128;
    rejected(&changed);
    for offset in [siz + 41, siz + 42] {
        let mut changed = data.clone();
        changed[offset] = 2;
        rejected(&changed);
    }
    let mut changed = data.clone();
    changed[cod + 13] = 0;
    rejected(&changed); // irreversible
    let mut changed = data.clone();
    changed[cod + 8] = 1;
    rejected(&changed); // MCT
    for offset in [siz + 6, siz + 10] {
        let mut changed = data.clone();
        changed[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        rejected(&changed);
    }
    // A very large nominal tile is valid when clipped to the small canvas.
    // Check saturating tile arithmetic instead of treating that as malformed.
    for offset in [siz + 22, siz + 26] {
        let mut changed = data.clone();
        changed[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        pixels(&changed, &read("jp2_ex_gray16_noise.u16le"), 33, 29, 16);
    }
    for bits in [12, 16] {
        let data = read(&format!("jp2_ex_gray{bits}_edges.jp2"));
        let colr = data.windows(4).position(|v| v == b"colr").unwrap();
        let mut changed = data.clone();
        changed[colr + 4] = 2;
        rejected(&changed);
        let mut changed = data.clone();
        changed[colr + 10] = 16;
        rejected(&changed); // RGB enum
        let ihdr = data.windows(4).position(|v| v == b"ihdr").unwrap();
        let mut changed = data.clone();
        changed[ihdr + 14] = 7;
        rejected(&changed); // SIZ disagreement
    }
    // Existing RGB/RGBA and Gray8 streams remain outside this EX profile.
    for name in [
        "jp2_general_RGBA.jp2",
        "jp2_general_RGB_LRCP_0.jp2",
        "jp2_general_L_LRCP_0.jp2",
    ] {
        rejected(&read(name));
    }
}

#[test]
fn truncations_and_bounded_header_body_mutations_never_panic_or_emit_on_error() {
    for bits in [12, 16] {
        for ext in ["j2k", "jp2"] {
            let data = read(&format!("jp2_ex_gray{bits}_tiles.{ext}"));
            for end in 0..data.len() {
                rejected(&data[..end]);
            }
            for at in (8..data.len()).step_by(7) {
                for mask in [1, 128, 255] {
                    let mut changed = data.clone();
                    changed[at] ^= mask;
                    let mut p = Probe::new(bits);
                    let result = decode(
                        &changed,
                        &mut p,
                        DecodeLimits {
                            pixels: 1_000_000,
                            expanded_bytes: 4 * 1024 * 1024,
                            metadata_bytes: 65536,
                            ..Default::default()
                        },
                    );
                    if result.is_err() {
                        assert!(p.events.is_empty());
                    }
                }
            }
        }
    }
}
