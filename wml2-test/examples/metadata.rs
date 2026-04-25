use icc_profile::DecodedICCProfile;
// use icc_profile crate from "https://github.com/mith-mmk/icc_profile"
use encoding_rs::SHIFT_JIS;
use std::env;
use std::error::Error;
use wml2::draw::*;
use wml2::metadata::c2pa::{C2PA_JSON_KEY, C2PA_RAW_KEY, c2pa_to_text};
use wml2::metadata::exif::gps_coordinate;
use wml2::metadata::{DataMap, json_pretty};

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let verbose_c2pa = args.iter().any(|arg| arg == "--verbose-c2pa");
    let filename = args
        .iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .cloned();
    let Some(filename) = filename else {
        println!("usage: metadata [--verbose-c2pa] <inputfilename>");
        return Ok(());
    };

    let mut image = image_from_file(filename.to_string())?;
    let metadata = image.metadata()?;
    if metadata.is_none() {
        println!("No metadata found.");
        return Ok(());
    }
    let metadata = metadata.unwrap();
    // key sorted output
    let mut keys: Vec<&String> = metadata.keys().collect();
    keys.sort();
    for key in keys {
        let value = metadata.get(key).unwrap();
        match value {
            DataMap::None => {
                println!("{}", key);
            }
            DataMap::SInt(value) => {
                println!("{}: {}", key, value);
            }
            DataMap::UInt(value) => {
                println!("{}: {}", key, value);
            }
            DataMap::Float(value) => {
                println!("{}: {}", key, value);
            }

            DataMap::Raw(_) if key == C2PA_RAW_KEY && !verbose_c2pa => {}
            DataMap::Raw(value) => {
                println!("{}: {}bytes", key, value.len());
            }
            DataMap::JSON(string) if key == C2PA_JSON_KEY => {
                if verbose_c2pa {
                    println!("=============== {} JSON START ==============", key);
                    println!("{}", json_pretty(string));
                    println!("================ {} JSON END ===============", key);
                } else {
                    println!("=============== {} START ==============", key);
                    println!("{}", c2pa_to_text(string));
                    println!("================ {} END ===============", key);
                }
            }
            DataMap::JSON(string) => {
                println!("=============== {} JSON START ==============", key);
                println!("{}", json_pretty(string));
                println!("================ {} JSON END ===============", key);
            }
            DataMap::Ascii(string) => {
                println!("{}: {}", key, string);
            }
            DataMap::Exif(value) => {
                println!("=============== EXIF START ==============");
                let string = value.to_string();
                println!("{}", string);
                if let Some(gps) = gps_coordinate(value) {
                    println!(
                        "GPS decimal: latitude={} longitude={} altitude={}",
                        gps.latitude,
                        gps.longitude,
                        gps.altitude
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "none".to_string())
                    );
                }
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
