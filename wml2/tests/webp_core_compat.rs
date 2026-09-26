#![cfg(feature = "webp")]
use wml2::webp::encoder as local;
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, b| {
        (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

#[test]
fn fixed_external_core_matches_retained_public_api() {
    let (width, height) = (17, 9);
    let rgba: Vec<u8> = (0..width * height)
        .flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 31) as u8, 255])
        .collect();
    for z_level in [0, 6, 9] {
        let a = local::LosslessEncodingConfig { z_level };
        let left = local::encode_lossless_rgba_to_webp_with_config_and_exif(
            width,
            height,
            &rgba,
            &a,
            Some(b"exif"),
        )
        .unwrap();
        assert_eq!(
            (left.len(), fingerprint(&left)),
            if z_level == 0 {
                (356, 9601916367321148370)
            } else {
                (100, 12550818343182108879)
            }
        );
    }
    // webp-rust 0.3.2 fixes lossy chroma conversion, changing these byte snapshots.
    for (quality, method) in [(10., 0), (75., 3), (100., 6)] {
        let a = local::LossyEncodingConfig {
            quality,
            method,
            ..Default::default()
        };
        let left = local::encode_lossy_rgba_to_webp_with_config_and_exif(
            width,
            height,
            &rgba,
            &a,
            Some(b"exif"),
        )
        .unwrap();
        assert_eq!(
            (left.len(), fingerprint(&left)),
            match method {
                0 => (138, 11919863120113901756),
                3 => (260, 1669460351440240245),
                _ => (416, 12933884274185595718),
            }
        );
    }
    let mut alpha = rgba;
    for (i, pixel) in alpha.chunks_exact_mut(4).enumerate() {
        pixel[3] = (i * 11) as u8;
    }
    let left = local::encode_lossless_rgba_to_webp(width, height, &alpha).unwrap();
    assert_eq!((left.len(), fingerprint(&left)), (86, 15474106219377295356));
    let left = local::encode_lossy_rgba_to_webp(width, height, &alpha).unwrap();
    assert_eq!(
        (left.len(), fingerprint(&left)),
        (274, 12699800231815007630)
    );
}

#[test]
fn borrowed_animation_matches_legacy_and_obeys_exact_limits() {
    use wml2::draw::*;
    use wml2::limits::DecodeLimits;
    let mut image = ImageBuffer::new();
    image
        .init(
            2,
            2,
            Some(InitOptions {
                animation: true,
                background: None,
                loop_count: 3,
            }),
        )
        .unwrap();
    for color in [32, 64, 96] {
        image.next(Some(NextOptions::wait(10))).unwrap();
        image.draw(0, 0, 2, 2, &[color; 16], None).unwrap();
    }
    let exact = DecodeLimits {
        frames: 3,
        animation_bytes: 48,
        expanded_bytes: 16,
        ..Default::default()
    };
    let typed =
        image_to_with_limits(&mut image, wml2::util::ImageFormat::Webp, None, exact).unwrap();
    let legacy = image_encoder_with_limits(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image,
            options: None,
        },
        wml2::util::ImageFormat::Webp,
        exact,
    )
    .unwrap();
    assert_eq!(typed, legacy);
    for limits in [
        DecodeLimits { frames: 2, ..exact },
        DecodeLimits {
            animation_bytes: 47,
            ..exact
        },
    ] {
        assert!(
            image_to_with_limits(&mut image, wml2::util::ImageFormat::Webp, None, limits).is_err()
        );
        assert!(
            image_encoder_with_limits(
                &mut EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,
                    options: None
                },
                wml2::util::ImageFormat::Webp,
                limits
            )
            .is_err()
        );
    }
}
