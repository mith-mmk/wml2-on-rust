/*
 * jpeg/util.rs  Mith@mmk (C) 2022
 * use MIT License
 */

//type Error = Box<dyn std::error::Error>;
use crate::iccprofile::{icc_profile_header_print, icc_profile_print, ICCProfile};
use super::header::JpegAppHeaders::{Adobe, Ducky, Exif,  Jfif, Unknown};
use super::header::JpegAppHeaders::ICCProfile as JpegICCProfile;
use super::header::JpegHaeder;
use super::header::ICCProfileData;


#[allow(unused)]
pub(crate) static ZIG_ZAG_SEQUENCE:[usize;64] = [
      0,  1,  5,  6, 14, 15, 27, 28 ,
      2,  4,  7, 13, 16, 26, 29, 42 ,
      3,  8, 12, 17, 25, 30, 41, 43 ,
      9, 11, 18, 24, 31, 40, 44, 53 ,
     10, 19, 23, 32, 39, 45, 52, 54 ,
     20, 22, 33, 38, 46, 51, 55, 60 ,
     21, 34, 37, 47, 50, 56, 59, 61 ,
     35, 36, 48, 49, 57, 58, 62, 63 ,
  ];

  /*
pub(crate) static UN_ZIG_ZAG_SEQUENCE:[usize;64] = [
      0,  1,  8, 16,  9,  2,  3, 10,
      17, 24, 32, 25, 18, 11,  4,  5,
      12, 19, 26, 33, 40, 48, 41, 34,
      27, 20, 13,  6,  7, 14, 21, 28,
      35, 42, 49, 56, 57, 50, 43, 36,
      29, 22, 15, 23, 30, 37, 44, 51,
      58, 59, 52, 45, 38, 31, 39, 46,
      53, 60, 61, 54, 47, 55, 62, 63,
  ];
*/

/*
    option
      0x01 = minimam
    & 0x02 = Huffman Table
    & 0x04 = Huffman Table Extract
    & 0x08 = Quantitation table
    & 0x10 = Exif

  */

pub fn print_header(header: &JpegHaeder,option: usize) -> Box<String> {
    let mut str :String = "JPEG\n".to_string();
    match &header.frame_header {
        Some(fh) =>  {
            str = str + &format!(
            "SOF\nBaseline {} Progressiv {} Huffman {} Diffelensial {} Sequensial {} Lossless {} \n",
            fh.is_baseline,fh.is_progressive,fh.is_huffman,fh.is_differential,fh.is_sequential,fh.is_lossress);

            str = str + &format!(
            "Width {} Height {} {} x {}bit\n",
            header.width,header.height,fh.plane,fh.bitperpixel);
            match &fh.component {
                Some (component) =>{
                    str = str + &format!(
                        "Nf {}\n",
                        component.len());
                    for c in component.iter() {
                        str = str + &format!(
                            "ID {} h{} x v{} Quatize Table #{}\n",
                            c.c,c.h,c.v,c.tq);
                    }        
                },
                _ => {
        
                },
            }        
        },
        _ => {
            str = str + "SOF is nothing!";
        },
    }

    match &header.huffman_scan_header {
        Some(sos) => {
            str = str + &format!("\nSOS\n {} ",sos.ns);
            for i in 0..sos.ns {
                str = str + &format!("Cs{} Td{} Ta{} ",sos.csn[i],sos.tdcn[i],sos.tacn[i]);
            }
            str = str + &format!("Ss {} Se {} Ah {} Al {}\n",sos.ss,sos.se,sos.ah,sos.al);
        },
        _ => {
            str = str + "SOS is nothing!";
        },
    }

    match &header.comment {
        Some(comment) => {
            str = str + "\nCOM\n" +&comment + "\n";
        },
        _ => {

        },
    }
    let mut icc_profile_header :Option<ICCProfile> = None;

    let mut icc_profile_data :Vec<u8> = Vec::new();

    match &header.jpeg_app_headers {
        Some(app_headers) => {
            for app in app_headers.iter() {
                match app {
                    Jfif(jfif) => {
                        let unit = 
                        match jfif.resolusion_unit {
                            0 => {""}, 1 => {"dpi"},
                            2 => {"dpi"}, _ => {"N/A"}
                        };

                        str = str + &format!(
                            "JFIF Ver{:>03x} Resilution Unit X{}{} Y{}{} Thumnail {} {}\n",
                            jfif.version,jfif.x_resolusion,unit,jfif.y_resolusion,unit,jfif.width,jfif.height);

                    },
                    Exif(exif) => {
                        if option & 0x10 == 0x10 {  // Exif tag full display flag
                            str = str + "Exif\n";
                            str = str + &crate::tiff::util::print_tags(&exif);
                        } else {
                            str = str + &format!(
                                "Exif has{} tags\n",
                                exif.headers.len());
                        }
                    },
                    Ducky(ducky) => {
                        str = str + &format!(
                            "Ducky .. unknow format {} {} {}\n",
                            ducky.quality,ducky.comment,ducky.copyright);

                    },
                    Adobe(adobe) => {
                        str = str + &format!(
                            "Adobe App14 DCTEncodeVersion:{} Flag1:{} Flag2:{} ColorTransform {} {}\n"
                            ,adobe.dct_encode_version
                            ,adobe.flag1,adobe.flag2,adobe.color_transform,match adobe.color_transform {
                                    1 => {"YCbCr"}, 2 => {"YCCK"}, _ =>{"Unknown"}});

                    },
                    JpegICCProfile(icc_profile) => {
                        str = str + &format!(
                            "ICC Profile {} of {}\n",icc_profile.number,icc_profile.total);
                        match &icc_profile.data {
                            ICCProfileData::Header(ref header) => {
                                if option & 0x20 != 0x20 {  // ICC Profile
                                    str += &icc_profile_header_print(&header);
                                } else {
                                    icc_profile_header = Some(ICCProfile::new(header));
                                    icc_profile_data = header.data.to_vec();
                                }
                            },
                            ICCProfileData::Data(data) => {
                                str += &format!("Data length {}bytes\n",&data.len());
                                if icc_profile_data.len() > 0 {
                                    icc_profile_data.append(&mut data.to_vec());
                                }
                            },
                        }
                    }
                    Unknown(app) => {
                        str = str + &format!("App{} {} {}bytes is unknown\n",app.number,app.tag,app.length);
                    },
                }

            }
        },
        _ => {

        },
    }


    if header.interval > 0 {  //DRI
        str = str +  &format!("Restart Interval {}\n",header.interval);
    }

    if header.line > 0 { //DNL
        str = str +  &format!("Define number of lines {}\n",header.line);
    }

    if option & 0x08 == 0x08 { // Define Quatization Table Flags
        match &(header.quantization_tables) {
            Some(qts) => {
                str = str + "DQT\n";
                for qt in qts.iter()  {
                    str = str + &format!("Pq{}(bytes) Tq{}\n",qt.presision + 1,qt.no);
                    for (i,q) in qt.q.iter().enumerate() {
                        str = str +&format!("{:3}",q);
                        if i % 8 == 7 { str = str + "\n"} else { str = str + ","}
                    }
                }
            },
            _ => {
                str = str + "DQT in nothing!\n\n";
            }
    
        }
    }

    if option & 0x02 == 0x02 { // Define Huffman Table Flags
        match &header.huffman_tables {
            Some(hts) => {
                str = str + "DHT\n";
                for ht in hts.iter()  {
                    str = str + &format!("{} Table{}\n",if ht.ac == true {"AC"} else {"DC"},ht.no);
                    str = str + "L ";
                    for l in ht.len.iter() {
                        str = str + &l.to_string() + " ";
                    }
                    str = str + "\n V ";
                    for v in ht.val.iter()  {
                        str = str + &v.to_string() + " ";                    
                    }
                    str = str + "\n";
                }
            },
            _ => {
                str = str + "DQT in nothing!\n\n";
            }
    
        }
    }

    if option & 0x04 == 0x04 { // Define Huffman Table Decoded
        match &header.huffman_tables {
            Some(huffman_tables) => {
                for (i,huffman_table) in huffman_tables.iter().enumerate() {
                    let mut current_max: Vec<i32> = Vec::new();
                    let mut current_min: Vec<i32> = Vec::new();

                    if huffman_table.ac {
                        str = str + &format!("Huffman Table AC {}\n",i);
                    } else {
                        str = str + &format!("Huffman Table DC {}\n",i);
                    }

                    let mut code :i32 = 0;
                    let mut pos :usize = 0;
                    for l in 0..16 {
                        if huffman_table.len[l] != 0 {
                            current_min.push(code);
                            for _ in 0..huffman_table.len[l] {
                                if pos >= huffman_table.val.len() { break;}
                                str = str + &format!("{:>02b}  {:>02x}\n",code,huffman_table.val[pos]);
                                pos = pos + 1;
                                code = code + 1;
                            }
                            current_max.push(code - 1); 
                        } else {
                            current_min.push(-1);
                            current_max.push(-1);
                        }
                        code = code << 1;
                    }                    
                }
            },
            _  =>  {}
        }
    }

    if option & 0x20 == 0x20 {  // ICC Profile decode
        if icc_profile_header.is_some() {
            let mut icc_profile = icc_profile_header.unwrap();
            icc_profile.data = icc_profile_data;
            str += &icc_profile_print(&icc_profile);
        }
    }

            
    let strbox :Box<String> = Box::new(str);

    strbox
}