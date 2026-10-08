//! Preserve PNG/TIFF samples as RGBA16LE and export an explicitly rounded PNG preview.
//! Run with `--features png,tiff,image-buffer-ex --example ex_decode --
//! input preview.png pixels.rgba16le`.
#[cfg(all(feature = "png", feature = "image-buffer-ex"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex, image_to};
    use wml2::util::ImageFormat;
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: ex_decode <input> <preview.png> <pixels.rgba16le>".into());
    }
    let input = std::fs::read(&args[1])?;
    let image = image_from_ex(&input, PixelFormatEx::Rgba16, PrecisionConversion::Exact)?;
    let mut preview = image.to_rgba8(PrecisionConversion::Round)?;
    let png = image_to(&mut preview, ImageFormat::Png, None)?;
    std::fs::write(&args[2], png)?;
    std::fs::write(&args[3], image.bytes().unwrap())?;
    println!(
        "{}x{} RGBA16LE; preview uses explicit Round",
        image.width(),
        image.height()
    );
    Ok(())
}

#[cfg(not(all(feature = "png", feature = "image-buffer-ex")))]
fn main() {
    eprintln!("ex_decode requires png,image-buffer-ex (and tiff for TIFF input)");
}
