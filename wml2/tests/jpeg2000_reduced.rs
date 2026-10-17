#![cfg(feature = "jpeg2000")]
#[path = "../../wml2-test/scripts/jpeg2000_reduced_checks.rs"]
mod checks;
use std::path::{Path, PathBuf};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(root().join(name)).unwrap()
}

#[test]
fn native_resolution_levels_match_independent_openjpeg_references() {
    let manifest = std::fs::read_to_string(root().join("jpeg2000-reduced-manifest.txt")).unwrap();
    let refs = read("jpeg2000-reduced-refs.bin");
    let mut count = 0;
    let mut maxima = [0; 3];
    for line in manifest.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        let offset: usize = f[1].parse().unwrap();
        let length: usize = f[2].parse().unwrap();
        let bits: u8 = f[3].parse().unwrap();
        let nc: usize = f[4].parse().unwrap();
        let reduce = f[5].parse().unwrap();
        let w = f[6].parse().unwrap();
        let h = f[7].parse().unwrap();
        let data = read(f[0]);
        let reference = &refs[offset..offset + length];
        let stats = if bits == 8 {
            checks::legacy_pixels(&data, reference, reduce, w, h)
        } else {
            #[cfg(feature = "image-buffer-ex")]
            {
                checks::ex_pixels(&data, reference, reduce, w, h, bits, nc)
            }
            #[cfg(not(feature = "image-buffer-ex"))]
            {
                let _ = nc;
                continue;
            }
        };
        maxima[match bits {
            8 => 0,
            12 => 1,
            _ => 2,
        }] = maxima[match bits {
            8 => 0,
            12 => 1,
            _ => 2,
        }]
        .max(stats.maximum);
        if !f[0].contains("_97") && !f[0].ends_with("_1.jp2") {
            assert_eq!(stats.maximum, 0, "{} reduce{}", f[0], reduce);
        }
        count += 1;
    }
    #[cfg(feature = "image-buffer-ex")]
    assert_eq!(count, 1188);
    #[cfg(not(feature = "image-buffer-ex"))]
    assert_eq!(count, 108);
    eprintln!("reduced references{count}, max8/12/16={maxima:?}");
}
#[test]
fn explicit_zero_preserves_legacy_profile_and_pixels() {
    let manifest = std::fs::read_to_string(root().join("jpeg2000-general-manifest.txt")).unwrap();
    for line in manifest.lines() {
        checks::legacy_zero(&read(line.split_whitespace().next().unwrap()));
    }
}
#[cfg(feature = "image-buffer-ex")]
#[test]
fn explicit_zero_preserves_all_400_ex_inputs() {
    let mut count = 0;
    for group in ["gray", "color", "97"] {
        let m = std::fs::read_to_string(root().join(format!("jpeg2000-ex-{group}-manifest.txt")))
            .unwrap();
        for line in m.lines() {
            let f: Vec<_> = line.split_whitespace().collect();
            checks::ex_zero(
                &read(f[0]),
                f[2].parse().unwrap(),
                if group == "gray" {
                    1
                } else {
                    f[5].parse().unwrap()
                },
            );
            count += 1;
        }
    }
    assert_eq!(count, 400);
}
#[test]
fn callback_limits_strides_conversion_and_large_origins() {
    for bits in [8, 12, 16] {
        for nc in [1, 4] {
            for wavelet in ["53", "97"] {
                let data = read(&format!("jp2_reduced_odd_{bits}b_{nc}c_{wavelet}.jp2"));
                if bits == 8 {
                    checks::legacy_contracts(&data, 1);
                    checks::shifted_origins(&data, 1, bits, nc);
                }
                #[cfg(feature = "image-buffer-ex")]
                if bits != 8 {
                    checks::ex_contracts(&data, 1, bits, nc);
                    checks::shifted_origins(&data, 1, bits, nc);
                }
            }
        }
    }
}
#[test]
fn excessive_reduction_and_truncated_or_unsupported_inputs_are_atomic() {
    for bits in [8, 12, 16] {
        let data = read(&format!("jp2_reduced_odd_{bits}b_4c_97.jp2"));
        for reduce in [4, 32, 33, 255] {
            checks::rejects_before_callbacks(&data, reduce, bits);
        }
        for end in [0, 1, 2, 16, data.len() / 2, data.len() - 2, data.len() - 1] {
            checks::rejects_before_callbacks(&data[..end], 1, bits);
        }
    }
    // Raw four-component data has no JP2 straight-alpha declaration.
    let rgba = read("jp2_reduced_odd_8b_4c_53.jp2");
    let at = rgba.windows(4).position(|v| v == b"jp2c").unwrap();
    let length = u32::from_be_bytes(rgba[at - 4..at].try_into().unwrap()) as usize;
    checks::rejects_before_callbacks(&rgba[at + 4..at - 4 + length], 1, 8);
    for file in ["jp2_general_LA.jp2", "jp2_general_ht_one.j2k"] {
        checks::rejects_before_callbacks(&read(file), 1, 8);
    }
}
