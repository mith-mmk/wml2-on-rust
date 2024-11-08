/*
 * tiff/util.rs  Mith@mmk (C) 2022
 * use MIT License
 */

const MAX_LENGTH: usize = 100;

use crate::metadata::DataMap;
use crate::tiff::header::DataPack;
use crate::tiff::header::TiffHeaders;
use crate::tiff::tags::gps_mapper;
use crate::tiff::tags::tag_mapper;
use bin_rs::Endian;

pub fn print_tags(header: &TiffHeaders) -> String {
    let endian = match header.endian {
        Endian::BigEndian => "Big Endian",
        Endian::LittleEndian => "Little Endian",
    };

    let mut s: String = format!("TIFF Ver{} {}\n", header.version, endian);

    for tag in &header.headers {
        let (tag_name, _) = tag_mapper(tag.tagid as u16, &tag.data, tag.length);
        let string = print_data(&tag.data, tag.length);
        s += &(tag_name + " : " + &string + "\n");
    }
    match &header.exif {
        Some(exif) => {
            s += "\nIFD Exif \n";
            for tag in exif {
                let (tag_name, _) = tag_mapper(tag.tagid as u16, &tag.data, tag.length);
                let string = print_data(&tag.data, tag.length);
                s += &(tag_name + " : " + &string + "\n");
            }
        }
        _ => {}
    }
    match &header.gps {
        Some(gps) => {
            s += "\nIFD GPS \n";
            for tag in gps {
                let (tag_name, _) = gps_mapper(tag.tagid as u16, &tag.data, tag.length);
                let string = print_data(&tag.data, tag.length);
                s += &(tag_name + " : " + &string + "\n");
            }
        }
        _ => {}
    }

    s
}

pub fn print_data_with_max_size(data: &DataPack, length: usize, max_size: usize) -> String {
    let mut s = "".to_string();
    let len = if length < 100 { length } else { max_size };

    match data {
        DataPack::Rational(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };

            for i in 0..length {
                s += &format!("{}/{} ", d[i].n, d[i].d);
            }
        }
        DataPack::SRational(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{}/{} ", d[i].n, d[i].d);
            }
        }
        DataPack::Bytes(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::SByte(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::Undef(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{:02x} ", d[i]);
            }
        }
        DataPack::Ascii(ss) => {
            s = ss.to_string();
        }
        DataPack::Short(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::Long(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::SShort(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::SLong(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        DataPack::Float(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{}", d[i]);
            }
        }
        DataPack::Double(d) => {
            if length > 1 {
                s = format!("\nlength {}\n", length)
            };
            for i in 0..length {
                s += &format!("{} ", d[i]);
            }
        }
        _ => {}
    }
    if length > len {
        s += "\n...";
    }
    s
}

pub fn print_data(data: &DataPack, length: usize) -> String {
    print_data_with_max_size(data, length, MAX_LENGTH)
}

/*
pub fn make_metadata(header: &TiffHeaders) -> HashMap<String,DataMap> {
    let mut map:HashMap<String,DataMap> = HashMap::new();
    let endian = match header.endian {
        Endian::BigEndian => {"Big Endian"},
        Endian::LittleEndian => {"Little Endian"}
    };
    map.insert("endian".to_string(),DataMap::Ascii(endian.to_string()));
    map.insert("Tiff version".to_string(),DataMap::UInt(header.version as u64));


    for tag in &header.headers {
        let (tag_name,data) = tag_mapper(tag.tagid as u16,&tag.data,tag.length);
        map.insert(tag_name,data);
    }
    match &header.exif {
        Some(exif) => {
            for tag in exif {
                let (tag_name,data) = tag_mapper(tag.tagid as u16,&tag.data,tag.length);
                map.insert(tag_name,data);
            }
        },
        _ => {},
    }
    match &header.gps {
        Some(gps) => {
            for tag in gps {
                let (tag_name,data) = gps_mapper(tag.tagid as u16,&tag.data,tag.length);
                map.insert(tag_name,data);
            }
        },
        _ => {},
    }

    map
}
*/

pub fn convert(data: &DataPack, length: usize) -> DataMap {
    match data {
        DataPack::Rational(d) => {
            if length == 1 {
                return DataMap::Float(d[0].n as f64 / d[0].d as f64);
            }
            let mut data = vec![];

            for val in d {
                let val = val.n as f64 / val.d as f64;
                data.push(val);
            }
            DataMap::FloatAllay(data)
        }
        DataPack::SRational(d) => {
            if length == 1 {
                return DataMap::Float(d[0].n as f64 / d[0].d as f64);
            }
            let mut data = vec![];

            for val in d {
                let val = val.n as f64 / val.d as f64;
                data.push(val);
            }
            DataMap::FloatAllay(data)
        }
        DataPack::Bytes(d) => {
            if length == 1 {
                return DataMap::UInt(d[0] as u64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as u64);
            }
            DataMap::UIntAllay(data)
        }
        DataPack::SByte(d) => {
            if length == 1 {
                return DataMap::SInt(d[0] as i64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as i64);
            }
            DataMap::SIntAllay(data)
        }
        DataPack::Undef(d) => DataMap::Raw(d.to_vec()),

        DataPack::Ascii(ss) => DataMap::Ascii(ss.to_string()),
        DataPack::Short(d) => {
            if length == 1 {
                return DataMap::UInt(d[0] as u64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as u64);
            }
            DataMap::UIntAllay(data)
        }
        DataPack::Long(d) => {
            if length == 1 {
                return DataMap::UInt(d[0] as u64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as u64);
            }
            DataMap::UIntAllay(data)
        }
        DataPack::SShort(d) => {
            if length == 1 {
                return DataMap::SInt(d[0] as i64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as i64);
            }
            DataMap::SIntAllay(data)
        }
        DataPack::SLong(d) => {
            if length == 1 {
                return DataMap::SInt(d[0] as i64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as i64);
            }
            DataMap::SIntAllay(data)
        }
        DataPack::Float(d) => {
            if length == 1 {
                return DataMap::Float(d[0] as f64);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val as f64);
            }
            DataMap::FloatAllay(data)
        }
        DataPack::Double(d) => {
            if length == 1 {
                return DataMap::Float(d[0]);
            }
            let mut data = vec![];
            for val in d {
                data.push(*val);
            }
            DataMap::FloatAllay(data)
        }
        DataPack::Unkown(d) => DataMap::Raw(d.to_vec()),
    }
}
