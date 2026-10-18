//! Explicit EX storage → AVIF8/12-bit output. Requires image-buffer-ex,avifenc.
#[cfg(all(feature = "image-buffer-ex", feature = "avifenc"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use wml2::draw::*;
    use wml2::metadata::DataMap;
    use wml2::util::ImageFormat;
    let values = [
        0u16, 4369, 65535, 65535, 21845, 34952, 4369, 0, 43690, 8738, 56797, 65535, 65535, 61166,
        52428, 30583,
    ];
    let bytes: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut image = ImageBufferEx::from_bytes(2, 2, PixelFormatEx::Rgba16, None, bytes.clone())?;
    let controls = Some(HashMap::from([("lossless".into(), DataMap::UInt(1))]));
    let avif = image_encoder_ex(
        &mut EncodeOptionsEx {
            debug_flag: 0,
            drawer: &mut image,
            output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
            options: controls.clone(),
        },
        ImageFormat::Avif,
    )?;
    // Lossless codec mode and EX precision conversion are independent policies.
    let decoded = image_from_ex(&avif, PixelFormatEx::Rgba16, PrecisionConversion::Exact)?;
    assert_eq!(decoded.bytes().unwrap(), bytes);
    assert_eq!(decoded.source_format(), Some(PixelFormatEx::Rgba16));
    assert_eq!(
        decoded.metadata.as_ref().unwrap()["AVIF Source Bit Depth"],
        DataMap::UInt(12)
    );
    assert_eq!(image.bytes().unwrap(), bytes);
    if let Some(path) = std::env::args().nth(1) {
        std::fs::write(path, &avif)?;
    }

    let mut low = ImageBufferEx::from_bytes(
        1,
        1,
        PixelFormatEx::Rgba16,
        None,
        [1u16, 32768, 65535, 32768]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect(),
    )?;
    let mut options = EncodeOptionsEx {
        debug_flag: 0,
        drawer: &mut low,
        output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
        options: controls,
    };
    assert!(image_encoder_ex(&mut options, ImageFormat::Avif).is_err());
    options.output.conversion = PrecisionConversion::Round;
    let rounded = image_encoder_ex(&mut options, ImageFormat::Avif)?;
    let twelve = image_from_ex(&rounded, PixelFormatEx::Rgba12, PrecisionConversion::Round)?;
    assert_eq!(
        twelve.bytes().unwrap(),
        [0u16, 2048, 4095, 2048]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>()
    );
    // Source ICC arrives as metadata after draw. External colour processing
    // belongs in a user callback/postprocessing stage, before output picking.
    // Only set output "ICC Profile" after choosing a compatible profile and
    // preparing matching samples. This example implements no ICC transform.
    println!(
        "EX AVIF: RGBA16 storage -> explicit12-bit Exact/lossless -> RGBA16 exact; low-bit Exact rejection and Round verified"
    );
    Ok(())
}
#[cfg(not(all(feature = "image-buffer-ex", feature = "avifenc")))]
fn main() {
    eprintln!("ex_avif requires image-buffer-ex,avifenc");
}
