use icc_profile::DecodedICCProfile; // use icc_profile crate from "https://github.com/mith-mmk/icc_profile"
use wml2::draw::*;
use wml2::metadata::DataMap;
use std::error::Error;
use std::env;


pub fn main()-> Result<(),Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: metadata <inputfilename>");
        return Ok(())
    }

    let filename = &args[1];
    let mut image = image_from_file(filename.to_string())?;
    let metadata = image.metadata()?;
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
                    print!("{:?}: ", value);
                    let string = value.to_string();
                    println!("{}", string);
                    println!("================ EXIF END ===============");
                },
                DataMap::ICCProfile(data) => {
                    println!("{}: {}bytes",key,data.len());
                    let decoded = DecodedICCProfile::new(&data)?;
                    let string = icc_profile::utils::decoded_print(&decoded, 0)?;
                    println!("{}",string);
                },
                _ => {
                    println!("{}: {:?}",key,value);
                }
            }
        }        
    }
    Ok(())
}
