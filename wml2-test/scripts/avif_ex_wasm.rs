//! AVIF extension to the EX wasm32/Node smoke procedure.
use std::collections::HashMap;
use wml2::draw::*;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

fn words(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn verify_avif_ex() -> u32 {
    let data = include_bytes!(env!("WML2_AVIF_EX_FIXTURE"));
    let decoded = image_from_ex(data, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    assert_eq!(
        decoded.bytes().unwrap(),
        words(&[
            0, 4369, 65535, 65535, 21845, 34952, 4369, 0, 43690, 8738, 56797, 65535, 65535, 61166,
            52428, 30583,
        ])
    );
    let raw = words(&[1, 2, 4095, 0, 2048, 4095, 1, 4095]);
    let mut image =
        ImageBufferEx::from_bytes(2, 1, PixelFormatEx::Rgba12, None, raw.clone()).unwrap();
    let controls = Some(HashMap::from([("lossless".into(), DataMap::UInt(1))]));
    let encoded = image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: &mut image,
            output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
            options: controls.clone(),
        },
        ImageFormat::Avif,
    )
    .unwrap();
    assert_eq!(image.bytes().unwrap(), raw);
    assert_eq!(
        image_from_ex(&encoded, PixelFormatEx::Rgba12, PrecisionConversion::Round)
            .unwrap()
            .bytes()
            .unwrap(),
        raw
    );
    let mut low = ImageBufferEx::from_bytes(
        1,
        1,
        PixelFormatEx::Rgba16,
        None,
        words(&[1, 32768, 65535, 32768]),
    )
    .unwrap();
    let mut options = EncodeOptionsEx {
        debug_flag: 0,
        drawer: &mut low,
        output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
        options: controls.clone(),
    };
    assert!(image_encoder_ex(&mut options, ImageFormat::Avif).is_err());
    options.output.conversion = PrecisionConversion::Round;
    let rounded = image_encoder_ex(&mut options, ImageFormat::Avif).unwrap();
    assert_eq!(
        image_from_ex(&rounded, PixelFormatEx::Rgba12, PrecisionConversion::Round)
            .unwrap()
            .bytes()
            .unwrap(),
        words(&[0, 2048, 4095, 2048])
    );
    let mut old = ImageBuffer::from_buffer(1, 1, vec![17, 34, 51, 68]);
    let legacy = image_to(&mut old, ImageFormat::Avif, controls).unwrap();
    assert_eq!(
        image_from(&legacy).unwrap().buffer.unwrap(),
        [17, 34, 51, 68]
    );
    4
}
