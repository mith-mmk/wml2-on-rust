mod app;
mod configs;
mod drawers;
mod filesystem;
pub mod options;
mod ui;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    println!(
        "wml2viewer {}",
        env!("CARGO_PKG_VERSION")
    );
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    app::run(args.image_path, args.config_path)
}

struct CliArgs {
    image_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
}

fn parse_args() -> Result<CliArgs, Box<dyn Error>> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("wml2viewer"));
    let mut positional_args = Vec::new();
    let mut config_path = None;

    while let Some(arg) = args.next() {
        if let Some(path) = parse_config_equals(&arg) {
            config_path = Some(path);
            continue;
        }

        if arg == "--config" {
            let Some(path) = args.next() else {
                return Err(usage_error(&program));
            };
            config_path = Some(PathBuf::from(path));
            continue;
        }

        if is_ignorable_shell_argument(&arg) {
            continue;
        }

        positional_args.push(PathBuf::from(arg));
    }

    let image_path = pick_image_path(positional_args);

    Ok(CliArgs {
        image_path,
        config_path,
    })
}

fn parse_config_equals(arg: &OsString) -> Option<PathBuf> {
    let text = arg.to_string_lossy();
    text.strip_prefix("--config=").map(PathBuf::from)
}

fn is_ignorable_shell_argument(arg: &OsString) -> bool {
    matches!(arg.to_string_lossy().as_ref(), "/dde" | "-Embedding" | "--")
}

fn pick_image_path(args: Vec<PathBuf>) -> Option<PathBuf> {
    if args.is_empty() {
        return None;
    }

    args.iter()
        .rev()
        .find(|path| path.exists())
        .cloned()
        .or_else(|| args.into_iter().next())
}

fn usage_error(program: &OsString) -> Box<dyn Error> {
    let program = Path::new(program)
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy();
    Box::new(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("Usage: {program} [--config <path>] [path]"),
    ))
}
