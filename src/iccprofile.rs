
use bin_rs::io::*;

pub struct ICCProfile {
    pub length : u32,
    pub cmmid : u32,
    pub version :u32,
    pub device_class :u32,
    pub color_space : u32,
    pub pcs : u32,
    pub create_date: String,
    pub magicnumber_ascp: u32,
    pub platform: u32,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub rendering_intent: u32,
    pub illuminate :[u32;3],
    pub creator: u32,
    pub profile_id: u128,
    pub reserved :Vec<u8>,  // 28byte,
    pub data: Vec<u8>   // left data
}


impl ICCProfile {    
    pub fn new(self: &ICCProfile) -> Self {
        Self {
            length: self.length,
            cmmid : self.cmmid,
            version: self.version,
            device_class: self.device_class,
            color_space: self.color_space,
            pcs: self.pcs,
            create_date: self.create_date.clone(),
            magicnumber_ascp: self.magicnumber_ascp,
            platform: self.platform,
            flags: self.flags,
            manufacturer: self.manufacturer,
            model: self.model,
            attributes: self.attributes,
            rendering_intent: self.rendering_intent,
            illuminate: self.illuminate.clone(),
            creator: self.creator,
            profile_id: self.profile_id,
            reserved: Vec::new(),
            data :self.data.to_vec(),
        }
    }
}


pub fn icc_profile_header_print(header: &ICCProfile) -> String {
    let mut str = "ICC Profile\n".to_string();
    str += &format!("cmmid {}\n",String::from_utf8_lossy(&header.cmmid.to_be_bytes()));
    str += &format!("version {:08x}\n",&header.version);
    str += &format!("Device Class {}\n",String::from_utf8_lossy(&header.device_class.to_be_bytes()));
    str += &format!("Color Space {}\n",String::from_utf8_lossy(&header.color_space.to_be_bytes()));
    str += &format!("PCS {}\n",String::from_utf8_lossy(&header.pcs.to_be_bytes()));
    str += &format!("DATE {}\n",header.create_date);
    str += &format!("It MUST be 'ascp' {}\n",String::from_utf8_lossy(&header.magicnumber_ascp.to_be_bytes()));
    str += &format!("Platform {}\n",String::from_utf8_lossy(&header.platform.to_be_bytes()));
    str += &format!("flags {}\n",&header.flags);
    str += &format!("Manuacture {}\n",String::from_utf8_lossy(&header.manufacturer.to_be_bytes()));
    str += &format!("Model {:>04x}\n",&header.model);
    str += &format!("Attributes {:>064b}\n",&header.attributes);
    str += &format!("Illiuminate X:{} Y:{} Z:{}\n",&header.illuminate[0],&header.illuminate[1],&header.illuminate[2]);
    str += &format!("Creator {}\n",String::from_utf8_lossy(&header.creator.to_be_bytes()));
    str += &format!("Profile ID (MD5 {:>16x}\n",&header.profile_id);
    str += &format!("Data length {}bytes\n",&header.data.len());    
    str
}

trait IICNumber {
    fn as_f32(&self) -> f32;
    fn as_f64(&self) -> f64;
    fn int(&self) -> i32;
    fn decimal(&self) -> u32;
}

pub struct S15FixedNumber {
    integer: i16,
    decimal:u16,
}

impl IICNumber for S15FixedNumber {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

pub struct U16FixNumber {
    integer:u16,
    decimal:u16,
}

impl IICNumber for U16FixNumber {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

pub struct U1Fixed15Number {
    decimal:u16,
}

impl IICNumber for U1Fixed15Number {
    fn as_f32(&self) -> f32 { self.decimal as f32 / i16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.decimal as f64 / i16::MAX as f64 }
    fn int(&self) -> i32 { 0 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

pub struct U8Fixed8Number {
    integer:u8,
    decimal:u8,
}

impl IICNumber for U8Fixed8Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u8::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u8::MAX as f64  }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}


pub enum Data {
    DataTimeNumber((u32,u32,u32,u32,u32,u32)),
    Float32Number(f32),
    PositionNumber(Box<[u8]>),
    S15FixedNumber(S15FixedNumber),
    U16FixNumber(U16FixNumber),
    Response16Number((u16,u16,S15FixedNumber)),
    U1Fixed15Number(U1Fixed15Number),
    U8Fixed8Number(U8Fixed8Number),
    UInt16Number(u16),
    UInt32Number(u32),
    UInt64Number(u64),
    UInt8Number(u8),
    XYZNumber((S15FixedNumber,S15FixedNumber,S15FixedNumber)),
    ASCII(String),
    None,
}

impl Data {
    pub fn get(data_type:String,_data: &[u8],_length: usize) -> Data {
        match data_type  {
            _ => {

            }
        }
        Data::None
    }
}


pub fn icc_profile_print(icc_profile :&ICCProfile) -> String {
    let mut str = icc_profile_header_print(&icc_profile);
    let header_size = 128;
    let mut ptr = 0;
    str += "ICC Profiles defined data\n";
    let tags = read_u32_be(&icc_profile.data,ptr);
    ptr +=  4;
    str += &format!("Tags {}\n",tags);
    for _ in 0..tags {
        let tag_name = read_string(&icc_profile.data,ptr,4);
        ptr +=  4;
        let tag_offset = read_u32_be(&icc_profile.data,ptr) as usize;
        ptr +=  4;
        let tag_length = read_u32_be(&icc_profile.data,ptr) as usize;
        ptr +=  4;
        str +=  &format!("Tag name {} {}bytes\n",tag_name,tag_length);
        match &*tag_name {
            "A2B0" | "A2B1" | "A2B2" => {
                let ptr = tag_offset - header_size;
                let data_type = read_string(&icc_profile.data, ptr as usize, 4);
                str += &(data_type + "\n");
                let data_type = read_string(&icc_profile.data, ptr as usize, 4);
                let ich = read_byte(&icc_profile.data, ptr+8) as usize;
                let och = read_byte(&icc_profile.data, ptr+9) as usize;
                let clut_point = read_byte(&icc_profile.data, ptr+10) as u32;
                let reserve = read_byte(&icc_profile.data, ptr+11) as u32;
                str +=  &format!("Input #{} Output #{} CLUT{} //{}\n",ich,och,clut_point,reserve);
                let mut p = 12;
                let e00 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e01 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e02 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e10 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e11 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e12 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e20 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e21 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };
                p += 4;
                let e22 = S15FixedNumber {
                    integer: read_i16_be(&icc_profile.data, ptr+p),
                    decimal: read_u16_be(&icc_profile.data, ptr+p+2)
                };

                str += "Transration\n";
                str += &format!("|{} {} {}|\n"
                    ,e00.as_f32(),e01.as_f32(),e02.as_f32());
                str += &format!("|{} {} {}|\n"
                    ,e10.as_f32(),e11.as_f32(),e12.as_f32());
                str += &format!("|{} {} {}|\n"
                    ,e20.as_f32(),e21.as_f32(),e22.as_f32());

                if data_type == "mft1" {
                    p = 48;
                    let length = tag_length - p;
                    let _input_channel_size = 256 * ich * 1;
                    let _clut_size = clut_point.pow(ich as u32);
                    // mft2
                    // ch4 YCMK (clut_size) ** color_d (YCMK,YCcK =4,RGB,YUV=3)
                    // input ch [255] -> Y [0..255] C [0..255] M [0..255] K[0..255]
                    // YCMK (0,0,0,0)                                 YUV[u8][u8][u8]
                    //
                    // YCMK (clut_size,clut_size,clut_size,clut_size) YUV[u8][u8][u8]
                    // output ch [255] -> Y [0..255] U [0..255] V [0..255]
                    for i in 0..length {
                        let _data = read_byte(&icc_profile.data, ptr+ p + i);
                        if i % 32 == 31 {
                        }
                    }
                    str +=  &format!("\n\n");

                } else if data_type == "mft2" {
                    let input = read_u16_be(&icc_profile.data, ptr+48) as usize;
                    let output = read_u16_be(&icc_profile.data, ptr+50) as usize;
                    str +=  &format!("Input #{} Output #{}\n",input,output);
                    p = 52;
                    let input_channel_size = input * ich * 2;
                    let _output_channel_size = output * och * 2;
                    let _start_point = p + input_channel_size;
                    let _clut_size = clut_point.pow(ich as u32) * 2;
                    // mft2
                    // ch4 YCMK (clut_size+1) ** color_d (YCMK,YCcK =4,RGB,YUV=3)
                    // input ch [#] -> Y [0..#] C [0..#] M [0..#] K[0..#]
                    // YCMK (0,0,0,0)                                 YUV[u16][u16][u16]
                    //
                    // YCMK (clut_size,clut_size,clut_size,clut_size) YUV[u16][u16][u16]
                    // output ch [#] -> Y [0..#] U [0..#] V [0..#]

                    let _length = tag_length - p;
                    for i in 0..input*output*256 {
                        let _data = read_byte(&icc_profile.data, ptr+ p + i);
                    }
                }
            },


            "bXYZ" => {

            },
            "bTRC" => { // bTRC

            },
            "desc" => {
                let mut ptr = tag_offset - header_size;
//                let data_type = read_string(&icc_profile.data, ptr as usize, 4);    //desc
                ptr +=  4;
//                str += &(data_type + "\n");
                ptr +=  4;
                let length = read_u32_be(&icc_profile.data,ptr) as usize;
                ptr +=  4;  // padding
//                str += &format!("length {}\n",length);        
                let desc = read_string(&icc_profile.data, ptr as usize, length);
                str += &format!("{}\n",desc);

            },
            "cprt" => {
                let mut ptr = tag_offset - header_size;
//                let data_type = read_string(&icc_profile.data, ptr as usize, 4);    //text
                ptr +=  4;
//                str += &(data_type + "\n");
                ptr +=  4;  // padding
                let text= read_string(&icc_profile.data, ptr as usize, tag_length);
                str += &format!("{}\n",text);
            },
            "wtpt" => {
                let mut ptr = tag_offset - header_size;
                let data_type = read_string(&icc_profile.data, ptr as usize, 4);
                ptr +=  4;
                str += &(data_type + "\n"); // XYZ only
                ptr +=  4;
//                let length = read_u32_be(&icc_profile.data,ptr) as usize;    // only 0
                ptr +=  4;  // padding
                // XYZNumber[n] 4*3*n//
                str += &format!("data length={}\n",(tag_length - 8));
                for _ in 0..(tag_length - 8)/12 {
                    let x = S15FixedNumber {
                        integer: read_i16_be(&icc_profile.data, ptr),
                        decimal: read_u16_be(&icc_profile.data, ptr+2)
                    };
                    let y = S15FixedNumber {
                        integer: read_i16_be(&icc_profile.data, ptr+4),
                        decimal: read_u16_be(&icc_profile.data, ptr+6)
                    };
                    let z = S15FixedNumber {
                        integer: read_i16_be(&icc_profile.data, ptr+8),
                        decimal: read_u16_be(&icc_profile.data, ptr+10)
                    };
                    str += &format!("X:{}({}.{}) Y:{}({}.{}) Z:{}({}.{})\n"
                                ,x.as_f32(),x.int(),x.decimal()
                                ,y.as_f32(),y.int(),y.decimal()
                                ,z.as_f32(),z.int(),z.decimal());
                    ptr += 12;
                }
            },
            _ => {

            },

        }
    }
    str
}
