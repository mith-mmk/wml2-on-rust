use icc_profile::DecodedICCProfile;
// use icc_profile crate from "https://github.com/mith-mmk/icc_profile"
use encoding_rs::SHIFT_JIS;
use std::env;
use std::error::Error;
use wml2::draw::*;
use wml2::metadata::c2pa::{C2PA_JSON_KEY, C2PA_RAW_KEY, c2pa_to_text};
use wml2::metadata::exif::gps_coordinate;
use wml2::metadata::{DataMap, Metadata, json_pretty};

fn read_metadata(filename: &str) -> Result<Option<Metadata>, Box<dyn Error>> {
    let mut image = image_from_file(filename.to_string())?;
    Ok(image.metadata()?)
}

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

    let metadata = read_metadata(&filename)?;
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
                    println!("{}", json_pretty(&string));
                    println!("================ {} JSON END ===============", key);
                } else {
                    println!("=============== {} START ==============", key);
                    println!("{}", c2pa_to_text(&string));
                    println!("================ {} END ===============", key);
                }
            }
            DataMap::JSON(string) => {
                println!("=============== {} JSON START ==============", key);
                println!("{}", json_pretty(&string));
                println!("================ {} JSON END ===============", key);
            }
            DataMap::Ascii(string) => {
                println!("{}: {}", key, string);
            }
            DataMap::Exif(value) => {
                println!("=============== EXIF START ==============");
                let string = value.to_string();
                println!("{}", string);
                if let Some(gps) = gps_coordinate(&value) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn put_le16(buffer: &mut [u8], offset: usize, value: u16) {
        buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn pcx_2x1() -> Vec<u8> {
        let mut data = vec![0u8; 128];
        data[0] = 0x0a;
        data[1] = 5;
        data[2] = 1;
        data[3] = 8;
        data[65] = 1;
        put_le16(&mut data, 8, 1);
        put_le16(&mut data, 66, 2);
        data.extend_from_slice(&[0, 1]);
        data.push(12);
        let mut palette = vec![0u8; 768];
        palette[0..3].copy_from_slice(&[255, 0, 0]);
        palette[3..6].copy_from_slice(&[0, 255, 0]);
        data.extend_from_slice(&palette);
        data
    }

    fn tga_2x1() -> Vec<u8> {
        let mut data = vec![0u8; 18];
        data[2] = 2;
        put_le16(&mut data, 12, 2);
        put_le16(&mut data, 14, 1);
        data[16] = 24;
        data.extend_from_slice(&[0, 0, 255, 0, 255, 0]);
        data
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wml2-metadata-{name}-{}-{unique}.tmp",
            std::process::id()
        ))
    }

    fn assert_shape_and_format(path: &PathBuf, bytes: Vec<u8>, format: &str) {
        fs::write(path, bytes).unwrap();
        let metadata = read_metadata(path.to_str().unwrap()).unwrap().unwrap();
        assert!(matches!(metadata.get("Format"), Some(DataMap::Ascii(value)) if value == format));
        assert!(matches!(metadata.get("width"), Some(DataMap::UInt(2))));
        assert!(matches!(metadata.get("height"), Some(DataMap::UInt(1))));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn metadata_reports_pcx_shape_and_format() {
        assert_shape_and_format(&unique_temp_path("pcx"), pcx_2x1(), "PCX");
    }

    #[test]
    fn metadata_reports_tga_shape_and_format() {
        assert_shape_and_format(&unique_temp_path("tga"), tga_2x1(), "TGA");
    }
}
