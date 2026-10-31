//! Baseline-compatible reproduction: link this same file against either revision.
use wml2::{draw::image_from_with_limits, limits::DecodeLimits};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let bytes=std::fs::read(&path)?;
        for (name, limits) in [("default",DecodeLimits::default()),("unlimited",DecodeLimits::unlimited())] {
            match image_from_with_limits(&bytes,limits) {
                Ok(image)=>println!("{path} {name}: OK {}x{}",image.width,image.height),
                Err(error)=>println!("{path} {name}: ERROR {error}"),
            }
        }
    }
    Ok(())
}
