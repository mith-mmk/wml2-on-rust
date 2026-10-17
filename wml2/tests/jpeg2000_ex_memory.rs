#![cfg(all(feature = "jpeg2000", feature = "image-buffer-ex"))]
#[path = "../../wml2-test/scripts/jpeg2000_ex_memory_checks.rs"]
mod checks;
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
fn all_400_native_owned_streams_keep_exact_callback_pixels_declarations_and_policy() {
    let mut count = 0;
    for name in ["gray", "color", "97"] {
        let manifest =
            std::fs::read_to_string(root().join(format!("jpeg2000-ex-{name}-manifest.txt")))
                .unwrap();
        for line in manifest.lines() {
            let f: Vec<_> = line.split_whitespace().collect();
            let bits = f[2].parse().unwrap();
            let nc = if name == "gray" {
                1
            } else {
                f[5].parse().unwrap()
            };
            checks::native(&read(f[0]), bits, nc);
            count += 1;
        }
    }
    assert_eq!(count, 400);
}

#[test]
fn conversion_and_all_limits_keep_callback_acceptance_and_errors() {
    for bits in [12, 16] {
        for (kind, nc) in [("gray", 1), ("rgb", 3), ("rgba", 4)] {
            let data = read(&format!(
                "jp2_ex_97_{kind}{bits}_edges_mct{}.jp2",
                if nc == 1 { 0 } else { 1 }
            ));
            checks::conversions_and_limits(&data, bits, nc, 9, 7);
        }
    }
}

#[test]
fn invalid_truncated_and_unsupported_owned_inputs_keep_rejection() {
    for name in [
        "jp2_ex_97_gray16_noise_mct0.j2k",
        "jp2_ex_97_rgba12_noise_mct1.jp2",
        "jp2_ex_97_rgba16_noise_mct1.jp2",
    ] {
        let data = read(name);
        for end in [
            0,
            1,
            2,
            8,
            16,
            data.len() / 2,
            data.len() - 2,
            data.len() - 1,
        ] {
            checks::compare(
                &data[..end],
                PixelFormatEx::Rgba16,
                PrecisionConversion::Round,
                DecodeLimits::default(),
            );
        }
        let mut trailing = data.clone();
        trailing.push(1);
        assert!(
            image_from_ex(&trailing, PixelFormatEx::Rgba16, PrecisionConversion::Round).is_err()
        );
        checks::compare(
            &trailing,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Round,
            DecodeLimits::default(),
        );
    }
    for name in [
        "jp2_ex_unsupported_signed16.j2k",
        "jp2_ex_unsupported_roi16.jp2",
        "jp2_ex_color_unsupported_4_signed16.jp2",
        "jp2_ex_color_unsupported_3_depth10.jp2",
    ] {
        let data = read(name);
        assert!(image_from_ex(&data, PixelFormatEx::Rgba16, PrecisionConversion::Round).is_err());
        checks::compare(
            &data,
            PixelFormatEx::Rgba16,
            PrecisionConversion::Round,
            DecodeLimits::default(),
        );
    }
}

#[test]
fn owned_large_origin_and_unlimited_overflow_keep_native_and_32bit_samples() {
    for bits in [12, 16] {
        for (kind, nc) in [("gray", 1), ("rgb", 3), ("rgba", 4)] {
            checks::origins(
                &read(&format!("jp2_ex_97_{kind}{bits}_edges_mct0.jp2")),
                bits,
                nc,
            );
        }
    }
}
