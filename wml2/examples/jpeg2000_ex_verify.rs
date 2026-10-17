//! Export native Gray12/16 EX bytes and time complete public-API decodes.
use std::{error::Error, time::Instant};
use wml2::draw::{PixelFormatEx, PrecisionConversion, image_from_ex};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 4 {
        return Err("usage: jpeg2000_ex_verify INPUT OUTPUT.u16le 12|16 [iterations]".into());
    }
    let format = match args[3].as_str() {
        "12" => PixelFormatEx::Gray12,
        "16" => PixelFormatEx::Gray16,
        _ => return Err("depth must be12 or16".into()),
    };
    let iterations: usize = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(1);
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let bytes = std::fs::read(&args[1])?;
    let decode = || image_from_ex(&bytes, format, PrecisionConversion::Exact);
    let image = decode()?;
    std::fs::write(&args[2], image.bytes().ok_or("missing pixels")?)?;
    for _ in 0..10 {
        std::hint::black_box(decode()?);
    }
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(decode()?);
    }
    println!(
        "{}x{}, depth{}, {} decodes, {:.3} us/decode",
        image.width(),
        image.height(),
        format.bits(),
        iterations,
        start.elapsed().as_secs_f64() * 1e6 / iterations as f64
    );
    Ok(())
}
