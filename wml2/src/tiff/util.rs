/*
 * tiff/util.rs  Mith@mmk (C) 2022
 * use MIT License
 */

use crate::tiff::header::DataPack;
use crate::tiff::tags::gps_mapper;
use crate::tiff::tags::tag_mapper;
use crate::tiff::header::TiffHeaders;
use bin_rs::Endian;

pub fn print_tags(header: &TiffHeaders) -> String {
    let endian = match header.endian {
        Endian::BigEndian => {"Big Endian"},
        Endian::LittleEndian => {"Little Endian"}
    };

    let mut s :String = format!("TIFF Ver{} {}\n",header.version,endian);

    for tag in &header.headers {
        let (tag_name,string) = tag_mapper(tag.tagid as u16,&tag.data,tag.length);
        s += &(tag_name + " : " + &string + "\n");
    }
    match &header.exif {
        Some(exif) => {
            s += "\nIFD Exif \n";
            for tag in exif {
                let (tag_name,string) = tag_mapper(tag.tagid as u16,&tag.data,tag.length);
                s += &(tag_name + " : " + &string + "\n");
            }
        },
        _ => {},
    }
    match &header.gps {
        Some(gps) => {
            s += "\nIFD GPS \n";
            for tag in gps {
                let (tag_name,string) = gps_mapper(tag.tagid as u16,&tag.data,tag.length);
                s += &(tag_name + " : " + &string + "\n");
            }
        },
        _ => {},
    }
    
    s
}


pub fn print_data (data: &DataPack,length:usize) -> String{
    let mut s = "".to_string();
    
    match data {
        DataPack::Rational(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };

            for i in 0..length {
                s += &format!("{}/{} ",d[i].n,d[i].d);
            }
        },
        DataPack::RationalU64(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}/{} ",d[i].n,d[i].d);
            }
        },
        DataPack::Bytes(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{} ",d[i]);
            }
        },
        DataPack::SByte(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Undef(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{:02x} ",d[i]);
            }

        },
        DataPack::Ascii(ss) => {
            s = format!("{}",ss);

        },
        DataPack::Short(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{} ",d[i]);
            }

        },
        DataPack::Long(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::SShort(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::SLong(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Float(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Double(d) => {
            if length > 1 { s = format!("\nlength {}\n",length) };
            for i in 0..length {
                s += &format!("{}",d[i]);
            }

        },
        _ => {

        },
    }
    s
}