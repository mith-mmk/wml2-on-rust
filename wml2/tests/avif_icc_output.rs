#![cfg(feature = "avifenc")]

use std::collections::HashMap;
use wml2::draw::*;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

fn declared_profile(model: &[u8; 4]) -> Vec<u8> {
    // Structurally checked container; no transform route is needed to identify
    // the declared channels. Native retention must remain independent of CMS.
    let mut bytes = vec![0; 132];
    bytes[..4].copy_from_slice(&132u32.to_be_bytes());
    bytes[8..12].copy_from_slice(&[4, 0, 0, 0]);
    bytes[12..16].copy_from_slice(b"mntr");
    bytes[16..20].copy_from_slice(model);
    bytes[20..24].copy_from_slice(b"XYZ ");
    bytes[36..40].copy_from_slice(b"acsp");
    assert!(icc_profile::Profile::parse(&bytes).is_ok());
    bytes
}

struct Tracked {
    image: ImageBuffer,
    picks: usize,
    ends: usize,
}
impl PickCallback for Tracked {
    fn encode_start(
        &mut self,
        option: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfiles>, Box<dyn std::error::Error>> {
        self.image.encode_start(option)
    }
    fn encode_pick(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        option: Option<PickOptions>,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        self.picks += 1;
        self.image.encode_pick(x, y, w, h, option)
    }
    fn encode_end(&mut self, option: Option<EndOptions>) -> Result<(), Box<dyn std::error::Error>> {
        self.ends += 1;
        self.image.encode_end(option)
    }
    fn metadata(&mut self) -> Result<Option<HashMap<String, DataMap>>, Box<dyn std::error::Error>> {
        self.image.metadata()
    }
}

#[test]
fn ordinary_avif_rejects_known_non_rgb_icc_and_finalizes_without_picking_pixels() {
    for model in [b"GRAY", b"CMYK"] {
        for key in ["ICC Profile", "Source ICC Profile"] {
            let mut image = ImageBuffer::from_buffer(1, 1, vec![128, 128, 128, 255]);
            image
                .set_metadata(key, DataMap::ICCProfile(declared_profile(model)))
                .unwrap();
            let mut tracked = Tracked {
                image,
                picks: 0,
                ends: 0,
            };
            let error = image_encoder(
                &mut EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut tracked,
                    // The ordinary encoder cannot signal this as an override.
                    options: Some(HashMap::from([(
                        "ICC Profile".into(),
                        DataMap::ICCProfile(
                            include_bytes!("fixtures/review/linear-rgb.icc").to_vec(),
                        ),
                    )])),
                },
                ImageFormat::Avif,
            )
            .unwrap_err();
            assert!(error.to_string().contains("use the EX encoder"));
            assert_eq!(tracked.picks, 0);
            assert_eq!(tracked.ends, 1);
            assert_eq!(
                tracked.image.buffer.as_deref(),
                Some([128, 128, 128, 255].as_slice())
            );
        }
    }
}

#[test]
fn ordinary_rgb_and_opaque_metadata_remain_compatible() {
    for profile in [
        include_bytes!("fixtures/review/linear-rgb.icc").to_vec(),
        vec![1, 2, 3],
    ] {
        let mut image = ImageBuffer::from_buffer(1, 1, vec![1, 2, 3, 255]);
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(profile))
            .unwrap();
        let encoded = image_to(
            &mut image,
            ImageFormat::Avif,
            Some(HashMap::from([("speed".into(), DataMap::UInt(10))])),
        )
        .unwrap();
        assert!(!encoded.is_empty());
    }
}

#[cfg(feature = "image-buffer-ex")]
#[test]
fn ordinary_avif_rejects_gray_provenance_from_owned_and_borrowed_ex_bridges() {
    let mut native =
        ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Gray8, None, vec![128]).unwrap();
    native
        .set_metadata(
            "ICC Profile",
            DataMap::ICCProfile(declared_profile(b"GRAY")),
        )
        .unwrap();
    let mut owned = native.to_rgba8(PrecisionConversion::Exact).unwrap();
    let error = image_to(&mut owned, ImageFormat::Avif, None).unwrap_err();
    assert!(error.to_string().contains("use the EX encoder"));
    let error = image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut native.legacy_picker(PrecisionConversion::Exact),
            options: None,
        },
        ImageFormat::Avif,
    )
    .unwrap_err();
    assert!(error.to_string().contains("use the EX encoder"));
    assert_eq!(native.bytes().unwrap(), &[128]);
}
