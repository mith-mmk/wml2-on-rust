//! Local inspection helper. Use an independent oracle to validate the pixels.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let input = arguments.next().ok_or("missing Q4 input path")?;
    let output = arguments.next().ok_or("missing RGBA output path")?;
    if arguments.next().is_some() {
        return Err("unexpected argument".into());
    }
    let image = wml2::draw::image_load(&std::fs::read(input)?)?;
    let pixels = image.buffer.ok_or("missing pixel buffer")?;
    if image.width != 640 || image.height != 400 || pixels.len() != 640 * 400 * 4 {
        return Err("unexpected Q4 dimensions/storage".into());
    }
    if pixels.chunks_exact(4).any(|pixel| pixel[3] != 255) {
        return Err("unexpected Q4 alpha".into());
    }
    std::fs::write(output, pixels)?;
    eprintln!("640x400 RGBA8; compare against an independent oracle");
    Ok(())
}
