use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::PathBuf;
use wml2::draw::*;
use wml2::metadata::DataMap;

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: to_jpeg <inputfilename> <output_dir> [quality]");
        return Ok(());
    }
    let filename = &args[1];
    let out_path = &args[2];
    let quality = args
        .get(3)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(80);

    wml_test(filename.to_string(), out_path.to_string(), quality)?;
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

fn encode_options(quality: u64) -> Option<HashMap<String, DataMap>> {
    let mut options = HashMap::new();
    options.insert("quality".to_string(), DataMap::UInt(quality));
    Some(options)
}

fn wml_test(filename: String, out_path: String, quality: u64) -> Result<(), Box<dyn Error>> {
    let mut image = loader(&filename)?;
    let mut option = EncodeOptions {
        debug_flag: 0x01,
        drawer: &mut image,
        options: encode_options(quality),
    };
    let data = wml2::jpeg::encoder::encode(&mut option)?;
    let path_buf = PathBuf::from(filename);
    let old_filename = path_buf.file_name().unwrap().to_string_lossy().into_owned();
    let filename = format!("{}/{}.jpg", out_path, old_filename);
    println!("{}", filename);
    let mut f = File::create(&filename)?;
    f.write_all(&data)?;
    f.flush()?;

    if let Some(animation) = &image.animation {
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
            let mut image =
                ImageBuffer::from_buffer(layer.width, layer.height, layer.buffer.to_vec());

            let mut option = EncodeOptions {
                debug_flag: 0x01,
                drawer: &mut image,
                options: encode_options(quality),
            };

            let data = wml2::jpeg::encoder::encode(&mut option);

            if let Ok(data) = data {
                let filename = format!("{}/{}_{:03}.jpg", out_path, old_filename, i);
                println!("{}", filename);
                let mut f = File::create(&filename)?;
                f.write_all(&data)?;
                f.flush()?;
            }
        }
    }
    Ok(())
}
