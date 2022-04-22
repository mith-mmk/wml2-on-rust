#[cfg(feature="parallel")]
use std::thread::JoinHandle;
use std::io::Write;
use std::time::Instant;
use wml2::draw::*;
use std::io::BufReader;
use std::error::Error;
use wml2::draw::CallbackResponse;
use std::fs;
use std::fs::File;

fn write_log(str: &str) -> Result<Option<CallbackResponse>,Box<dyn Error>> {
    println!("{}", str);
    Ok(None)
}

fn crc_test() {
    let crc32 = wml2::png::utils::CRC32::new();
    let buf = [73,69,78,68];
    let result = crc32.crc32(&buf);
    println!("{:04x}",result);

}

pub fn main()-> Result<(),Box<dyn Error>> {
    wml_test()?;
//    crc_test();
    Ok(())
}

fn loader(filename: &std::path::PathBuf) -> Option<ImageBuffer> {
    println!("decode {:?}", filename);

    let f = File::open(&filename);
    match f {
        Ok(f) => {
            let reader = BufReader::new(f);
            let now = Instant::now();
            let mut image = ImageBuffer::new();
            image.set_verbose(write_log);
            let mut option = DecodeOptions {
                debug_flag: 0x01,
                drawer: &mut image,
            };
            let r = image_reader(reader, &mut option);
            let eslaped_time = now.elapsed();
            match r {
                Ok(..) => {
                    println! ("{:?} {} ms",filename,eslaped_time.as_millis());
                    return Some(image);
                },
                Err(err) => {
                    println! ("{:?} {}",filename,err);
                }
            }
        },
        Err(err) =>{
            println! ("{:?} {}",filename,err);  
        }
    }
    None
}

#[cfg(not(feature="parallel"))]
fn wml_test() -> Result<(),Box<dyn Error>>{
    let path = dotenv::var("IMAGEPATH")?;
    let out_path = dotenv::var("RESULTPATH")?;
    println!("read");
    let dir = fs::read_dir(path)?;
    for file in dir {
        let filename = file?.path();
        let image = loader(&filename);
        if let Some(mut image) = image {
            if let Some(animation) = &image.animation {
                println!("Animation frames {}",animation.len());
                for i in 0..animation.len() {
                    let layer = &animation[i];
                    println!("{}: {} {} {}x{} {}ms",i,layer.start_x,layer.start_y,layer.width,layer.height,layer.control.await_time);
                }
            }
            let mut option = EncodeOptions {
                debug_flag: 0x01,
                drawer: &mut image,
                options: None,
            };
//            let data = wml2::bmp::encoder::encode(&mut option);
            let data = wml2::png::encoder::encode(&mut option);
            if let Ok(data) = data {
                let filename = filename.file_name().unwrap().to_str().unwrap();
                let filename = format!("{}/{}.png",out_path,filename);
                println!("{}", filename);
                let mut f = File::create(&filename).unwrap();
                f.write_all(&data).unwrap();
                f.flush().unwrap();
            }
        }
        println!("");
    }
    Ok(())
}

#[cfg(feature="parallel")]
fn wml_test() -> Result<(),Box<dyn Error>>{
    let path = dotenv::var("IMAGEPATH")?;
    let out_path = dotenv::var("RESULTPATH")?;
    println!("read");
    let dir = fs::read_dir(path)?;
    let mut handles:Vec<JoinHandle<()>> = Vec::new();
    for file in dir {
        let filename = file?.path();
        println!("decode {:?}", filename);

        let handle = std::thread::spawn(move || {
            image = loader(&filename);
            if let Ok(image) = image {
                let option = EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,    
                };
                let data = wml2::bmp::encoder(option);
                if let Ok(data) = data {
                    let filename = format!("{}.bmp",filename);
                    let f = File::create(&filename).unwrap();
                    f.write_all(data).unwrap();
                    f.flush().unwrap();
                }
            }

            ()         
        });

        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join();
    }
    Ok(())
}