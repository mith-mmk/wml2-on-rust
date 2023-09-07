use crate::metadata::DataMap;
use bin_rs::io::read_ascii_string;
use bin_rs::io::read_byte;
use std::collections::HashMap;

pub(crate) fn paeth_dec(d: u8, a: i32, b: i32, c: i32) -> u8 {
    let pa = (b - c).abs();
    let pb = (a - c).abs();
    let pc = (b + a - c - c).abs();
    let d = d as i32;
    if pa <= pb && pa <= pc {
        ((d + a) & 0xff) as u8
    } else if pb <= pc {
        ((d + b) & 0xff) as u8
    } else {
        ((d + c) & 0xff) as u8
    }
}

pub(crate) fn paeth_enc(d: u8, a: i32, b: i32, c: i32) -> u8 {
    let pa = (b - c).abs();
    let pb = (a - c).abs();
    let pc = (b + a - c - c).abs();
    let d = d as i32;
    if pa <= pb && pa <= pc {
        ((d - a) & 0xff) as u8
    } else if pb <= pc {
        ((d - b) & 0xff) as u8
    } else {
        ((d - c) & 0xff) as u8
    }
}

pub struct CRC32 {
    crc_table: [u32; 256],
}

impl CRC32 {
    pub fn new() -> Self {
        let mut crc_table = [0_u32; 256];
        for n in 0..256 {
            let mut c = n as u32;
            for _ in 0..8 {
                if c & 0x01 == 0x01 {
                    c = 0xedb8_8320_u32 ^ (c >> 1);
                } else {
                    c >>= 1;
                }
                crc_table[n] = c;
            }
        }
        Self { crc_table }
    }

    fn update_crc32(&self, crc: u32, buf: &[u8]) -> u32 {
        let crc_table = self.crc_table;

        let mut c = crc;
        for data in buf.iter() {
            c = crc_table[((c ^ *data as u32) & 0xff) as usize] ^ (c >> 8);
        }
        c
    }

    pub fn crc32(&self, buf: &[u8]) -> u32 {
        self.update_crc32(0xffff_ffff, buf) ^ 0xffff_ffff
    }
}

/*
fn crc_test() {
    let crc32 = wml2::png::utils::CRC32::new();
    let buf = [73,69,78,68];
    let result = crc32.crc32(&buf);
    assert_eq!(result,0xAE26082);
}

*/

pub(crate) fn make_metadata(header: &super::header::PngHeader) -> HashMap<String, DataMap> {
    let mut map: HashMap<String, DataMap> = HashMap::new();
    map.insert("Format".to_string(), DataMap::Ascii("PNG".to_string()));
    map.insert("width".to_string(), DataMap::UInt(header.width as u64));
    map.insert("height".to_string(), DataMap::UInt(header.height as u64));
    if let Some(gamma) = header.gamma {
        let gamma = gamma as f64 / 100000.0;
        map.insert("gamma".to_string(), DataMap::Float(gamma));
    }
    if let Some(modified_time) = &header.modified_time {
        map.insert(
            "gamma".to_string(),
            DataMap::Ascii(modified_time.to_string()),
        );
    }
    if let Some(srgb) = &header.srgb {
        map.insert("sRGB".to_string(), DataMap::UInt(*srgb as u64));
    }
    for (key, val) in &header.text {
        map.insert(key.to_string(), DataMap::Ascii(val.to_string()));
    }
    if let Some(profile) = &header.iccprofile {
        let profile_name = read_ascii_string(profile, 0, 79);
        let mut ptr = profile_name.len() + 1;
        let _ = read_byte(profile, ptr); // alway 0
        ptr += 1;
        let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(&profile[ptr..]);
        if let Ok(icc_profile) = decompressed {
            map.insert(
                "ICC Profile name".to_string(),
                DataMap::Ascii(profile_name.to_string()),
            );
            map.insert("ICC Profile".to_string(), DataMap::ICCProfile(icc_profile));
        }
    }
    /*
        pub transparency: Option<Vec<u8>>,
        pub background_color: Option<BacgroundColor>,
        pub sbit: Option<Vec<u8>>,
    */

    map
}
