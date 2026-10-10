#![cfg(all(feature = "dds", not(feature = "noretoro")))]

use wml2::draw::{EncodeOptions, ImageBuffer, image_load};

fn encode(
    width: usize,
    height: usize,
    pixels: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut source = ImageBuffer::from_buffer(width, height, pixels);
    wml2::draw::image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut source,
            options: None,
        },
        wml2::util::ImageFormat::Dds,
    )
}

#[test]
fn legacy_dds_header_and_bgra_storage_preserve_all_channels() {
    let pixels = vec![1, 2, 3, 0, 11, 12, 13, 127, 21, 22, 23, 255, 31, 32, 33, 1];
    let encoded = encode(2, 2, pixels.clone()).unwrap();
    assert_eq!(encoded.len(), 128 + pixels.len());
    let word = |offset| u32::from_le_bytes(encoded[offset..offset + 4].try_into().unwrap());
    assert_eq!(&encoded[..4], b"DDS ");
    assert_eq!(word(4), 124);
    assert_eq!(word(8), 0x100f);
    assert_eq!(word(20), 8);
    assert_eq!(word(80), 0x41);
    assert_eq!(word(108), 0x1000);
    assert_eq!(
        &encoded[128..],
        &[3, 2, 1, 0, 13, 12, 11, 127, 23, 22, 21, 255, 33, 32, 31, 1]
    );
    let decoded = image_load(&encoded).unwrap();
    assert_eq!(decoded.buffer.unwrap(), pixels);
}

#[test]
fn rejects_legacy_surface_types_instead_of_discarding_faces_or_slices() {
    let original = encode(1, 1, vec![1, 2, 3, 255]).unwrap();
    for caps2 in [0x200u32, 0x400, 0x200000] {
        let mut encoded = original.clone();
        encoded[112..116].copy_from_slice(&caps2.to_le_bytes());
        let error = image_load(&encoded).err().unwrap();
        assert!(error.to_string().contains("Unsupported DDS legacy"));
    }
    let mut volume = original;
    volume[8..12].copy_from_slice(&0x80100fu32.to_le_bytes());
    assert!(image_load(&volume).is_err());
}

#[test]
fn validates_dimensions_and_missing_source_rows() {
    assert!(encode(0, 1, vec![]).is_err());
    assert!(encode(1, 0, vec![]).is_err());
    assert!(encode(u32::MAX as usize, 1, vec![]).is_err());
    assert!(encode(1, 1, vec![1, 2, 3]).is_err());
}

#[test]
fn callback_rows_must_be_present_and_exactly_rgba8() {
    use std::collections::HashMap;
    use wml2::draw::{EncoderOptions, EndOptions, ImageProfiles, PickCallback, PickOptions};
    use wml2::metadata::DataMap;
    type Error = Box<dyn std::error::Error>;
    struct Source {
        row: Option<Vec<u8>>,
        ended: bool,
    }
    impl PickCallback for Source {
        fn encode_start(
            &mut self,
            _: Option<EncoderOptions>,
        ) -> Result<Option<ImageProfiles>, Error> {
            Ok(Some(ImageProfiles {
                width: 1,
                height: 1,
                background: None,
                metadata: None,
            }))
        }
        fn encode_pick(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            _: Option<PickOptions>,
        ) -> Result<Option<Vec<u8>>, Error> {
            assert_eq!((x, y, w, h), (0, 0, 1, 1));
            Ok(self.row.clone())
        }
        fn encode_end(&mut self, _: Option<EndOptions>) -> Result<(), Error> {
            self.ended = true;
            Ok(())
        }
        fn metadata(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error> {
            Ok(None)
        }
    }
    for row in [None, Some(vec![0; 3]), Some(vec![0; 5]), Some(vec![0; 4])] {
        let valid = row.as_ref().is_some_and(|v| v.len() == 4);
        let mut source = Source { row, ended: false };
        let result = wml2::dds::encode(&mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut source,
            options: None,
        });
        assert_eq!(result.is_ok(), valid);
        assert_eq!(source.ended, valid);
    }
}

#[test]
#[ignore = "Writes independent-decoder/display fixture to WML2_DDS_ARTIFACT"]
fn export_interoperability_sample() {
    let path = std::env::var("WML2_DDS_ARTIFACT").unwrap();
    let pixels = (0..64)
        .flat_map(|y| {
            (0..96).flat_map(move |x| {
                [
                    (x * 255 / 95) as u8,
                    (y * 255 / 63) as u8,
                    ((x + y) * 255 / 158) as u8,
                    ((x * 3 + y * 5) % 256) as u8,
                ]
            })
        })
        .collect();
    std::fs::write(path, encode(96, 64, pixels).unwrap()).unwrap();
}
