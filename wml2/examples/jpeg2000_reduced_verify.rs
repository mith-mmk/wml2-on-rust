//! Export native JPEG2000 resolution levels and time complete public-API decodes.
use bin_rs::reader::BytesReader;
use std::{error::Error, hint::black_box, time::Instant};
use wml2::{draw::*, jpeg2000, limits::DecodeLimits};

enum Raster {
    Legacy(ImageBuffer),
    Native(ImageBufferEx),
}
impl Raster {
    fn shape(&self) -> (usize, usize) {
        match self {
            Self::Legacy(v) => (v.width, v.height),
            Self::Native(v) => (v.width(), v.height()),
        }
    }
    fn bytes(&self) -> Result<&[u8], Box<dyn Error>> {
        match self {
            Self::Legacy(v) => v.buffer.as_deref().ok_or_else(|| "missing pixels".into()),
            Self::Native(v) => v.bytes().ok_or_else(|| "missing pixels".into()),
        }
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 6 {
        return Err("usage: jpeg2000_reduced_verify INPUT OUTPUT REDUCE 8|12|16 1|3|4 [iterations] [expanded_MiB]".into());
    }
    let reduce = args[3].parse()?;
    let bits: u8 = args[4].parse()?;
    let nc: usize = args[5].parse()?;
    if !matches!(bits, 8 | 12 | 16) || !matches!(nc, 1 | 3 | 4) {
        return Err("unsupported example depth/components".into());
    }
    let iterations: usize = args.get(6).map(|s| s.parse()).transpose()?.unwrap_or(1);
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    let mut limits = DecodeLimits::default();
    if let Some(budget) = args.get(7) {
        limits.expanded_bytes = budget
            .parse::<usize>()?
            .checked_mul(1024 * 1024)
            .ok_or("budget overflow")?;
    }
    let input = std::fs::read(&args[1])?;
    let decode = || -> Result<Raster, Box<dyn Error>> {
        if bits == 8 {
            let mut image = ImageBuffer::new();
            jpeg2000::decode_with_reduction(
                &mut BytesReader::new(&input),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,
                },
                reduce,
                limits,
            )?;
            Ok(Raster::Legacy(image))
        } else {
            let format = match (bits, nc) {
                (12, 1) => PixelFormatEx::Gray12,
                (16, 1) => PixelFormatEx::Gray16,
                (12, _) => PixelFormatEx::Rgba12,
                _ => PixelFormatEx::Rgba16,
            };
            Ok(Raster::Native(jpeg2000::image_from_ex_with_reduction(
                &input,
                format,
                PrecisionConversion::Exact,
                reduce,
                limits,
            )?))
        }
    };
    let image = decode()?;
    let (width, height) = image.shape();
    std::fs::write(&args[2], image.bytes()?)?;
    drop(image);
    for _ in 0..3 {
        black_box(decode()?);
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(decode()?);
    }
    println!(
        "{width}x{height}, depth{bits}, reduce{reduce}, {iterations} decodes, {:.3} us/decode",
        start.elapsed().as_secs_f64() * 1e6 / iterations as f64
    );
    Ok(())
}
