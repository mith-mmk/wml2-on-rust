use icc_profile::utils::decoded_print;
use icc_profile::DecodedICCProfile;
use std::io::Write;
use std::time::Instant;
use wml2::draw::*;
use wml2::metadata::DataMap;
use std::io::BufReader;
use std::error::Error;
use wml2::draw::CallbackResponse;
use std::fs;
use std::fs::File;

#[cfg(feature="parallel")]
use std::thread::JoinHandle;


fn write_log(str: &str) -> Result<Option<CallbackResponse>,Box<dyn Error>> {
    println!("{}", str);
    Ok(None)
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
                debug_flag: 0x0,
                drawer: &mut image,
            };
            let r = image_reader(reader, &mut option);
            let eslaped_time = now.elapsed();
            match r {
                Ok(..) => {
                    println! ("{:?} {} ms",filename,eslaped_time.as_millis());
                    let metadata = image.metadata();
                    if let Ok(metadata) = metadata {
                        if let Some(metadata) = metadata {
                            for (key,value) in metadata {
                                match value {
                                    DataMap::None => {
                                        println!("{}",key);
                                    },
                                    DataMap::Raw(value) => {
                                        println!("{}: {}bytes",key,value.len());
                                    },
                                    DataMap::Ascii(string) => {
                                        println!("{}: {}",key,string);
                                    },
                                    DataMap::Exif(value) => {
                                        println!("=============== EXIF START ==============");
                                        let string = value.to_string();
                                        println!("{}", string);
                                        println!("================ EXIF END ===============");
                                    },
                                    DataMap::ICCProfile(data) => {
                                        println!("{}: {}bytes",key,data.len());
                                        let decoded = DecodedICCProfile::new(&data).unwrap();
                                        println!("=========== ICC Profile START ===========");
                                        println!("{}",decoded_print(&decoded, 0).unwrap());
                                        println!("============= ICC Profile END ===========");

                                    /*
                                        let out_path = dotenv::var("RESULTPATH");
                                        if let Ok(out_path) = out_path {
                                            let filename = filename.file_name().unwrap().to_str().unwrap();
                                            let filename = format!("{}/{}.icc",out_path,filename);
                                            println!("{}", filename);
                                            let mut f = File::create(&filename).unwrap();
                                            f.write_all(&data).unwrap();
                                            f.flush().unwrap();
                                        }
                                    */
                                    },
                                    _ => {
                                        println!("{}: {:?}",key,value);
                                    }
                                }
                            }        
                        }
                    }

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
