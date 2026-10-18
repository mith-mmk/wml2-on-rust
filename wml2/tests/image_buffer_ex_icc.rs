#![cfg(feature = "image-buffer-ex")]

use wml2::draw::*;
use wml2::metadata::DataMap;

const OPAQUE: &[u8] = b"unsupported native Gray ICC bytes\0\xff";

fn profile<'a>(image: &'a ImageBufferEx, key: &str) -> Option<&'a [u8]> {
    match image
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get(key))
    {
        Some(DataMap::ICCProfile(bytes)) => Some(bytes),
        _ => None,
    }
}

fn declaration(format: PixelFormatEx) -> Option<InitOptionsEx> {
    Some(InitOptionsEx {
        source_format: format,
        ..Default::default()
    })
}

fn native_gray() -> ImageBufferEx {
    let mut image =
        ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray16, None, vec![0x34, 0x12]).unwrap();
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    image
}

#[test]
fn native_gray_and_depth_changes_retain_opaque_icc_without_a_cms() {
    let mut image = native_gray();
    assert_eq!(profile(&image, "ICC Profile"), Some(OPAQUE));
    assert_eq!(profile(&image, "Source ICC Profile"), None);
    let pixels = image
        .encode_pick_ex(
            0,
            0,
            1,
            1,
            PixelOutputEx::new(PixelFormatEx::Gray8, PrecisionConversion::Round),
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(pixels.bytes(), &[18]);
    assert_eq!(profile(&image, "ICC Profile"), Some(OPAQUE));
    assert_eq!(image.bytes().unwrap(), &[0x34, 0x12]);
    let mut rgb = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
    rgb.init_ex(1, 1, declaration(PixelFormatEx::Rgba12))
        .unwrap();
    rgb.set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    assert_eq!(profile(&rgb, "ICC Profile"), Some(OPAQUE));
}

#[test]
fn gray_callback_profile_is_demoted_before_or_after_storage_initialization() {
    for before in [true, false] {
        let mut image =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
        if before {
            image
                .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
                .unwrap();
        }
        image
            .init_ex(1, 1, declaration(PixelFormatEx::Gray12))
            .unwrap();
        if !before {
            image
                .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
                .unwrap();
        }
        image.draw(0, 0, 1, 1, &[1, 0], None).unwrap();
        assert_eq!(image.source_format(), Some(PixelFormatEx::Gray12));
        assert_eq!(image.format(), PixelFormatEx::Rgba16);
        assert_eq!(image.bytes().unwrap(), &[16, 0, 16, 0, 16, 0, 255, 255]);
        assert_eq!(profile(&image, "ICC Profile"), None);
        assert_eq!(profile(&image, "Source ICC Profile"), Some(OPAQUE));
    }
}

#[test]
fn owned_and_borrowed_legacy_exports_demote_gray_icc_without_mutating_native_storage() {
    let image = native_gray();
    let legacy = image.to_rgba8(PrecisionConversion::Round).unwrap();
    assert_eq!(legacy.buffer.as_deref(), Some([18, 18, 18, 255].as_slice()));
    assert!(
        !legacy
            .metadata
            .as_ref()
            .unwrap()
            .contains_key("ICC Profile")
    );
    assert_eq!(
        legacy.metadata.as_ref().unwrap()["Source ICC Profile"],
        DataMap::ICCProfile(OPAQUE.to_vec())
    );
    let imported = ImageBufferEx::try_from(legacy).unwrap();
    assert_eq!(imported.source_format(), Some(PixelFormatEx::Rgba8));
    assert_eq!(profile(&imported, "Source ICC Profile"), Some(OPAQUE));
    assert_eq!(profile(&imported, "ICC Profile"), None);
    let mut picker = image.legacy_picker(PrecisionConversion::Round);
    let metadata = picker.metadata().unwrap().unwrap();
    assert!(!metadata.contains_key("ICC Profile"));
    assert_eq!(
        metadata["Source ICC Profile"],
        DataMap::ICCProfile(OPAQUE.to_vec())
    );
    assert_eq!(
        picker
            .encode_start(None)
            .unwrap()
            .unwrap()
            .metadata
            .unwrap(),
        metadata
    );
    assert_eq!(
        picker.encode_pick(0, 0, 1, 1, None).unwrap().unwrap(),
        [18, 18, 18, 255]
    );
    assert_eq!(profile(&image, "ICC Profile"), Some(OPAQUE));
    assert_eq!(profile(&image, "Source ICC Profile"), None);
    assert_eq!(image.bytes().unwrap(), &[0x34, 0x12]);
}

#[test]
fn failed_initialization_keeps_pending_icc_and_source_collisions_preserve_both_profiles() {
    let mut image = ImageBufferEx::new();
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    assert!(
        image
            .init_ex(0, 1, declaration(PixelFormatEx::Gray8))
            .is_err()
    );
    assert_eq!(profile(&image, "ICC Profile"), Some(OPAQUE));
    assert_eq!(profile(&image, "Source ICC Profile"), None);
    image
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(vec![9, 8, 7]))
        .unwrap();
    let before = image.metadata.clone();
    assert!(
        image
            .init_ex(1, 1, declaration(PixelFormatEx::Gray8))
            .unwrap_err()
            .to_string()
            .contains("competing source profiles")
    );
    assert_eq!(image.metadata, before);
    assert!(image.bytes().is_none());
    let mut native = native_gray();
    native
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(vec![9, 8, 7]))
        .unwrap();
    let before = native.metadata.clone();
    assert!(native.to_rgba8(PrecisionConversion::Round).is_err());
    assert!(
        native
            .legacy_picker(PrecisionConversion::Round)
            .metadata()
            .is_err()
    );
    assert_eq!(native.metadata, before);
}

#[test]
fn equal_sources_deduplicate_and_competing_callback_profiles_remain_available() {
    let mut image = ImageBufferEx::new();
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    image
        .set_metadata("Source ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    image
        .init_ex(1, 1, declaration(PixelFormatEx::Gray8))
        .unwrap();
    assert_eq!(profile(&image, "ICC Profile"), None);
    assert_eq!(profile(&image, "Source ICC Profile"), Some(OPAQUE));
    // A conflicting new callback ICC stays available alongside existing source bytes.
    assert!(
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(vec![8]))
            .is_err()
    );
    assert_eq!(profile(&image, "ICC Profile"), Some([8].as_slice()));
    assert_eq!(profile(&image, "Source ICC Profile"), Some(OPAQUE));
}

#[test]
fn begin_image_clears_previous_declaration_before_preinit_metadata_but_keeps_configuration() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
    image
        .init_ex(1, 1, declaration(PixelFormatEx::Gray12))
        .unwrap();
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
        .unwrap();
    image.begin_image().unwrap();
    assert!(image.metadata.is_none());
    assert!(image.bytes().is_none());
    assert_eq!(image.source_format(), None);
    assert_eq!(image.format(), PixelFormatEx::Rgba16);
    assert_eq!(image.precision_conversion(), PrecisionConversion::Round);
    image
        .set_metadata("ICC Profile", DataMap::ICCProfile(vec![1, 2, 3]))
        .unwrap();
    image
        .init_ex(1, 1, declaration(PixelFormatEx::Rgba8))
        .unwrap();
    assert_eq!(profile(&image, "ICC Profile"), Some([1, 2, 3].as_slice()));
    assert_eq!(profile(&image, "Source ICC Profile"), None);
}

#[cfg(feature = "avifenc")]
mod avif {
    use super::*;
    use std::collections::HashMap;
    use wml2::util::ImageFormat;

    // A checked ICC container without a transform route deliberately proves
    // that channel identity checks do not require CMS compilation support.
    fn declared_gray_profile() -> Vec<u8> {
        let mut bytes = vec![0; 132];
        bytes[..4].copy_from_slice(&132u32.to_be_bytes());
        bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
        bytes[12..16].copy_from_slice(b"mntr");
        bytes[16..20].copy_from_slice(b"GRAY");
        bytes[20..24].copy_from_slice(b"XYZ ");
        bytes[36..40].copy_from_slice(b"acsp");
        assert_eq!(
            icc_profile::Profile::parse(&bytes).unwrap().color_space(),
            icc_profile::ColorSpace::Gray
        );
        bytes
    }

    fn encode(
        image: &mut dyn PickCallbackEx,
        format: PixelFormatEx,
        output_icc: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut options = HashMap::from([
            ("lossless".to_string(), DataMap::UInt(1)),
            ("speed".to_string(), DataMap::UInt(10)),
        ]);
        if let Some(profile) = output_icc {
            options.insert("ICC Profile".into(), DataMap::ICCProfile(profile));
        }
        image_encoder_ex(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: image,
                output: PixelOutputEx::new(format, PrecisionConversion::Round),
                options: Some(options),
            },
            ImageFormat::Avif,
        )
    }

    fn gray() -> ImageBufferEx {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray8, None, vec![128]).unwrap();
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(declared_gray_profile()))
            .unwrap();
        image
    }

    struct Tracked {
        image: ImageBufferEx,
        picks: usize,
        ends: usize,
    }
    impl PickCallbackEx for Tracked {
        fn encode_start_ex(
            &mut self,
            option: Option<EncoderOptions>,
        ) -> Result<Option<ImageProfilesEx>, Box<dyn std::error::Error>> {
            self.image.encode_start_ex(option)
        }
        fn encode_pick_ex(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            output: PixelOutputEx,
            option: Option<PickOptions>,
        ) -> Result<Option<PixelBytesEx>, Box<dyn std::error::Error>> {
            self.picks += 1;
            self.image.encode_pick_ex(x, y, w, h, output, option)
        }
        fn encode_end_ex(
            &mut self,
            option: Option<EndOptions>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.ends += 1;
            self.image.encode_end_ex(option)
        }
        fn metadata_ex(
            &mut self,
        ) -> Result<Option<HashMap<String, DataMap>>, Box<dyn std::error::Error>> {
            self.image.metadata_ex()
        }
    }

    #[test]
    fn incompatible_ex_icc_finalizes_once_before_picking_or_mutating_pixels() {
        let mut tracked = Tracked {
            image: gray(),
            picks: 0,
            ends: 0,
        };
        let before = tracked.image.bytes().unwrap().to_vec();
        assert!(encode(&mut tracked, PixelFormatEx::Rgba8, None).is_err());
        assert_eq!(tracked.ends, 1);
        assert_eq!(tracked.picks, 0);
        assert_eq!(tracked.image.bytes().unwrap(), before);
    }

    #[test]
    fn native_gray_icc_remains_encodable_without_compiling_a_transform() {
        let mut image = gray();
        let bytes = encode(&mut image, PixelFormatEx::Gray8, None).unwrap();
        assert!(
            avif_codec::decode_frame_bytes(&bytes)
                .unwrap()
                .color_config
                .monochrome
        );
        let decoded =
            image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        assert_eq!(
            profile(&decoded, "Source ICC Profile"),
            Some(declared_gray_profile().as_slice())
        );
        assert_eq!(
            decoded.bytes().unwrap(),
            &[128, 128, 128, 128, 128, 128, 255, 255]
        );
    }

    #[test]
    fn active_gray_icc_cannot_be_attached_to_requested_rgb_output() {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba8, None, vec![128, 128, 128, 255])
                .unwrap();
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(declared_gray_profile()))
            .unwrap();
        let before = image.bytes().unwrap().to_vec();
        let error = encode(&mut image, PixelFormatEx::Rgba8, None).unwrap_err();
        assert!(error.to_string().contains("output ICC color model"));
        assert_eq!(image.bytes().unwrap(), before);
        assert_eq!(
            profile(&image, "ICC Profile"),
            Some(declared_gray_profile().as_slice())
        );
    }

    #[test]
    fn demoted_gray_source_requires_a_compatible_explicit_rgb_override_after_both_bridges() {
        let mut expanded = ImageBufferEx::new();
        expanded
            .init_ex(1, 1, declaration(PixelFormatEx::Gray8))
            .unwrap();
        expanded.draw(0, 0, 1, 1, &[128], None).unwrap();
        expanded
            .set_metadata("ICC Profile", DataMap::ICCProfile(declared_gray_profile()))
            .unwrap();
        let bridged =
            ImageBufferEx::try_from(gray().to_rgba8(PrecisionConversion::Round).unwrap()).unwrap();
        for mut image in [expanded, bridged] {
            let before = image.bytes().unwrap().to_vec();
            let error = encode(&mut image, PixelFormatEx::Rgba8, None).unwrap_err();
            assert!(error.to_string().contains("source ICC color model"));
            assert!(encode(&mut image, PixelFormatEx::Rgba8, Some(vec![1, 2, 3])).is_err());
            let rgb = include_bytes!("fixtures/review/linear-rgb.icc").to_vec();
            let encoded = encode(&mut image, PixelFormatEx::Rgba8, Some(rgb.clone())).unwrap();
            assert!(
                !avif_codec::decode_frame_bytes(&encoded)
                    .unwrap()
                    .color_config
                    .monochrome
            );
            let decoded =
                image_from_ex(&encoded, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(
                profile(&decoded, "Source ICC Profile"),
                Some(rgb.as_slice())
            );
            assert_eq!(image.bytes().unwrap(), before);
            assert_eq!(
                profile(&image, "Source ICC Profile"),
                Some(declared_gray_profile().as_slice())
            );
        }
    }

    #[test]
    fn opaque_native_gray_icc_requires_explicit_compatible_rgb_output_on_expansion() {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray8, None, vec![128]).unwrap();
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(OPAQUE.to_vec()))
            .unwrap();
        let native = encode(&mut image, PixelFormatEx::Gray8, None).unwrap();
        let decoded =
            image_from_ex(&native, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        assert_eq!(profile(&decoded, "Source ICC Profile"), Some(OPAQUE));
        let error = encode(&mut image, PixelFormatEx::Rgba8, None).unwrap_err();
        assert!(error.to_string().contains("Gray storage ICC"));
        assert!(encode(&mut image, PixelFormatEx::Rgba8, Some(vec![1, 2, 3])).is_err());
        let rgb = include_bytes!("fixtures/review/linear-rgb.icc").to_vec();
        assert!(encode(&mut image, PixelFormatEx::Rgba8, Some(rgb)).is_ok());
        assert_eq!(profile(&image, "ICC Profile"), Some(OPAQUE));
        assert_eq!(image.bytes().unwrap(), &[128]);
    }

    #[test]
    fn known_wrong_override_rejects_but_opaque_explicit_output_stays_compatible() {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba8, None, vec![1, 2, 3, 255])
                .unwrap();
        assert!(
            encode(
                &mut image,
                PixelFormatEx::Rgba8,
                Some(declared_gray_profile())
            )
            .is_err()
        );
        let opaque = vec![1, 2, 3];
        let bytes = encode(&mut image, PixelFormatEx::Rgba8, Some(opaque.clone())).unwrap();
        let decoded =
            image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        assert_eq!(
            profile(&decoded, "Source ICC Profile"),
            Some(opaque.as_slice())
        );
    }
}
