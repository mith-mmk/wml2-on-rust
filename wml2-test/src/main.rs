#[cfg(feature="parallel")]
use std::thread::JoinHandle;
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

pub fn main()-> Result<(),Box<dyn Error>> {
    wml_test()?;
    Ok(())
}

fn loader(filename: std::path::PathBuf) {
    println!("decode {:?}", filename);

    let f = File::open(&filename);
    match f {
        Ok(f) => {
            let reader = BufReader::new(f);
            let now = Instant::now();
            let mut image = ImageBuffer::new();
            image.set_verbose(write_log);
            let mut option = DecodeOptions {
                debug_flag: 0x00,
                drawer: &mut image,
            };
            let r = image_reader(reader, &mut option);
            let eslaped_time = now.elapsed();
            match r {
                Ok(..) => {
                    println! ("{:?} {} ms",filename,eslaped_time.as_millis());
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
}

#[cfg(not(feature="parallel"))]
fn wml_test() -> Result<(),Box<dyn Error>>{
    let path = dotenv::var("IMAGEPATH")?;
    println!("read");
    let dir = fs::read_dir(path)?;
    for file in dir {
        let filename = file?.path();
        loader(filename);
    }
    Ok(())
}

#[cfg(feature="parallel")]
fn wml_test() -> Result<(),Box<dyn Error>>{
    let path = dotenv::var("IMAGEPATH")?;
    println!("read");
    let dir = fs::read_dir(path)?;
    let mut handles:Vec<JoinHandle<()>> = Vec::new();
    for file in dir {
        let filename = file?.path();
        println!("decode {:?}", filename);

        let handle = std::thread::spawn(move || {
            loader(filename);
            ()         
        });

        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join();
    }
    Ok(())
}