use icc_profile::DecodedICCProfile; // use icc_profile crate from "https://github.com/mith-mmk/icc_profile"
use std::env;
use std::error::Error;
use wml2::draw::*;
use wml2::metadata::DataMap;
use encoding_rs::SHIFT_JIS;

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: metadata <inputfilename>");
        return Ok(());
    }

    let filename = &args[1];
    let mut image = image_from_file(filename.to_string())?;
    let metadata = image.metadata()?;
    if metadata.is_none() {
        println!("No metadata found.");
        return Ok(());
    }
    let metadata = metadata.unwrap();

    let format = metadata.get("Format").cloned().unwrap_or(DataMap::None);
    println!("Format: {:?}", format); 
    for (key, value) in metadata {
        match value {
            DataMap::None => {
                println!("{}", key);
            }
            DataMap::Raw(value) => {
                println!("{}: {}bytes", key, value.len());
            }
            DataMap::Ascii(string) => {
                println!("{}: {}", key, string);
            }
            DataMap::Exif(value) => {
                println!("=============== EXIF START ==============");
                let string = value.to_string();
                println!("{}", string);
                println!("================ EXIF END ===============");
            }
            DataMap::ICCProfile(data) => {
                println!("{}: {}bytes", key, data.len());
                let decoded = DecodedICCProfile::new(&data)?;
                let string = icc_profile::utils::decoded_print(&decoded, 0)?;
                println!("{}", string);
            }
            DataMap::I18NString(str) => {
                println!("{}: {}", key, str);
            }
            DataMap::SJISString(bytes) => {
                let (cow, _, had_errors) = SHIFT_JIS.decode(&bytes);
                if had_errors {
                    println!("{}: {}bytes (decoding error)", key, bytes.len());
                } else {
                    println!("{}: {}", key, cow);
                }
            }
            _ => {
                println!("{}: {:?}", key, value);
            }
        }
    }

    Ok(())
}
