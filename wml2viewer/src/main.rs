mod app;
mod drawers;
mod filesystem;
pub mod options;
mod ui;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let image_path = parse_image_path()?;
    app::run(image_path)
}

fn parse_image_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("wml2viewer"));
    let filename = args.next().ok_or_else(|| usage_error(&program))?;

    if args.next().is_some() {
        return Err(usage_error(&program));
    }

    Ok(PathBuf::from(filename))
}

fn usage_error(program: &OsString) -> Box<dyn Error> {
    let program = Path::new(program)
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy();
    Box::new(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Usage: {program} <path>"),
    ))
}
