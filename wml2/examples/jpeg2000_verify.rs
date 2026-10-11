//! Decode through the public API, export PPM for inspection, and time decoding.
//! `cargo run -p wml2 --release --example jpeg2000_verify --features jpeg2000 -- INPUT OUTPUT.ppm [iterations]`
use std::{error::Error, io::Write, time::Instant};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err("usage: jpeg2000_verify INPUT OUTPUT.ppm [iterations]".into());
    }
    let data = std::fs::read(&args[1])?;
    let image = wml2::draw::image_load(&data)?;
    let rgba = image.buffer.as_ref().ok_or("missing pixels")?;
    let mut output = std::fs::File::create(&args[2])?;
    write!(output, "P6\n{} {}\n255\n", image.width, image.height)?;
    for pixel in rgba.chunks_exact(4) {
        output.write_all(&pixel[..3])?;
    }
    let iterations: u32 = args.get(3).map(|n| n.parse()).transpose()?.unwrap_or(10000);
    if iterations == 0 {
        return Err("iterations must be positive".into());
    }
    for _ in 0..100 {
        std::hint::black_box(wml2::draw::image_load(&data)?);
    }
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(wml2::draw::image_load(&data)?);
    }
    println!(
        "{}x{}, {} decodes, {:.3} us/decode",
        image.width,
        image.height,
        iterations,
        start.elapsed().as_secs_f64() * 1e6 / f64::from(iterations)
    );
    Ok(())
}
