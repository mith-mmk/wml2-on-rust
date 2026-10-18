#![cfg(all(feature = "jpeg2000", feature = "image-buffer-ex"))]
#[allow(dead_code)]
#[path = "../../wml2-test/scripts/jpeg2000_ex_checks.rs"]
mod checks;
#[allow(dead_code)]
#[path = "../../wml2-test/scripts/jpeg2000_ex_color_checks.rs"]
mod color;
#[path = "../../wml2-test/scripts/jpeg2000_ex_97_checks.rs"]
mod irreversible;
use std::path::{Path, PathBuf};
use wml2::draw::*;
use wml2::limits::DecodeLimits;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(root().join(name)).unwrap()
}

#[test]
fn all_216_irreversible_streams_match_independent_pgx_with_source_depth_tolerance() {
    let manifest = std::fs::read_to_string(root().join("jpeg2000-ex-97-manifest.txt")).unwrap();
    let mut count = 0;
    for line in manifest.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        let bits: u8 = f[2].parse().unwrap();
        let w: usize = f[3].parse().unwrap();
        let h: usize = f[4].parse().unwrap();
        let nc: usize = f[5].parse().unwrap();
        let reference = read(f[1]);
        let channels = if nc == 1 { 1 } else { 4 };
        for c in 0..nc {
            let pgx = read(&f[1].replace(".ref.u16le", &format!("_{c}.pgx")));
            let start = pgx.iter().position(|&v| v == b'\n').unwrap() + 1;
            assert_eq!(
                std::str::from_utf8(&pgx[..start]).unwrap().trim(),
                format!("PG ML + {bits} {w} {h}")
            );
            assert_eq!(pgx.len() - start, w * h * 2);
            for (i, v) in pgx[start..].chunks_exact(2).enumerate() {
                assert_eq!(
                    u16::from_be_bytes([v[0], v[1]]).to_le_bytes(),
                    reference[(i * channels + c) * 2..(i * channels + c + 1) * 2]
                );
            }
        }
        irreversible::pixels(&read(f[0]), &reference, w, h, bits, nc);
        count += 1;
    }
    assert_eq!(count, 216);
}

#[test]
fn floating_callbacks_budgets_conversion_and_large_origins_keep_exact_storage_contracts() {
    for bits in [12, 16] {
        for (kind, nc) in [("gray", 1), ("rgb", 3), ("rgba", 4)] {
            let mct = if nc == 1 { 0 } else { 1 };
            for name in ["edges", "noise"] {
                let stem = format!("jp2_ex_97_{kind}{bits}_{name}_mct{mct}");
                let data = read(&format!("{stem}.jp2"));
                let reference = read(&format!("{stem}_jp2.ref.u16le"));
                let (w, h) = if name == "edges" { (9, 7) } else { (17, 13) };
                let actual = irreversible::pixels(&data, &reference, w, h, bits, nc);
                irreversible::extra(&data, &actual, w, h, bits, nc);
                if nc == 4 && name == "edges" {
                    assert!(
                        actual
                            .chunks_exact(8)
                            .any(|p| p[6..] == [0, 0] && p[..6].iter().any(|&v| v != 0))
                    );
                }
            }
        }
    }
}

#[test]
fn newly_supported_preexisting_irreversible_profiles_declare_high_depth() {
    for name in [
        "jp2_ex_unsupported_irreversible16.j2k",
        "jp2_ex_unsupported_irreversible16.jp2",
        "jp2_ex_color_unsupported_3_irreversible16.jp2",
        "jp2_ex_color_unsupported_4_irreversible16.jp2",
    ] {
        let fmt = if name.contains("color") {
            PixelFormatEx::Rgba16
        } else {
            PixelFormatEx::Gray16
        };
        let image = image_from_ex(&read(name), fmt, PrecisionConversion::Exact).unwrap();
        assert_eq!(image.source_format(), Some(fmt));
        assert!(image_load(&read(name)).is_err());
    }
}

#[test]
fn invalid_9_7_quantization_precision_levels_and_mixed_filters_reject_before_callbacks() {
    for (kind, nc) in [("gray", 1), ("rgb", 3), ("rgba", 4)] {
        let data = read(&format!("jp2_ex_97_{kind}16_noise_mct0.jp2"));
        let cod = data.windows(2).position(|v| v == [255, 0x52]).unwrap();
        let qcd = data.windows(2).position(|v| v == [255, 0x5c]).unwrap();
        let siz = data.windows(2).position(|v| v == [255, 0x51]).unwrap();
        for (offset, value) in [
            (cod + 9, 7),
            (qcd + 4, 0x40),
            (qcd + 4, 0x43),
            (qcd + 5, 0xff),
            (siz + 40, 0x8f),
        ] {
            let mut changed = data.clone();
            changed[offset] = value;
            let mut p = checks::Probe::with_format(irreversible::format(16, nc));
            assert!(checks::decode(&changed, &mut p, DecodeLimits::default()).is_err());
            assert!(p.events.is_empty());
        }
    }
    // A COC overrides only component 1 to reversible 5/3. Its QCD remains scalar.
    let data = read("jp2_ex_97_rgb16_noise_mct0.j2k");
    let cod = data.windows(2).position(|v| v == [255, 0x52]).unwrap();
    let sot = data.windows(2).position(|v| v == [255, 0x90]).unwrap();
    let coc = [&[255, 0x53, 0, 9, 1, 0][..], &data[cod + 9..cod + 13], &[1]].concat();
    let changed = [&data[..sot], &coc, &data[sot..]].concat();
    let mut p = checks::Probe::with_format(PixelFormatEx::Rgba16);
    assert!(checks::decode(&changed, &mut p, DecodeLimits::default()).is_err());
    assert!(p.events.is_empty());
}
