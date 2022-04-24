use std::collections::HashMap;
use bin_rs::io::*;
use crate::iccprofile::Data::*;

pub fn icc_profile_print(icc_profile :&ICCProfile) -> String {
    let mut str = icc_profile_header_print(&icc_profile);
    let header_size = 128;
    let mut ptr = header_size;
    str += "==== ICC Profiles defined data ====\n";
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
            "A2B0" | "A2B1" | "A2B2" | "B2A0" | "B2A1" | "B2A2" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("LUT Table - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n";
            },
            "chad" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("Conversion D65 to D50 - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n";
            },
            "bkpt" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("Media Black Point - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n\n";
            },

            "bXYZ" | "gXYZ" | "rXYZ" => {
                str += "rgb XYZ Tag ";
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n";
            },
            "bTRC" | "gTRC" | "rTRC"=> { // bTRC
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("Color tone reproduction curve - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n\n";
            },
            "desc" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("desc - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n\n";
            },
            "cprt" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("cprt - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n\n";
            },
            "wtpt" => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("Media white point - data_type:{}\n",data_type);
                str += &val.as_string();
                str += "\n";
            },
            _ => {
                let ptr = tag_offset;
                let (data_type,val) = Data::parse(&icc_profile.data[ptr..],tag_length);
                str += &format!("{} - data_type:{}\n",tag_name,data_type);
                str += &val.as_string();
                str += "\n";

            },

        }
    }
    str
}

pub fn icc_profile_decode(icc_profile :&ICCProfile) -> DecodedICCProfile {
    let mut decoded: HashMap<String,Data> = HashMap::new();
    let header_size = 128;
    let mut ptr = 0;
    let tags = read_u32_be(&icc_profile.data,ptr);
    ptr +=  4;
    for _ in 0..tags {
        let tag_name = read_string(&icc_profile.data,ptr,4);
        ptr +=  4;
        let tag_offset = read_u32_be(&icc_profile.data,ptr) as usize - header_size;
        ptr +=  4;
        let tag_length = read_u32_be(&icc_profile.data,ptr) as usize;
        ptr +=  4;
        let (_,val) = Data::parse(&icc_profile.data[tag_offset..],tag_length);
        decoded.insert(tag_name,val);
    }
    DecodedICCProfile {
        length : icc_profile.length, 
        cmmid : icc_profile.cmmid,
        version :icc_profile.version,
        device_class :icc_profile.device_class,
        color_space : icc_profile.color_space,
        pcs : icc_profile.pcs,
        create_date: icc_profile.create_date.clone(),
        magicnumber_ascp: icc_profile.magicnumber_ascp,
        platform: icc_profile.platform,
        flags: icc_profile.flags,
        manufacturer: icc_profile.manufacturer,
        model: icc_profile.model,
        attributes: icc_profile.attributes,
        rendering_intent: icc_profile.rendering_intent,
        illuminate :icc_profile.illuminate.clone(),
        creator: icc_profile.creator,
        profile_id: icc_profile.profile_id,
        tags: decoded,
    }
}

#[derive(Debug)]
pub struct DecodedICCProfile {
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
    pub tags: HashMap<String,Data>,
}

#[derive(Debug)]
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
    pub fn new(buffer :&Vec<u8>) -> Self {
        let mut ptr = 0;
        let length = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let cmmid = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let version = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let device_class = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let color_space = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let pcs = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let year = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let month = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let day = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let hour = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let minute = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let second = read_u16_be(&buffer,ptr);
        ptr = ptr + 2;
        let magicnumber_ascp = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let platform = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let flags = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let manufacturer = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let model = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let attributes = read_u64_be(&buffer,ptr);
        ptr = ptr + 8;
        let rendering_intent = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let mut illuminate = [0_u32;3];
        illuminate[0] = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        illuminate[1] = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        illuminate[2] = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let creator = read_u32_be(&buffer,ptr);
        ptr = ptr + 4;
        let profile_id = read_u128_be(&buffer, ptr);
        // ptr = ptr + 16;
        // reserved 28 byte skip

        let create_date = format!("{:>4}/{:>2}/{:>2} {:>02}:{:>02}:{:>02}",
            year,month,day,hour,minute,second);
        Self {
            length: length,
            cmmid : cmmid,
            version: version,
            device_class: device_class,
            color_space: color_space,
            pcs: pcs,
            create_date: create_date.clone(),
            magicnumber_ascp: magicnumber_ascp,
            platform: platform,
            flags: flags,
            manufacturer: manufacturer,
            model: model,
            attributes: attributes,
            rendering_intent: rendering_intent,
            illuminate: illuminate.clone(),
            creator: creator,
            profile_id: profile_id,
            reserved: Vec::new(),
            data : buffer.to_vec(),
        }
    }
}


pub fn icc_profile_header_print(header: &ICCProfile) -> String {
    let mut str = "=========== ICC Profile ===========\n".to_string();
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
    str += &format!("Model {:04x}\n",&header.model);
    str += &format!("Attributes {:>064b}\n",&header.attributes);
    str += &format!("Illiuminate X:{} Y:{} Z:{}\n",&header.illuminate[0],&header.illuminate[1],&header.illuminate[2]);
    str += &format!("Creator {}\n",String::from_utf8_lossy(&header.creator.to_be_bytes()));
    str += &format!("Profile ID (MD5 {:016x})\n",&header.profile_id);
    str += &format!("Data length {}bytes\n",&header.data.len());
    str
}

trait IICNumber {
    fn as_f32(&self) -> f32;
    fn as_f64(&self) -> f64;
    fn int(&self) -> i32;
    fn decimal(&self) -> u32;
}

#[derive(Debug)]
pub struct S15Fixed16Number {
    integer: i16,
    decimal:u16,
}

impl IICNumber for S15Fixed16Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct U16Fixed16Number {
    integer:u16,
    decimal:u16,
}

impl IICNumber for U16Fixed16Number {
    fn as_f32(&self) -> f32 { self.integer as f32 + self.decimal as f32 / u16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.integer as f64 + self.decimal as f64 / u16::MAX as f64 }
    fn int(&self) -> i32 { self.integer as i32 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
pub struct U1Fixed15Number {
    decimal:u16,
}

impl IICNumber for U1Fixed15Number {
    fn as_f32(&self) -> f32 { self.decimal as f32 / i16::MAX as f32 }
    fn as_f64(&self) -> f64 { self.decimal as f64 / i16::MAX as f64 }
    fn int(&self) -> i32 { 0 }
    fn decimal(&self) -> u32 { self.decimal as u32}
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct XYZNumber {
    pub x:S15Fixed16Number,
    pub y:S15Fixed16Number,
    pub z:S15Fixed16Number
}

#[derive(Debug)]
pub struct Mft1 {
    pub input_channels :u8,
    pub output_channels:u8,
    pub number_of_clut_grid_points:u8,
    pub e_params:Vec<S15Fixed16Number>,
    pub input_table: Vec<u8>,
    pub clut_values: Vec<u8>,
    pub output_table: Vec<u8>,
}

#[derive(Debug)]
pub struct Mft2 {
    pub input_channels :u8,
    pub output_channels:u8,
    pub number_of_clut_grid_points:u8,
    pub e_params:Vec<S15Fixed16Number>,
    pub input_table_enteries: u16,
    pub output_table_enteries: u16,
    pub input_table: Vec<u16>,
    pub clut_values: Vec<u16>,
    pub output_table: Vec<u16>, 
}



#[derive(Debug)]
pub enum Data {
    ASCII(String),
    DataTimeNumber(u32,u32,u32,u32,u32,u32),
    Float32Number(f32),
    PositionNumber(Box<[u8]>),
    S15Fixed16Number(S15Fixed16Number),
    S15Fixed16NumberArray(Vec<S15Fixed16Number>),
    ParametricCurve(u16,Vec<S15Fixed16Number>),
    U16Fixed16Number(U16Fixed16Number),
    Response16Number(u16,u16,S15Fixed16Number),
    U1Fixed15Number(U1Fixed15Number),
    U8Fixed8Number(U8Fixed8Number),
    UInt16Number(u16),
    UInt32Number(u32),
    UInt64Number(u64),
    UInt8Number(u8),
    XYZNumber(XYZNumber),
    XYZNumberArray(Vec<XYZNumber>),
    ChromaticityType(u16,u16,Vec<(U16Fixed16Number,U16Fixed16Number)>),
    MultiLocalizedUnicode(u32,u32,String,String,String),
    ViewCondtions(XYZNumber,XYZNumber,u32),
    Measurement(u32,XYZNumber,u32,U16Fixed16Number,u32),
    Curve(Vec<u16>),
    Lut8(Mft1),
    Lut16(Mft2),
    None,
}

impl Data {

    pub fn parse(data: &[u8],length:usize) -> (String,Data) {
        let data_type = Self::read_data_type(data,0);
        (data_type.clone(),Self::get(&data_type,data,length))
    }

    pub fn get(data_type:&str,data: &[u8],length:usize) -> Data {
        let len = length - 8;
        let mut ptr = 8;
        match data_type {
            "para" => {
                let funtion_type = read_u16_be(data,ptr);
                ptr += 4;
                let mut vals :Vec<S15Fixed16Number> = vec![];
                while  ptr < length {
                    vals.push(S15Fixed16Number{
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    });
                    ptr += 4;
                }
                ParametricCurve(funtion_type,vals)
            },
            "sig " => {
                let string = read_ascii_string(data, ptr, 4);
                ASCII(string)
            }
            "XYZ " | "XYZ" => {
                let mut xyzs :Vec<XYZNumber> = vec![];
                while  ptr < length {
                    let xyz = Self::xyz_number(data, ptr);
                    xyzs.push(xyz);
                    ptr += 12;
                }
                XYZNumberArray(xyzs)
            },
            "sf32" => { //s16Fixed16ArrayType
                let mut vals :Vec<S15Fixed16Number> = vec![];
                while  ptr < length {
                    vals.push(S15Fixed16Number{
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    });
                    ptr += 4;
                }
                S15Fixed16NumberArray(vals)
            },
            "text"=> {
                let string = read_ascii_string(data, ptr,len);
                Self::ASCII(string)
            },
            "desc" => {
                let string = read_ascii_string(data, ptr+4,len-4);
                Self::ASCII(string)
            }, 
            "chrm" => {
                let device_number = read_u16_be(data,ptr);
                let encoded_value = read_u16_be(data,ptr+2);
                ptr += 4;
                let mut vals :Vec<(U16Fixed16Number,U16Fixed16Number)> = vec![];
                while  ptr < length {
                    vals.push((
                        U16Fixed16Number{
                            integer: read_u16_be(data, ptr),
                            decimal: read_u16_be(data, ptr+2)
                        },
                        U16Fixed16Number{
                            integer: read_u16_be(data, ptr+2),
                            decimal: read_u16_be(data, ptr+4)
                        }));
                    ptr += 8;
                }
                ChromaticityType(device_number,encoded_value,vals)
            },
            "mluc" => {
                let number_of_names = read_u32_be(data,ptr);
                ptr +=4;
                let name_recode_size = read_u32_be(data,ptr);
                ptr +=4;
                let first_name_language_code = read_ascii_string(data,ptr,2);
                ptr +=2;
                let first_name_country_code = read_ascii_string(data,ptr,2);
                ptr +=2;
                let lang = first_name_language_code + "." + &first_name_country_code;
                let name_length = read_u32_be(data,ptr) as usize;
                ptr +=4;
                let name_offset = read_u32_be(data,ptr) as usize;
                ptr +=4;
                let mut len = 0;
                let mut vals = vec![];
                while len < name_length {
                    let val = read_u16_be(data, name_offset + len);
                    if val == 0 {
                        break;
                    }
                    vals.push(val);
                    len += 2;
                }
                let string = String::from_utf16_lossy(&vals);
                let mut vals = vec![];
                while ptr < name_offset {
                    let val = read_u16_be(data, ptr);
                    vals.push(val);
                    ptr += 2;
                }
                let more_string = String::from_utf16_lossy(&vals);
                MultiLocalizedUnicode(number_of_names,name_recode_size,lang,string,more_string)

            },
            "view" => {
                let xyz_ilu = Self::xyz_number(data, ptr);
                ptr += 12;
                let xyz_sur = Self::xyz_number(data, ptr);
                ptr += 12;
                let ilu_type = read_u32_be(data,ptr);
                ViewCondtions(xyz_ilu,xyz_sur,ilu_type)              
            },
            "meas" => {
                let encoded_value = read_u32_be(data, ptr);
                ptr += 4;
                let xyz = Self::xyz_number(data, ptr);
                ptr += 12;
                let measurement_geometry = read_u32_be(data, ptr);
                ptr += 4;
                let measurement_flate = U16Fixed16Number{
                    integer: read_u16_be(data, ptr),
                    decimal: read_u16_be(data, ptr+2)
                };
                ptr += 4;

                let measurement_illuminate = read_u32_be(data, ptr);
                Measurement(encoded_value,xyz,measurement_geometry,measurement_flate,measurement_illuminate)

            },
            "curv" => {
                let mut curv = vec![];
                let count = read_u32_be(data, ptr) as usize;
                ptr += 4;
                for _ in 0..count {
                    curv.push(read_u16_be(data, ptr));
                    ptr += 2;
                }
                Curve(curv)
            },
            "mft1" | "mft2"  => {         
                let input_channels= read_byte(data, ptr);
                ptr +=1;
                let output_channels = read_byte(data, ptr);
                ptr +=1;
                let number_of_clut_grid_points = read_byte(data, ptr);
                ptr +=2; // with skip padding

                let mut e_params:Vec<S15Fixed16Number> = vec![];
                // e00 e01 e02 ... e20 d21 e22
                for _ in 0..9 {
                    let e = S15Fixed16Number {
                        integer: read_i16_be(data, ptr),
                        decimal: read_u16_be(data, ptr+2)
                    };
                    e_params.push(e);
                    ptr += 4;
                }

                let clut_size = ((number_of_clut_grid_points as u32).pow(input_channels as u32) * output_channels as u32) as usize;

                if data_type == "mft1" {
                    let mut input_table = vec![];
                    let mut clut_values = vec![];
                    let mut output_table =vec![];

                    let input_channels_size = input_channels as usize * 256;
                    let output_channels_size = output_channels as usize * 256;

                    for _ in 0..input_channels_size {
                        input_table.push(read_byte(data, ptr));
                        ptr += 1;
                    }

                    for _ in  0..clut_size {
                        clut_values.push(read_byte(data, ptr));
                        ptr += 1;
                    }

                    for _ in  0..output_channels_size {
                        output_table.push(read_byte(data, ptr));
                        ptr += 1;
                    }


                    let mft = Mft1{
                        input_channels,
                        output_channels,
                        number_of_clut_grid_points,
                        e_params,
                        input_table,
                        clut_values,
                        output_table, 

                    };
                    Lut8(mft)
                } else {
                    let mut input_table = vec![];
                    let mut clut_values = vec![];
                    let mut output_table =vec![];

                    let input_table_enteries = read_u16_be(data, ptr);
                    ptr += 2;
                    let output_table_enteries = read_u16_be(data, ptr);
                    ptr += 2;

                    let input_channels_size = input_channels as usize * input_table_enteries as usize;
                    let output_channels_size = output_channels as usize * output_table_enteries as usize;

                    for _ in 0..input_channels_size {
                        input_table.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    for _ in  0..clut_size {
                        clut_values.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    for _ in  0..output_channels_size {
                        output_table.push(read_u16_be(data, ptr));
                        ptr += 2;
                    }

                    let mft = Mft2{
                        input_channels,
                        output_channels,
                        number_of_clut_grid_points,
                        e_params,
                        input_table_enteries,
                        output_table_enteries,
                        input_table,
                        clut_values,
                        output_table, 

                    };
                    Lut16(mft)
                }
            },
            _ => { // no impl
                Self::None
            }
        }

    }

    pub fn as_string(&self) -> String{
        match &*self {
            DataTimeNumber(year,month,day,hour,minutes,second) => {
                format!("{:4}-{:02}-{:02} {:02}:{:02}:{:02}",
                    year,month,day,hour,minutes,second)
            },
            Float32Number(f) => {
                f.to_string()
            },
            PositionNumber(boxed) => {
                format!("{} bytes..",boxed.len())
            },
            S15Fixed16Number(f) => {
                f.as_f32().to_string()
            },
            S15Fixed16NumberArray(vals) => {
                let mut str = "".to_string();
                for f in vals {
                    str += &f.as_f32().to_string();
                    str += " ";
                }
                str.to_string()
            },
            ParametricCurve(funtion_type,vals) => {
                let mut str = match funtion_type {
                    0x000 => {"function Y = X**ganma\n"},
                    0x001 => {"function Y = (aX+b)**ganma (X >= -b/a), Y = 0 (X < -b/a)\n"},
                    0x002 => {"function Y = (aX+b)**ganma + c(X >= -b/a), Y = c (X < -b/a)\n"},
                    0x003 => {"function Y = (aX+b)**ganma (X >= d), Y = cX (X < d)\n"},
                    0x004 => {"function Y = (aX+b)**ganma + e(X >= d), Y = cX + f (X < d)\n"},
                    _ => {"function Unknown"},
                }.to_string();
                for f in vals {
                    str += &f.as_f32().to_string();
                    str += " ";
                }
                str.to_string()
            }
            U16Fixed16Number(f) => {
                f.as_f32().to_string()
            },
            Response16Number(v1,v2,f) => {
                format!("{} {} {}",v1,v2,f.as_f32())              
            },
            U1Fixed15Number(f) => {
                f.as_f32().to_string()
            },
            U8Fixed8Number(f) => {
                f.as_f32().to_string()
            },
            UInt8Number(n) => {
                n.to_string()
            },
            UInt16Number(n) => {
                n.to_string()
            },
            UInt32Number(n) => {
                n.to_string()
            },
            UInt64Number(n) => {
                n.to_string()
            },
            XYZNumber(xyz) => {
                format!("X:{} Y:{} Z:{}\n",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32())
            },
            XYZNumberArray(xyzs) => {
                let mut str = "".to_string();
                for xyz in xyzs {
                    str += &format!("X:{} Y:{} Z:{} ",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32())
                }
                str + "\n"
            },
            ChromaticityType(device_number,encoded_value,vals) => {
                let encoded = match encoded_value {
                    0x001 => {"ITU-R BT.709"},
                    0x002 => {"SMPTE RP145-1994"},
                    0x003 => {"EBU Tech.3213-E"},
                    0x004 => {"P22"},
                    _ => {"unknown"},

                };
                let mut str = format!("Number of Device Channels {} {} ",device_number,encoded);
                for (x,y) in vals {
                    str += &format!("x:{} y:{} ",x.as_f32(),y.as_f32());
                }

                str + "\n"
            },
            Measurement(encoded_value,xyz,measurement_geometry,measurement_flate,measurement_illuminate) => {
                let mut str = match encoded_value {
                    0x00000001 => {"Standard Observer: CIE 1931 standard colorimetric observer\n"},
                    0x00000002 => {"Standard Observer: CIE 1964 standard colorimetric observer\n"},
                    _ => {"Standard: Observer unknown\n"},
                }.to_string();
                str += &format!("XYZ tristimulus values X:{} Y:{} Z:{}\n",xyz.x.as_f32(),xyz.y.as_f32(),xyz.z.as_f32(),);
                str += "Measurement geometry ";
                str += match measurement_geometry {
                    0x00000001 => {"0/45 or 45/0\n"},
                    0x00000002 => {"0/d or d/0\n"},
                    _ => {"unknown\n"},
                };
                str += &format!("Measurement flare {}\n",measurement_flate.as_f32());
                str += "Standard Illuminant: ";
                str += match measurement_illuminate {
                    0x00000001 => {"D50\n"},
                    0x00000002 => {"D65\n"},
                    0x00000003 => {"D93\n"},
                    0x00000004 => {"F2\n"},
                    0x00000005 => {"D55\n"},
                    0x00000006 => {"A\n"},
                    0x00000007 => {"Equi-Power (E)\n"},
                    0x00000008 => {"F8\n"},
                    _ => {"unknown\n"},
                };

                str + "\n"
            },

            ASCII(string) => {
                string.to_string()
            },
            None => {
                "None".to_string()
            },
            _ => {
                format!("{:?}",*self)
            }
        }

    }

    pub fn xyz_number(data: &[u8],ptr: usize) -> XYZNumber {
        let cie_x = S15Fixed16Number {
            integer: read_i16_be(&data, ptr),
            decimal: read_u16_be(&data, ptr+2)
        };
        let cie_y = S15Fixed16Number {
            integer: read_i16_be(&data, ptr+4),
            decimal: read_u16_be(&data, ptr+6)
        };
        let cie_z = S15Fixed16Number {
            integer: read_i16_be(&data, ptr+8),
            decimal: read_u16_be(&data, ptr+10)
        };
        XYZNumber{x:cie_x,y:cie_y,z:cie_z}
    }

    pub fn read_data_type(data:&[u8],ptr: usize) -> String {
        let data_type = read_string(data, ptr as usize, 4);
        if data_type.len() == 0 {
            return read_string(data, ptr as usize, 3)
        }
        data_type
    }
}

