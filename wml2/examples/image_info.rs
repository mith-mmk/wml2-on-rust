//! List basic header information without allocating decoded pixels.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args_os().skip(1) {
        let mut file = std::fs::File::open(&path)?;
        let info = wml2::image_info::image_info_from_reader(&mut file)?;
        println!("{}: {info:#?}", std::path::Path::new(&path).display());
    }
    Ok(())
}
