#![cfg(all(feature = "dds", not(feature = "noretoro")))]

use wml2::draw::{image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;

fn dx10(format: u32, alpha_mode: u32, payload: &[u8]) -> Vec<u8> {
    let mut data = vec![0; 148];
    data[..4].copy_from_slice(b"DDS ");
    data[84..88].copy_from_slice(b"DX10");
    for (offset, value) in [
        (4, 124),
        (8, 0x1007),
        (12, 1),
        (16, 1),
        (76, 32),
        (80, 4),
        (108, 0x1000),
        (128, format),
        (132, 3),
        (140, 1),
        (144, alpha_mode),
    ] {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    data.extend_from_slice(payload);
    data
}

#[test]
fn dx10_alpha_modes_are_interpreted_before_legacy_rgba_callbacks() {
    for (format, pixel) in [
        (28, [64, 32, 16, 128]),
        (29, [64, 32, 16, 128]),
        (87, [16, 32, 64, 128]),
        (91, [16, 32, 64, 128]),
    ] {
        for (mode, expected) in [
            (0, [64, 32, 16, 128]),
            (1, [64, 32, 16, 128]),
            (2, [128, 64, 32, 128]),
            (3, [64, 32, 16, 255]),
        ] {
            assert_eq!(
                image_load(&dx10(format, mode, &pixel))
                    .unwrap()
                    .buffer
                    .unwrap(),
                expected
            );
        }
    }
    assert_eq!(
        image_load(&dx10(28, 2, &[255, 25, 1, 0]))
            .unwrap()
            .buffer
            .unwrap(),
        [0; 4]
    );
    assert_eq!(
        image_load(&dx10(28, 2, &[255, 25, 1, 1]))
            .unwrap()
            .buffer
            .unwrap(),
        [255, 255, 255, 1]
    );
    assert_eq!(
        image_load(&dx10(28, 3, &[255, 25, 1, 0]))
            .unwrap()
            .buffer
            .unwrap(),
        [255, 25, 1, 255]
    );
}

#[test]
fn compressed_dx10_alpha_is_normalized_once() {
    // BC2: alpha8/15, red endpoint 16/31 (bit-replicated to 132), all indices0.
    let block = [
        0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0, 0x80, 0, 0, 0, 0, 0, 0,
    ];
    let base = image_load(&dx10(74, 1, &block)).unwrap().buffer.unwrap();
    assert_eq!(base, [132, 0, 0, 136]);
    assert_eq!(
        image_load(&dx10(74, 2, &block)).unwrap().buffer.unwrap(),
        [248, 0, 0, 136]
    );
    assert_eq!(
        image_load(&dx10(74, 3, &block)).unwrap().buffer.unwrap(),
        [132, 0, 0, 255]
    );
    let block = &include_bytes!("fixtures/dds-bc7-targeted.dds")[148 + 43 * 16..148 + 44 * 16];
    let base = image_load(&dx10(98, 1, block)).unwrap().buffer.unwrap();
    assert_eq!(base, [37, 93, 150, 122]);
    assert_eq!(
        image_load(&dx10(98, 2, block)).unwrap().buffer.unwrap(),
        [77, 194, 255, 122]
    );
}

#[test]
fn custom_and_reserved_alpha_are_rejected_before_drawing() {
    use wml2::draw::*;
    use wml2::metadata::DataMap;
    struct Untouched;
    impl DrawCallback for Untouched {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            panic!("unexpected init")
        }
        fn draw(
            &mut self,
            _: usize,
            _: usize,
            _: usize,
            _: usize,
            _: &[u8],
            _: Option<DrawOptions>,
        ) -> Response {
            panic!("unexpected draw")
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            panic!("unexpected terminate")
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            panic!("unexpected next")
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            panic!("unexpected verbose")
        }
        fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
            panic!("unexpected metadata")
        }
    }
    for flags in [4, 5, 6, 7, 8, 0x80000001, u32::MAX] {
        let data = dx10(28, flags, &[1, 2, 3, 4]);
        let mut reader = bin_rs::reader::BytesReader::new(&data);
        let error = image_decoder(
            &mut reader,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut Untouched,
            },
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("alpha mode"));
    }
    let mut data = dx10(28, 0, &[]);
    data.truncate(144);
    assert!(image_load(&data).is_err());
}

#[test]
fn huge_compressed_end_is_checked_before_payload_access_or_allocation() {
    let mut data = dx10(98, 0, &[]);
    data[12..16].copy_from_slice(&1073741800u32.to_le_bytes());
    let error = image_from_with_limits(&data, DecodeLimits::unlimited())
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains(if cfg!(target_pointer_width = "32") {
                "offset overflow"
            } else {
                "block data is truncated"
            })
    );
}
