#![cfg(feature = "jpeg-xl")]
use wml2::draw::image_from_with_limits;
use wml2::jpegxl::{JxlResourceLimits, image_from_with_resource_limits};
use wml2::limits::DecodeLimits;
const SMALL: &[u8] = include_bytes!("fixtures/review/jxl-rgba8.jxl");
const REFERENCE: &[u8] = include_bytes!("fixtures/review/jxl-rgba8-djxl.bin");
fn policy() -> JxlResourceLimits {
    JxlResourceLimits::default()
}

#[test]
fn bounded_ordinary_matches_independent_pixels_and_legacy_contract() {
    let image = image_from_with_resource_limits(SMALL, DecodeLimits::default(), policy()).unwrap();
    assert_eq!((image.width, image.height), (7, 5));
    assert_eq!(image.buffer.as_deref().unwrap(), REFERENCE);
    assert_eq!(
        image.metadata,
        image_from_with_limits(SMALL, DecodeLimits::default())
            .unwrap()
            .metadata
    );
}

#[test]
fn policies_only_tighten_and_workspace_must_be_finite() {
    let defaults = policy();
    for invalid in [
        JxlResourceLimits {
            workspace_bytes: usize::MAX,
            ..defaults
        },
        JxlResourceLimits {
            internal_pixels: defaults.internal_pixels + 1,
            ..defaults
        },
        JxlResourceLimits {
            effective_pixels: defaults.effective_pixels + 1,
            ..defaults
        },
        JxlResourceLimits {
            internal_layers: 65,
            ..defaults
        },
        JxlResourceLimits {
            toc_entries: 4097,
            ..defaults
        },
        JxlResourceLimits {
            icc_coded_bytes: defaults.icc_coded_bytes + 1,
            ..defaults
        },
        JxlResourceLimits {
            icc_expanded_bytes: defaults.icc_expanded_bytes + 1,
            ..defaults
        },
        JxlResourceLimits {
            brob_bytes: defaults.brob_bytes + 1,
            ..defaults
        },
    ] {
        let e = image_from_with_resource_limits(b"invalid", DecodeLimits::unlimited(), invalid)
            .err()
            .unwrap();
        assert!(e.to_string().contains("cannot raise shipping caps"), "{e}");
    }
    for limited in [
        JxlResourceLimits {
            workspace_bytes: 0,
            ..defaults
        },
        JxlResourceLimits {
            internal_pixels: 1,
            ..defaults
        },
        JxlResourceLimits {
            internal_layers: 0,
            ..defaults
        },
        JxlResourceLimits {
            toc_entries: 0,
            ..defaults
        },
    ] {
        assert!(
            image_from_with_resource_limits(SMALL, DecodeLimits::unlimited(), limited).is_err()
        );
    }
    assert!(
        image_from_with_resource_limits(
            SMALL,
            DecodeLimits {
                expanded_bytes: 1,
                ..DecodeLimits::unlimited()
            },
            defaults
        )
        .is_err()
    );
}

#[test]
fn effective_padding_and_expanded_icc_policies_have_independent_exact_boundaries() {
    // Existing effective minimum canvas basis is ceil(16*16*4/3), not 7*5.
    assert!(
        image_from_with_resource_limits(
            SMALL,
            DecodeLimits::unlimited(),
            JxlResourceLimits {
                effective_pixels: 342,
                internal_pixels: 256,
                ..policy()
            }
        )
        .is_ok()
    );
    let e = image_from_with_resource_limits(
        SMALL,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            effective_pixels: 341,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("padded frame pixels"), "{e}");
    let e = image_from_with_resource_limits(
        SMALL,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            internal_pixels: 255,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("internal layer pixels"), "{e}");
    let input = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");
    let size = include_bytes!("fixtures/review/jxl-gamma2.icc").len();
    assert!(
        image_from_with_resource_limits(
            input,
            DecodeLimits::unlimited(),
            JxlResourceLimits {
                icc_expanded_bytes: size,
                ..policy()
            }
        )
        .is_ok()
    );
    let e = image_from_with_resource_limits(
        input,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            icc_expanded_bytes: size - 1,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("expanded ICC policy"), "{e}");
}

#[test]
fn preview_intrinsic_and_oriented_native_precision_keep_independent_pixels() {
    let input = include_bytes!("fixtures/review/jxl-composition-preview-intrinsic.jxl");
    let expected = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-preview-oriented-icc16.rgba.zlib"
    ))
    .unwrap();
    let image = image_from_with_resource_limits(input, DecodeLimits::default(), policy()).unwrap();
    assert_eq!((image.width, image.height), (5, 7));
    assert_eq!(image.buffer.unwrap(), expected);
    let metadata = image.metadata.unwrap();
    assert!(matches!(
        metadata["orientation_applied"],
        wml2::metadata::DataMap::Bool(true)
    ));
    assert!(matches!(
        metadata["intrinsic_display_width"],
        wml2::metadata::DataMap::UInt(11)
    ));
    assert!(matches!(
        metadata["intrinsic_display_height"],
        wml2::metadata::DataMap::UInt(19)
    ));
}

#[test]
fn cumulative_output_and_collector_have_an_exact_workspace_boundary() {
    let mut low = 0;
    let mut high = 4 << 20;
    while low < high {
        let mid = low + (high - low) / 2;
        if image_from_with_resource_limits(
            SMALL,
            DecodeLimits::unlimited(),
            JxlResourceLimits {
                workspace_bytes: mid,
                ..policy()
            },
        )
        .is_ok()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let at = JxlResourceLimits {
        workspace_bytes: low,
        ..policy()
    };
    assert_eq!(
        image_from_with_resource_limits(SMALL, DecodeLimits::unlimited(), at)
            .unwrap()
            .buffer
            .as_deref()
            .unwrap(),
        REFERENCE
    );
    let error = image_from_with_resource_limits(
        SMALL,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            workspace_bytes: low - 1,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(
        error.to_string().contains("collector workspace"),
        "{low}: {error}"
    );
}

#[test]
fn internal_reference_layers_are_independent_from_one_visible_canvas() {
    let input = include_bytes!("fixtures/review/jxl-sunset-logo.jxl");
    let e = image_from_with_resource_limits(
        input,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            internal_layers: 1,
            workspace_bytes: 1 << 30,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("internal layers"), "{e}");
    let expected = miniz_oxide::inflate::decompress_to_vec_zlib(include_bytes!(
        "fixtures/review/jxl-sunset-logo.rgba.zlib"
    ))
    .unwrap();
    let image = image_from_with_resource_limits(
        input,
        DecodeLimits {
            pixels: 924 * 1386,
            frames: 1,
            ..DecodeLimits::default()
        },
        JxlResourceLimits {
            internal_layers: 2,
            ..policy()
        },
    )
    .unwrap();
    assert_eq!((image.width, image.height), (924, 1386));
    assert_eq!(image.buffer.unwrap(), expected);
    // The two-layer file has one visible canvas. Caller frame limits stay independent.
    assert!(
        image_from_with_resource_limits(
            input,
            DecodeLimits {
                frames: 0,
                ..DecodeLimits::unlimited()
            },
            policy()
        )
        .is_err()
    );
}

#[test]
fn icc_and_cumulative_brob_limits_reject_before_a_result_is_returned() {
    let icc = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");
    for limits in [
        JxlResourceLimits {
            icc_coded_bytes: 0,
            ..policy()
        },
        JxlResourceLimits {
            icc_expanded_bytes: 0,
            ..policy()
        },
    ] {
        let e = image_from_with_resource_limits(icc, DecodeLimits::unlimited(), limits)
            .err()
            .unwrap();
        assert!(e.to_string().contains("ICC"), "{e}");
    }
    let input = include_bytes!("fixtures/review/jxl-brob-metadata.jxl");
    let e = image_from_with_resource_limits(
        input,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            brob_bytes: 0,
            ..policy()
        },
    )
    .err()
    .unwrap();
    assert!(e.to_string().contains("brob"), "{e}");
    assert!(image_from_with_resource_limits(input, DecodeLimits::unlimited(), policy()).is_ok());
    // Independent C manifest: Exif30 + XML93 + empty0 + duplicate XML93.
    let at = JxlResourceLimits {
        brob_bytes: 216,
        ..policy()
    };
    assert!(image_from_with_resource_limits(input, DecodeLimits::unlimited(), at).is_ok());
    assert!(
        image_from_with_resource_limits(
            input,
            DecodeLimits::unlimited(),
            JxlResourceLimits {
                brob_bytes: 215,
                ..at
            }
        )
        .is_err()
    );
}

#[test]
fn raw_container_metadata_is_admitted_before_its_payload_copy() {
    fn boxed(out: &mut Vec<u8>, kind: &[u8; 4], bytes: &[u8]) {
        out.extend_from_slice(&((bytes.len() + 8) as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(bytes);
    }
    let mut data = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    boxed(&mut data, b"ftyp", b"jxl \0\0\0\0jxl ");
    boxed(&mut data, b"jxlc", SMALL);
    boxed(&mut data, b"test", &vec![b'x'; 65536]);
    let resources = JxlResourceLimits {
        workspace_bytes: data.len() * 4,
        ..policy()
    };
    let e = image_from_with_resource_limits(&data, DecodeLimits::unlimited(), resources)
        .err()
        .unwrap();
    assert!(e.to_string().contains("container workspace"), "{e}");
}

#[test]
fn malformed_prefixes_and_untrusted_extents_return_errors_without_partial_success() {
    for bytes in [
        SMALL,
        include_bytes!("fixtures/review/jxl-meta-brob.jxl").as_slice(),
    ] {
        for length in 0..bytes.len().min(48) {
            assert!(
                image_from_with_resource_limits(
                    &bytes[..length],
                    DecodeLimits::default(),
                    policy()
                )
                .is_err()
            );
        }
    }
    let mut data = include_bytes!("fixtures/review/jxl-meta-brob.jxl").to_vec();
    data[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(image_from_with_resource_limits(&data, DecodeLimits::unlimited(), policy()).is_err());
}

#[test]
fn published_large_image_refusals_remain_and_finite_workspace_can_admit_old_boundary() {
    let qhd = include_bytes!("fixtures/review/jxl-resource-qhd.jxl");
    for limits in [DecodeLimits::default(), DecodeLimits::unlimited()] {
        for result in [
            image_from_with_limits(qhd, limits),
            image_from_with_resource_limits(
                qhd,
                limits,
                JxlResourceLimits {
                    workspace_bytes: 1 << 30,
                    ..policy()
                },
            ),
        ] {
            let e = result.err().unwrap();
            assert!(e.to_string().contains("padded frame pixels"), "{e}");
        }
    }
    let boundary = include_bytes!("fixtures/review/jxl-resource-boundary.jxl");
    assert!(image_from_with_limits(boundary, DecodeLimits::default()).is_err());
    let image = image_from_with_resource_limits(
        boundary,
        DecodeLimits::unlimited(),
        JxlResourceLimits {
            workspace_bytes: 1 << 30,
            ..policy()
        },
    )
    .unwrap();
    assert_eq!((image.width, image.height), (2048, 1536));
    let bytes = image.buffer.unwrap();
    assert_eq!(bytes.len(), 2048 * 1536 * 4);
    assert!(bytes.chunks_exact(4).all(|p| p == [64, 128, 192, 255]));
}

#[cfg(feature = "color-management")]
#[test]
fn managed_pixels_match_independent_littlecms_and_cancel_never_returns_a_partial_image() {
    use std::cell::Cell;
    use wml2::jpegxl::{
        DecodeCancelled, image_from_srgb_with_resource_limits,
        image_from_srgb_with_resource_limits_and_cancel,
    };
    let input = include_bytes!("fixtures/review/jxl-icc-lossless.jxl");
    let reference = include_bytes!("fixtures/review/jxl-icc-lossless.srgb.rgba");
    let image =
        image_from_srgb_with_resource_limits(input, DecodeLimits::default(), policy()).unwrap();
    let pixels = image.buffer.unwrap();
    assert_eq!(pixels.len(), reference.len());
    for (a, b) in pixels.chunks_exact(4).zip(reference.chunks_exact(4)) {
        for c in 0..3 {
            assert!(a[c].abs_diff(b[c]) <= 2);
        }
        assert_eq!(a[3], b[3]);
    }
    let calls = Cell::new(0);
    image_from_srgb_with_resource_limits_and_cancel(
        input,
        DecodeLimits::default(),
        policy(),
        &|| {
            calls.set(calls.get() + 1);
            false
        },
    )
    .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = image_from_srgb_with_resource_limits_and_cancel(
            input,
            DecodeLimits::default(),
            policy(),
            &|| {
                count.set(count.get() + 1);
                count.get() == stop
            },
        );
        assert!(result.err().unwrap().is::<DecodeCancelled>());
        assert_eq!(count.get(), stop);
    }
    let invalid = JxlResourceLimits {
        workspace_bytes: usize::MAX,
        ..policy()
    };
    assert!(
        image_from_srgb_with_resource_limits_and_cancel(
            b"invalid",
            DecodeLimits::unlimited(),
            invalid,
            &|| true
        )
        .err()
        .unwrap()
        .is::<DecodeCancelled>()
    );
}
