use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use wml2::draw::{convert, image_from_file, image_to_file, ImageBuffer};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

enum OutputFormat {
    Png,
    Jpeg,
    Bmp,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.to_ascii_lowercase().as_str() {
            "png" | "apng" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "bmp" => Ok(Self::Bmp),
            _ => Err(format!("unknown output format: {}", value).into()),
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Bmp => "bmp",
        }
    }
}

struct Config {
    inputs: Vec<String>,
    output_dir: PathBuf,
    format: OutputFormat,
    quality: u64,
    split: bool,
}

impl Config {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut inputs = Vec::new();
        let mut output_dir = None;
        let mut format = OutputFormat::Png;
        let mut quality = 80;
        let mut split = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "-o" => {
                    index += 1;
                    if index >= args.len() {
                        return Err("missing value for -o".into());
                    }
                    output_dir = Some(PathBuf::from(&args[index]));
                }
                "-q" => {
                    index += 1;
                    if index >= args.len() {
                        return Err("missing value for -q".into());
                    }
                    quality = args[index].parse::<u64>()?;
                }
                "-f" | "--format" => {
                    index += 1;
                    if index >= args.len() {
                        return Err("missing value for --format".into());
                    }
                    format = OutputFormat::parse(&args[index])?;
                }
                "--split" => {
                    split = true;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown option: {}", value).into());
                }
                value => inputs.push(value.to_string()),
            }
            index += 1;
        }

        if inputs.is_empty() {
            return Err("no input files".into());
        }
        let output_dir = output_dir.ok_or("missing -o <outputfolder>")?;
        if split && !matches!(format, OutputFormat::Png) {
            return Err("--split is currently supported only for PNG output".into());
        }

        Ok(Self {
            inputs,
            output_dir,
            format,
            quality,
            split,
        })
    }

    fn encode_options(&self) -> Option<HashMap<String, DataMap>> {
        if matches!(self.format, OutputFormat::Jpeg) {
            let mut options = HashMap::new();
            options.insert("quality".to_string(), DataMap::UInt(self.quality));
            Some(options)
        } else {
            None
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config = match Config::parse(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{}", error);
            eprintln!(
                "usage: converter [inputfiles...] -o <outputfolder> [-f png|jpeg|bmp] [-q <quality>] [--split]"
            );
            return Err(error);
        }
    };

    fs::create_dir_all(&config.output_dir)?;
    let input_files = expand_inputs(&config.inputs)?;
    if input_files.is_empty() {
        return Err("no input files matched".into());
    }

    let mut failed = 0usize;
    for input_file in input_files {
        if let Err(error) = convert_one(&config, &input_file) {
            failed += 1;
            eprintln!("{}: {}", input_file.display(), error);
        }
    }

    if failed > 0 {
        return Err(format!("{} file(s) failed", failed).into());
    }

    Ok(())
}

fn convert_one(config: &Config, input_file: &Path) -> Result<(), Box<dyn Error>> {
    if matches!(config.format, OutputFormat::Png) {
        let image = image_from_file(input_file.to_string_lossy().into_owned())?;
        if config.split || should_split_png(&image) {
            return write_split_pngs(config, input_file, image);
        }
    }

    let output_file = output_path(
        &config.output_dir,
        input_file,
        config.format.extension(),
    )?;
    println!("{}", output_file.display());
    convert(
        input_file.to_string_lossy().into_owned(),
        output_file.to_string_lossy().into_owned(),
        config.encode_options(),
    )?;
    Ok(())
}

fn should_split_png(image: &ImageBuffer) -> bool {
    if let Some(metadata) = &image.metadata {
        if matches!(
            metadata.get("container"),
            Some(DataMap::Ascii(container)) if container == "DAT"
        ) {
            return true;
        }
    }

    let Some(animation) = &image.animation else {
        return false;
    };
    animation.iter().any(|frame| {
        if frame.start_x < 0 || frame.start_y < 0 {
            return true;
        }
        frame.start_x as usize + frame.width > image.width
            || frame.start_y as usize + frame.height > image.height
    })
}

fn write_split_pngs(
    config: &Config,
    input_file: &Path,
    mut image: ImageBuffer,
) -> Result<(), Box<dyn Error>> {
    let Some(animation) = &image.animation else {
        let output_file = output_path(&config.output_dir, input_file, "png")?;
        println!("{}", output_file.display());
        image_to_file(
            output_file.to_string_lossy().into_owned(),
            &mut image,
            ImageFormat::Png,
        )?;
        return Ok(());
    };

    if animation.is_empty() {
        let output_file = output_path(&config.output_dir, input_file, "png")?;
        println!("{}", output_file.display());
        image_to_file(
            output_file.to_string_lossy().into_owned(),
            &mut image,
            ImageFormat::Png,
        )?;
        return Ok(());
    }

    let base_name = input_file
        .file_name()
        .ok_or("input file has no file name")?
        .to_string_lossy()
        .into_owned();
    println!("Animation frames {}", animation.len());
    for (index, layer) in animation.iter().enumerate() {
        println!(
            "{}: {} {} {}x{} {}ms blend {:?} dispose {:?}",
            index,
            layer.start_x,
            layer.start_y,
            layer.width,
            layer.height,
            layer.control.await_time,
            layer.control.blend,
            layer.control.dispose_option
        );
        let mut frame = ImageBuffer::from_buffer(layer.width, layer.height, layer.buffer.clone());
        let output_file = config
            .output_dir
            .join(format!("{}_{:03}.png", base_name, index));
        println!("{}", output_file.display());
        image_to_file(
            output_file.to_string_lossy().into_owned(),
            &mut frame,
            ImageFormat::Png,
        )?;
    }
    Ok(())
}

fn output_path(output_dir: &Path, input_file: &Path, extension: &str) -> Result<PathBuf, Box<dyn Error>> {
    let file_name = input_file
        .file_name()
        .ok_or("input file has no file name")?
        .to_string_lossy()
        .into_owned();
    Ok(output_dir.join(format!("{}.{}", file_name, extension)))
}

fn expand_inputs(patterns: &[String]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    for pattern in patterns {
        let expanded = expand_pattern(pattern)?;
        if expanded.is_empty() {
            return Err(format!("no files matched {}", pattern).into());
        }
        for path in expanded {
            if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn expand_pattern(pattern: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if !pattern.contains('*') && !pattern.contains('?') {
        return Ok(vec![PathBuf::from(pattern)]);
    }

    let path = Path::new(pattern);
    let mut base = PathBuf::new();
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => base.push(prefix.as_os_str()),
            Component::RootDir => base.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => base.push(".."),
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
        }
    }

    if base.as_os_str().is_empty() {
        base.push(".");
    }

    let mut results = Vec::new();
    expand_component(&base, &components, 0, &mut results)?;
    Ok(results)
}

fn expand_component(
    base: &Path,
    components: &[String],
    index: usize,
    results: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    if index >= components.len() {
        results.push(base.to_path_buf());
        return Ok(());
    }

    let component = &components[index];
    if component.contains('*') || component.contains('?') {
        for entry in fs::read_dir(base)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if wildcard_match(component, &file_name) {
                expand_component(&entry.path(), components, index + 1, results)?;
            }
        }
    } else {
        expand_component(&base.join(component), components, index + 1, results)?;
    }
    Ok(())
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.as_bytes();
    let text = text.as_bytes();
    let mut p = 0usize;
    let mut t = 0usize;
    let mut star = None;
    let mut star_text = 0usize;

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            star_text = t;
        } else if let Some(star_index) = star {
            p = star_index + 1;
            star_text += 1;
            t = star_text;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}
