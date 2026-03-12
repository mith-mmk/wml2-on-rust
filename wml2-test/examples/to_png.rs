use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
use wml2::draw::*;

enum OutputMode {
    Apng,
    Split,
}

impl OutputMode {
    fn parse(value: Option<&String>) -> Result<Self, Box<dyn Error>> {
        match value.map(|value| value.to_ascii_lowercase()) {
            None => Ok(Self::Apng),
            Some(value) if value == "apng" => Ok(Self::Apng),
            Some(value) if value == "split" => Ok(Self::Split),
            Some(value) => Err(format!("unknown output mode: {}", value).into()),
        }
    }
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: to_png <inputfilename> <output_dir> [apng|split]");
        return Ok(());
    }
    let filename = &args[1];
    let out_path = &args[2];
    let mode = OutputMode::parse(args.get(3))?;

    wml_test(filename.to_string(), out_path.to_string(), mode)?;
    Ok(())
}

fn loader(filename: &String) -> Result<ImageBuffer, Box<dyn Error>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);
    let mut image = ImageBuffer::new();
    image.set_animation(true);
    let mut option = DecodeOptions {
        debug_flag: 0x0,
        drawer: &mut image,
    };
    image_reader(reader, &mut option)?;
    Ok(image)
}

fn write_png(filename: &str, image: &mut ImageBuffer) -> Result<(), Box<dyn Error>> {
    let mut option = EncodeOptions {
        debug_flag: 0x01,
        drawer: image,
        options: None,
    };
    let data = wml2::png::encoder::encode(&mut option)?;
    println!("{}", filename);
    let mut f = File::create(filename)?;
    f.write_all(&data)?;
    f.flush()?;
    Ok(())
}

fn write_split_frames(
    out_path: &str,
    old_filename: &str,
    animation: &[AnimationLayer],
) -> Result<(), Box<dyn Error>> {
    println!("Animation frames {}", animation.len());
    for i in 0..animation.len() {
        let layer = &animation[i];
        println!(
            "{}: {} {} {}x{} {}ms blend {:?} dispose {:?}",
            i,
            layer.start_x,
            layer.start_y,
            layer.width,
            layer.height,
            layer.control.await_time,
            layer.control.blend,
            layer.control.dispose_option
        );
        let mut image = ImageBuffer::from_buffer(layer.width, layer.height, layer.buffer.to_vec());
        let filename = format!("{}/{}_{:03}.png", out_path, old_filename, i);
        write_png(&filename, &mut image)?;
    }
    Ok(())
}

fn wml_test(filename: String, out_path: String, mode: OutputMode) -> Result<(), Box<dyn Error>> {
    let mut image = loader(&filename)?;
    let path_buf = PathBuf::from(filename);
    let old_filename = path_buf.file_name().unwrap().to_string_lossy().into_owned();

    if let Some(animation) = &image.animation {
        match mode {
            OutputMode::Apng => {
                let filename = format!("{}/{}.png", out_path, old_filename);
                println!("Animation output mode: apng");
                write_png(&filename, &mut image)?;
            }
            OutputMode::Split => {
                println!("Animation output mode: split");
                write_split_frames(&out_path, &old_filename, animation)?;
            }
        }
    } else {
        let filename = format!("{}/{}.png", out_path, old_filename);
        write_png(&filename, &mut image)?;
    }
    Ok(())
}
