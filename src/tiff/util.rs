/*
 * tiff/util.rs  Mith@mmk (C) 2022
 * use MIT License
 */

use crate::tiff::header::DataPack;
use crate::tiff::tags::gps_mapper;
use crate::tiff::tags::tag_mapper;
use crate::tiff::header::TiffHeaders;

pub fn print_tags(header: &TiffHeaders) -> String {
    let mut s :String = format!("TIFF Ver{} {}\n",header.version,if header.little_endian {"Little Endian"} else {"Big Endian"});

    for tag in &header.headers {
        let (tag_name,string) = tag_mapper(tag.tagid as u16,&tag.data);
        s += &(tag_name + " : " + &string + "\n");
    }
    match &header.exif {
        Some(exif) => {
            s += "\nIFD Exif \n";
            for tag in exif {
                let (tag_name,string) = tag_mapper(tag.tagid as u16,&tag.data);
                s += &(tag_name + " : " + &string + "\n");
            }
        },
        _ => {},
    }
    match &header.gps {
        Some(gps) => {
            s += "\nIFD GPS \n";
            for tag in gps {
                let (tag_name,string) = gps_mapper(tag.tagid as u16,&tag.data);
                s += &(tag_name + " : " + &string + "\n");
            }
        },
        _ => {},
    }
    
    s
}


pub fn print_data (data: &DataPack) -> String{
    let mut s = "".to_string();
    match data {
        DataPack::Rational(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };

            for i in 0..d.len() {
                s += &format!("{}/{} ",d[i].n,d[i].d);
            }
        },
        DataPack::RationalU64(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}/{} ",d[i].n,d[i].d);
            }
        },
        DataPack::Bytes(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{} ",d[i]);
            }
        },
        DataPack::SByte(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Undef(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{:02x} ",d[i]);
            }

        },
        DataPack::Ascii(ss) => {
            s = format!("{}",ss);

        },
        DataPack::Short(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{} ",d[i]);
            }

        },
        DataPack::Long(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::SShort(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::SLong(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Float(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        DataPack::Double(d) => {
            if d.len() > 1 { s = format!("\nlength {}\n",d.len()) };
            for i in 0..d.len() {
                s += &format!("{}",d[i]);
            }

        },
        _ => {

        },
    }
    s
}