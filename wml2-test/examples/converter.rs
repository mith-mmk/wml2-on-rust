use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use wml2::draw::{EncodeOptions, ImageBuffer, convert, image_encoder, image_from_file};
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

enum OutputFormat {
    Png,
    Jpeg,
    Bmp,
    Webp,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value.to_ascii_lowercase().as_str() {
            "png" | "apng" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "bmp" => Ok(Self::Bmp),
            "webp" => Ok(Self::Webp),
            _ => Err(format!("unknown output format: {}", value).into()),
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Bmp => "bmp",
            Self::Webp => "webp",
        }
    }
}

struct Config {
    inputs: Vec<String>,
    output_dir: PathBuf,
    format: OutputFormat,
    quality: Option<u64>,
    optimize: Option<u64>,
    split: bool,
}

struct ExpandedInputs {
    files: Vec<PathBuf>,
    failures: Vec<String>,
}

impl Config {
    fn parse(args: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut inputs = Vec::new();
        let mut output_dir = None;
        let mut format = OutputFormat::Png;
        let mut quality = None;
        let mut optimize = None;
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
                    quality = Some(args[index].parse::<u64>()?);
                }
                "-f" | "--format" => {
                    index += 1;
                    if index >= args.len() {
                        return Err("missing value for --format".into());
                    }
                    format = OutputFormat::parse(&args[index])?;
                }
                "-z" => {
                    index += 1;
                    if index >= args.len() {
                        return Err("missing value for -z".into());
                    }
                    optimize = Some(args[index].parse::<u64>()?);
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
        if split && matches!(format, OutputFormat::Jpeg | OutputFormat::Bmp) {
            return Err("--split is currently supported only for PNG and WebP output".into());
        }

        Ok(Self {
            inputs,
            output_dir,
            format,
            quality,
            optimize,
            split,
        })
    }

    fn encode_options(&self) -> Option<HashMap<String, DataMap>> {
        let mut options = HashMap::new();
        match self.format {
            OutputFormat::Jpeg => {
                options.insert(
                    "quality".to_string(),
                    DataMap::UInt(self.quality.unwrap_or(80)),
                );
            }
            OutputFormat::Webp => {
                if let Some(quality) = self.quality {
                    options.insert("quality".to_string(), DataMap::UInt(quality));
                }
                if let Some(optimize) = self.optimize {
                    options.insert("optimize".to_string(), DataMap::UInt(optimize));
                }
            }
            _ => {}
        }
        (!options.is_empty()).then_some(options)
    }

    fn image_format(&self) -> ImageFormat {
        match self.format {
            OutputFormat::Png => ImageFormat::Png,
            OutputFormat::Jpeg => ImageFormat::Jpeg,
            OutputFormat::Bmp => ImageFormat::Bmp,
            OutputFormat::Webp => ImageFormat::Webp,
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
                "usage: converter [inputfiles...] -o <outputfolder> [-f png|jpeg|bmp|webp] [-q <quality>] [-z <0-9>] [--split]"
            );
            return Err(error);
        }
    };

    fs::create_dir_all(&config.output_dir)?;
    let expanded = expand_inputs(&config.inputs)?;
    let mut failed = expanded.failures.len();
    for failure in expanded.failures {
        eprintln!("{}", failure);
    }
    if expanded.files.is_empty() {
        if failed > 0 {
            return Err(format!("{} input pattern(s) failed", failed).into());
        }
        return Err("no input files matched".into());
    }

    for input_file in expanded.files {
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
    if matches!(config.format, OutputFormat::Png | OutputFormat::Webp) {
        let image = image_from_file(input_file.to_string_lossy().into_owned())?;
        if should_split_output(config, input_file, &image) {
            return write_split_images(config, input_file, image);
        }
    }

    let output_file = output_path(&config.output_dir, input_file, config.format.extension())?;
    println!("{}", output_file.display());
    convert(
        input_file.to_string_lossy().into_owned(),
        output_file.to_string_lossy().into_owned(),
        config.encode_options(),
    )?;
    Ok(())
}

fn should_split_output(config: &Config, input_file: &Path, image: &ImageBuffer) -> bool {
    if config.split {
        return true;
    }

    if !matches!(config.format, OutputFormat::Png) {
        return false;
    }

    if matches!(
        input_file
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("dat")
    ) {
        return true;
    }

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

fn write_encoded_image(
    output_file: &Path,
    image: &mut ImageBuffer,
    format: ImageFormat,
    options: Option<HashMap<String, DataMap>>,
) -> Result<(), Box<dyn Error>> {
    let mut encode = EncodeOptions {
        debug_flag: 0,
        drawer: image,
        options,
    };
    let data = image_encoder(&mut encode, format)?;
    fs::write(output_file, data)?;
    Ok(())
}

fn write_split_images(
    config: &Config,
    input_file: &Path,
    mut image: ImageBuffer,
) -> Result<(), Box<dyn Error>> {
    let extension = config.format.extension();
    let Some(animation) = &image.animation else {
        let output_file = output_path(&config.output_dir, input_file, extension)?;
        println!("{}", output_file.display());
        write_encoded_image(
            &output_file,
            &mut image,
            config.image_format(),
            config.encode_options(),
        )?;
        return Ok(());
    };

    if animation.is_empty() {
        let output_file = output_path(&config.output_dir, input_file, extension)?;
        println!("{}", output_file.display());
        write_encoded_image(
            &output_file,
            &mut image,
            config.image_format(),
            config.encode_options(),
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
            .join(format!("{}_{:03}.{}", base_name, index, extension));
        println!("{}", output_file.display());
        write_encoded_image(
            &output_file,
            &mut frame,
            config.image_format(),
            config.encode_options(),
        )?;
    }
    Ok(())
}

fn output_path(
    output_dir: &Path,
    input_file: &Path,
    extension: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let file_name = input_file
        .file_name()
        .ok_or("input file has no file name")?
        .to_string_lossy()
        .into_owned();
    Ok(output_dir.join(format!("{}.{}", file_name, extension)))
}

fn expand_inputs(patterns: &[String]) -> Result<ExpandedInputs, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut failures = Vec::new();
    for pattern in patterns {
        match expand_pattern(pattern) {
            Ok(expanded) => {
                let mut matched = false;
                for path in expanded {
                    if path.is_file() {
                        files.push(path);
                        matched = true;
                    }
                }
                if !matched {
                    failures.push(format!("{}: no input files matched", pattern));
                }
            }
            Err(error) => {
                failures.push(format!("{}: {}", pattern, error));
            }
        }
    }
    files.sort();
    files.dedup();
    Ok(ExpandedInputs { files, failures })
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
        let entries = match fs::read_dir(base) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(Box::new(error)),
        };
        for entry in entries {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wml2::draw::{AnimationLayer, NextOptions};

    fn unique_temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wml2-converter-{name}-{}-{unique}.tmp",
            std::process::id()
        ))
    }

    #[test]
    fn dat_inputs_always_split_for_png_output() {
        let image = ImageBuffer::from_buffer(1, 1, vec![0, 0, 0, 0xff]);
        let config = Config {
            inputs: Vec::new(),
            output_dir: PathBuf::new(),
            format: OutputFormat::Png,
            quality: None,
            optimize: None,
            split: false,
        };
        assert!(should_split_output(
            &config,
            Path::new("sample.dat"),
            &image
        ));
    }

    #[test]
    fn out_of_canvas_animation_frames_split_for_png_output() {
        let mut image = ImageBuffer::from_buffer(16, 16, vec![0; 16 * 16 * 4]);
        image.animation = Some(vec![AnimationLayer {
            width: 4,
            height: 4,
            start_x: 20,
            start_y: 0,
            buffer: vec![0; 4 * 4 * 4],
            control: NextOptions::new(),
        }]);

        let config = Config {
            inputs: Vec::new(),
            output_dir: PathBuf::new(),
            format: OutputFormat::Png,
            quality: None,
            optimize: None,
            split: false,
        };
        assert!(should_split_output(&config, Path::new("frame.gif"), &image));
    }

    #[test]
    fn explicit_split_is_available_for_webp_output() {
        let image = ImageBuffer::from_buffer(1, 1, vec![0, 0, 0, 0xff]);
        let config = Config {
            inputs: Vec::new(),
            output_dir: PathBuf::new(),
            format: OutputFormat::Webp,
            quality: None,
            optimize: Some(7),
            split: true,
        };

        assert!(should_split_output(&config, Path::new("frame.gif"), &image));
        let options = config.encode_options().unwrap();
        assert!(matches!(options.get("optimize"), Some(DataMap::UInt(7))));
    }

    #[test]
    fn expand_inputs_keeps_matching_files_when_some_patterns_fail() {
        let temp_file = unique_temp_path("expand-inputs");
        fs::write(&temp_file, b"ok").unwrap();

        let existing = temp_file.to_string_lossy().into_owned();
        let missing = unique_temp_path("missing-input")
            .to_string_lossy()
            .into_owned();
        let expanded = expand_inputs(&[existing.clone(), missing.clone()]).unwrap();

        assert_eq!(expanded.files, vec![PathBuf::from(existing)]);
        assert_eq!(
            expanded.failures,
            vec![format!("{missing}: no input files matched")]
        );

        let _ = fs::remove_file(temp_file);
    }
}
