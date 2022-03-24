/*
 * jpeg/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */
use bin_rs::reader::BinaryReader;
use crate::io::{read_byte, read_string, read_u128be, read_u16be, read_u32be, read_u64be};
type Error = Box<dyn std::error::Error>;
use crate::error::ImgError;
use crate::error::ImgErrorKind;

/* from SOS */
pub struct HuffmanScanHeader {
    pub ns: usize,
    pub csn: Vec<usize>,
    pub tdcn: Vec<usize>,
    pub tacn: Vec<usize>,
    pub ss: usize,
    pub se: usize,
    pub ah: usize,
    pub al: usize,
}

impl HuffmanScanHeader {
    pub fn new(ns: usize,csn: Vec<usize>,tdcn: Vec<usize>,tacn: Vec<usize>,ss:usize, se:usize,ah: usize,al :usize) -> Self{
        Self {
            ns,
            csn,
            tdcn,
            tacn,
            ss,
            se,
            ah,
            al,
        }
    }
}


/* from DHT */
pub struct HuffmanTable {
    pub ac: bool,
    pub no: usize,
    pub len: Vec<usize>,
    pub pos: Vec<usize>,
    pub val: Vec<usize>,
}

impl HuffmanTable {
    pub fn new(ac:bool,no:usize,len: Vec<usize>,pos: Vec<usize>,val: Vec<usize>) -> Self {
        Self {
            ac,
            no,
            len,
            pos,
            val,
        }
    }
}

/* from DQT */
pub struct QuantizationTable {
    pub presision: usize,
    pub no: usize,
    pub q: Vec<usize>,
}

impl QuantizationTable {
    pub fn new(presision:usize,no: usize,q: Vec<usize>) -> Self {
        Self {
            presision,
            no,
            q,
        }
    }

}
/* SOF */
pub struct Component{
    pub c: usize,
    pub h: usize,
    pub v: usize,
    pub tq: usize
}

pub struct FrameHeader {
    pub is_baseline: bool,
    pub is_sequential: bool,
    pub is_progressive: bool,
    pub is_lossress: bool,
    pub is_differential: bool,
    pub is_huffman: bool,
    pub width: usize,
    pub height: usize,
    pub bitperpixel: usize,
    pub plane: usize,
    pub component: Option<Vec<Component>>,
}

impl FrameHeader {
    #[warn(unused_assignments)]
    pub fn new(num: usize,buffer: &[u8]) -> Self {
        let mut is_baseline: bool = false;
        let mut is_sequential: bool = false;
        let mut is_progressive: bool = false;
        let mut is_lossress: bool = false;
        let mut is_differential: bool = false;
        let is_huffman;
        let width: usize;
        let height: usize;
        let bitperpixel: usize;
        let plane: usize;
        let mut component: Vec<Component>;

        if num & 0x03 == 0x00 {
            is_baseline = true;
        }
        if num & 0x03 == 0x01 {
            is_sequential = true;
        }
        if num & 0x03 == 0x02 
        {
            is_progressive = true;
        }
        if num & 0x03 == 0x03 {
            is_lossress = true;
        }
        if num & 0x08 == 0x00 {
            is_huffman = true;
        } else {
            is_huffman = false;
        }
        if num & 0x04 == 0x00 {
            is_differential = false;
        }
        if num & 0x04 == 0x04 {
            is_differential = true;
        }

        let p = read_byte(&buffer,0) as i32;
        bitperpixel = p as usize;
        height = read_u16be(&buffer,1) as usize;
        width = read_u16be(&buffer,3) as usize;
        let nf = read_byte(&buffer,5) as i32;
        plane = nf as usize;

        

        let mut ptr = 6;

        component = Vec::new();

        for _ in 0..nf {
            let c = read_byte(&buffer,ptr) as usize;
            let h = (read_byte(&buffer,ptr + 1) >> 4) as usize;
            let v = (read_byte(&buffer,ptr + 1) & 0x07) as usize;
            let tq = read_byte(&buffer,ptr + 2) as usize;
            ptr = ptr + 3;
//            log(format!("No{} {}x{} Table{}", c,h,v,tq);
            component.push(Component{c,h,v,tq});
        }
 
        Self {
            is_baseline,
            is_sequential,
            is_progressive,
            is_lossress,
            is_differential,
            is_huffman,
            width,
            height,
            bitperpixel,
            plane,
            component: Some(component), 
        }
    }
}


/* APP0 */
pub struct Jfif {
    pub version: u16,
    pub resolusion_unit: usize,
    pub x_resolusion: usize,
    pub y_resolusion: usize,
    pub width: usize,
    pub height: usize,
    pub thumnail: Option<Vec<u8>>,  // (width*height*3)  + tag
}

#[allow(unused)]
pub struct Jfxx {
    pub id : String,// +2   // JFXX\0
    pub ver: usize, // +7
    pub t: usize,   // +9   
    pub width: usize,   //+10 if t == 11 or 12
    pub height: usize,  //+11 if t == 11 or 12
    pub palette: Option<Vec<(u8,u8,u8)>>, // if t ==11
    pub thumnail: Option<Vec<u8>>,  // +16 - (xt*yt*3)
}

#[allow(unused)]
pub struct AdobeApp14 {
    pub dct_encode_version: usize,
    pub flag1: usize,
    pub flag2: usize,
    pub color_transform: usize,
}

pub type Exif = crate::tiff::header::TiffHeaders;

#[allow(unused)]
pub struct Ducky {
    pub quality: usize,
    pub comment: String,
    pub copyright: String,
}

pub enum ICCProfileData {
    Header(crate::iccprofile::ICCProfile),
    Data(Vec<u8>),
}

pub struct ICCProfilePacker {
    pub number : usize,
    pub total: usize,
    pub data: ICCProfileData,
}




#[allow(unused)]
pub struct UnknownApp {
    pub number : usize,
    pub tag : String,
    pub length : usize,
}


pub struct JpegHaeder {
    pub width : usize,
    pub height: usize,
    pub bpp: usize,
    pub frame_header:Option<FrameHeader>,
    pub huffman_tables:Option<Vec<HuffmanTable>>,
    pub huffman_scan_header:Option<HuffmanScanHeader>,
    pub quantization_tables:Option<Vec<QuantizationTable>>,
    pub line: usize,
    pub interval :usize,
    pub comment: Option<String>,
    pub jpeg_app_headers: Option<Vec<JpegAppHeaders>>,
    pub is_hierachical: bool,
    pub adobe_color_transform: usize,
}

#[allow(unused)]
pub enum JpegAppHeaders {
    Jfif(Jfif),
    Exif(Exif),
    Ducky(Ducky),
    Adobe(AdobeApp14),
    ICCProfile(ICCProfilePacker),
    Unknown(UnknownApp),
}

fn read_app(num: usize,tag :&String,buffer :&[u8]) -> Result<JpegAppHeaders,Error> {
    let mut ptr = tag.len() + 1;
    let mut len = buffer.len();
    match num {
        0 => {
            match tag.as_str() {
                "JFIF" => {
                    ptr = 5;
                    let version = read_u16be(&buffer,ptr) as u16;
                    let unit = read_byte(&buffer,ptr + 2) as usize;
                    let xr = read_u16be(&buffer,ptr + 3) as usize;
                    let yr = read_u16be(&buffer,ptr + 5) as usize;
                    let width = read_byte(&buffer,ptr + 7) as usize;
                    let height = read_byte(&buffer,ptr + 8) as usize;


                    let jfif :Jfif  = Jfif{
                        version: version,
                        resolusion_unit: unit,
                        x_resolusion: xr,
                        y_resolusion: yr,
                        width: width,
                        height: height,
                        thumnail: None,  // (width*height*3)  + tag
                    };

                    return Ok(JpegAppHeaders::Jfif(jfif))
                },
                _ => {

                }
            }
        },
        1 => {
            match tag.as_str() {
                "Exif" => {
                    let buf = &buffer[6..];
                    let exif = super::super::tiff::header::read_tags(buf)?;
                    return Ok(JpegAppHeaders::Exif(exif))
                },
                _ => {
                }
            }
        },
        2 => {
            match tag.as_str() {
                "ICC_PROFILE" => {
                    let number = read_byte(&buffer, ptr) as usize;
                    ptr = ptr + 1;
                    let total = read_byte(&buffer, ptr) as usize;
                    ptr = ptr + 1;
                    if number != 1 {
                        let data = buffer[ptr..].to_vec();
                        let icc_profile = ICCProfilePacker{
                            number: number,
                            total: total,
                            data: ICCProfileData::Data(data),
                        };

                        return Ok(JpegAppHeaders::ICCProfile(icc_profile))
                    };
                    let length = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let cmmid = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let version = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let device_class = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let color_space = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let pcs = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let year = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let month = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let day = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let hour = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let minute = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let second = read_u16be(&buffer,ptr);
                    ptr = ptr + 2;
                    let magicnumber_ascp = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let platform = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let flags = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let manufacturer = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let model = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let attributes = read_u64be(&buffer,ptr);
                    ptr = ptr + 8;
                    let rendering_intent = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let mut illuminate = [0_u32;3];
                    illuminate[0] = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    illuminate[1] = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    illuminate[2] = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let creator = read_u32be(&buffer,ptr);
                    ptr = ptr + 4;
                    let profile_id = read_u128be(&buffer, ptr);
                    ptr = ptr + 16;
                    let reserved :Vec<u8> = (0..28).map(|i| buffer[i]).collect();
                    ptr = ptr + 28;
                    let data :Vec<u8> = buffer[ptr..len].to_vec();

                    let create_date = format!("{:>4}/{:>2}/{:>2} {:>02}:{:>02}:{:>02}",
                        year,month,day,hour,minute,second);
                    let icc_profile = ICCProfilePacker{
                        number: number,
                        total: total,
                        data: ICCProfileData::Header(crate::iccprofile::ICCProfile{
                            length,
                            cmmid,
                            version,
                            device_class,
                            color_space,
                            pcs,
                            create_date,
                            magicnumber_ascp,
                            platform,
                            flags,
                            manufacturer,
                            model,
                            attributes,
                            rendering_intent,
                            illuminate,
                            creator,
                            profile_id,
                            reserved,
                            data,
                        })};
                    return Ok(JpegAppHeaders::ICCProfile(icc_profile))
                },
                _ => {

                }
            }
        },
        12 => {
            match tag.as_str() {
                "Ducky" => {
                    let q = read_u32be(&buffer,ptr) as usize;
                    ptr = ptr + 4;
                    len = len - 4;
                    let comment = read_string(&buffer,ptr,len);
                    ptr = ptr + comment.len() + 1;
                    len = len - comment.len() + 1;
                    let copyright = read_string(&buffer,ptr,len);
                    return Ok(JpegAppHeaders::Ducky(Ducky{quality: q,comment: comment,copyright: copyright}));
                },
                _ => {
                },
            }
        },
        14 => {
            match tag.as_str() {
                "Adobe" => {
                    let ver = read_byte(&buffer, ptr) as usize;
                    let flag1 = read_byte(&buffer, ptr + 1) as usize;
                    let flag2 = read_byte(&buffer, ptr + 2) as usize;
                    let ct = read_byte(&buffer, ptr + 3) as usize;
                        return Ok(JpegAppHeaders::Adobe(AdobeApp14{dct_encode_version: ver,flag1 :flag1,flag2: flag2,color_transform: ct}));
                },
                _ => {
                }
            }
        },
        _ => {
        }
    }
    Ok(JpegAppHeaders::Unknown(UnknownApp{number:num ,tag: tag.to_string(),length: len}))
}

impl JpegHaeder {
    pub fn new<B: BinaryReader>(reader:&mut B,opt :usize) -> Result<Self,Error> {
        let mut offset = 0;

        while offset < 16 { //SOI check
            let soi = reader.read_u16_be()?;
            if soi == 0xffd8 {break};
            offset = offset + 1;
        }

        if offset >= 16 {
            return Err(Box::new(
                ImgError::new_const(
                    ImgErrorKind::NoSupportFormat,
                    "Not Jpeg".to_string()
                )))
        }

        return Self::read_makers(reader,opt,true,false)
    }

    /* 
     * is_only_tables = only allow DQT,DHT,DAC,DRI,COM,APPn
     */

    pub(crate) fn read_makers<B: BinaryReader>(reader:&mut B,opt :usize,include_soi:bool,is_only_tables:bool) ->  Result<Self,Error> {
        let _flag = opt;
        let mut _flag = false;
        let mut _dqt_flag = false;
        let mut _dht_flag = false;
        let mut _sof_flag = is_only_tables;
        let mut _sos_flag = false;
        let mut is_hierachical = false;
        let mut width : usize = 0;
        let mut height: usize = 0;
        let mut bpp: usize = 0;
        let mut _huffman_tables:Vec<HuffmanTable> = Vec::new();
        let huffman_tables:Option<Vec<HuffmanTable>>;
        let huffman_scan_header:Option<HuffmanScanHeader>;
        let mut quantization_tables:Vec<QuantizationTable> = Vec::new();
        let mut line: usize = 0;
        let mut interval :usize = 0;
        let mut frame_header:Option<FrameHeader> = None;
        let mut comment: Option<String> = None;
        let mut _jpeg_app_headers: Vec<JpegAppHeaders> = Vec::new();
        let jpeg_app_headers: Option<Vec<JpegAppHeaders>>;
        let mut adobe_color_transform = 0;
        let mut offset = 0;

        'header:  loop {
            let byte = reader.read_byte()?;  // read byte
            if byte == 0xff { // header head
                let nextbyte :u8 = reader.read_byte()?;
                offset = offset + 2;
                if cfg!(debug_assertions) {
                    println!("{:02x}{:02x}",byte,nextbyte);
                }
                match nextbyte {
                    0xc4 => { // DHT maker
                        _dht_flag = true;
                        let length = reader.read_u16_be()? as usize;

                        let mut size :usize = 2;
                        while size < length {
                            let t = reader.read_byte()?;
                            let tc = t >> 4;
                            let th = t & 0x0f;

                            let ac = if tc == 0 { false } else { true };
                            let no = th as usize;
                            size = size + 1;
                            let mut len :Vec<usize> = Vec::with_capacity(16);
                            let mut p :Vec<usize> = Vec::with_capacity(16);
                            let mut val :Vec<usize> = Vec::new();
                            let mut vlen = 0;
                            for _ in 0..16 {
                                let l = reader.read_byte()? as usize;
                                vlen = vlen + l;
                                len.push(l);
                            }
                            let mut pss :usize = 0;
                            for i in 0..16 {
                                for _ in 0..len[i] {
                                    val.push(reader.read_byte()? as usize);
                                }
                                p.push(pss);
                                pss += len[i]; 
                            }
                            size = size + 16;

                            _huffman_tables.push(HuffmanTable::new(ac,no,len,p,val));
                            size = size + vlen;
                        }

                        //  offset = offset + length; // skip
                    },
                    0xcc => {   //DAC no impl
                        let length = reader.read_u16_be()? as usize;
                        reader.skip_ptr(length-2)?;
                        // offset = offset + length; // skip
                    },
                    0xc0..=0xcf => {  // SOF Frame Headers;
                        if !_sof_flag  {
                            _sof_flag = true;
                            let num = (nextbyte & 0x0f) as usize;
                            let length = reader.read_u16_be()? as usize;
                            let buf = reader.read_bytes_as_vec(length - 2)?;
                            let fh = FrameHeader::new(num,&buf);
                            width = fh.width;
                            height = fh.height;
                            bpp = fh.bitperpixel * fh.plane;
                            frame_header = Some(fh);
                            offset = offset + length; //skip
                        } else {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"SOF Header Multiple".to_string())))
                        }
                    },
                    0xd8 => { // Start of Image
                        if include_soi {
                            _flag = true;
                        } else {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"SOI Header Mutiple".to_string())))
                        }
                    },
                    0xd9=> { // End of Image
                        return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError ,"Unexpect EOI".to_string())))
                    },
                    0xda=> { // SOS Scan header
                        _sos_flag = true;
                        let _length = reader.read_u16_be()? as usize;
                        let ns = reader.read_byte()? as usize;
                        let mut csn: Vec<usize> = Vec::with_capacity(ns);
                        let mut tdn: Vec<usize> = Vec::with_capacity(ns);
                        let mut tan: Vec<usize> = Vec::with_capacity(ns);
                        for _i in 0..ns {
                            csn.push(reader.read_byte()? as usize);
                            let t = reader.read_byte()?;
                            tdn.push((t >> 4) as usize);
                            tan.push((t & 0xf ) as usize);
                        }
                        // progressive
                        let ss = reader.read_byte()? as usize;
                        let se = reader.read_byte()? as usize;
                        let a = reader.read_byte()?;
                        let ah = (a >> 4) as usize;
                        let al = (a & 0xf) as usize;
                        huffman_scan_header = Some(HuffmanScanHeader::new(ns,csn,tdn,tan,ss,se,ah,al));

                        break 'header;
                    },
                    0xdb =>{ // Define Quantization Table
                        _dqt_flag = true;
                        let length: usize = reader.read_u16_be()? as usize;
                        // read_dqt;
                        let mut pos :usize = 2;
                        while pos < length {
                            let mut quantizations :Vec<usize> = Vec::with_capacity(64);
                            let presision :usize;
                            let b = reader.read_byte()?;
                            let p = b >> 4;
                            let no = (b & 0x0f) as usize;
                            pos = pos + 1;
                            if p == 0 {
                                presision = 8;
                                for _ in 0..64 {
                                    quantizations.push(reader.read_byte()? as usize);
                                    pos = pos + 1;
                                }
                            } else {
                                presision = 16;
                                for _ in 0..64 {
                                    quantizations.push(reader.read_u16_be()? as usize);
                                    pos = pos + 2;
                                }
                            }
                            quantization_tables.push(QuantizationTable::new(presision,no,quantizations));
                        }
                        // offset = offset + length; // skip
                    },
                    0xdc =>{ // DNL Define Number Lines
                        _dqt_flag = true;
                        if is_only_tables {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Disallow DNL Header".to_string())))
                        }
                        let _length: usize = reader.read_u16_be()?  as usize;
                        let nl = reader.read_u16_be()? as usize;
                        line = nl;
                        // read_dqt;
                        // offset = offset + length; // skip
                    },
                    0xdd => { // Define Restart Interval
                        let _length = reader.read_u16_be()? as usize;
                        let ri = reader.read_u16_be()?;
                        interval = ri as usize;
                        // offset = offset + length; // skip
                    },
                    0xde => {   // DHP Hierachical mode
                        if is_only_tables {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Disallow DNP Header".to_string())))
                        }
                        let length = reader.read_u16_be()? as usize;
                        is_hierachical = true;
                        reader.skip_ptr(length - 2)?;
//                        offset = offset + length; // skip
                    },
                    0xdf => {   //EXP
                        if is_only_tables {
                            return Err(Box::new(ImgError::new_const(ImgErrorKind::DecodeError,"Disallow EXP Header".to_string())))
                        }
                        let length = reader.read_u16_be()? as usize;
                        reader.skip_ptr(length - 2)?;
//                        offset = offset + length; // skip
                    },
                    0xfe => { // Comment
                        let length = reader.read_u16_be()? as usize;
                        comment = Some(reader.read_ascii_string(length- 2)?);
                        offset = offset + length; // skip
                    },
                    0xe0..=0xef => { // Applications 
                        let num = (nextbyte & 0xf) as usize;
                        let length = reader.read_u16_be()? as usize;
                        let buffer = reader.read_bytes_as_vec(length- 2)?;
                        let tag = read_string(&buffer ,0, length-2);
                        let len = buffer.len() - tag.len() -1;
                        let ptr = tag.len();
                        if cfg!(debug_assertions) {
                            println!("App {} {} {} {} {}",num,length,tag,len,ptr);
                        }
                        let result = read_app(num , &tag, &buffer)?;
                        match &result {
                            JpegAppHeaders::Adobe(ref app) => {
                                adobe_color_transform = app.color_transform;
                            }, 
                            _ => {},
                        }
                        _jpeg_app_headers.push(result);
                        offset = offset + length; // skip
                    },
                    0xff => { // padding
                        // offset = offset + 1;
                    },
                    0x00 => { //data
                        // skip
                    },
                    0xd0..=0xd7 => {   // REST0-7
                    },
                    _ => {
                        let length = reader.read_u16_be()? as usize;
                        reader.skip_ptr(length-2)?;
//                        offset = offset + length;
                    }
                }
            } else {
                return Err(Box::new(ImgError::new_const(ImgErrorKind::UnknownFormat,"Not Jpeg".to_string())));
            }

        }

        if _sof_flag && _sos_flag && _dht_flag && _dqt_flag == false {
            return Err(Box::new(ImgError::new_const(ImgErrorKind::IlligalData,"Maker is shortage".to_string())));
        }

        if _jpeg_app_headers.len() > 0 {
            jpeg_app_headers = Some(_jpeg_app_headers);
        } else {
            jpeg_app_headers = None;
        }

        if _huffman_tables.len() > 0 {
            huffman_tables = Some(_huffman_tables);
        } else {
            huffman_tables = None;
        }



        Ok(Self {
            width,
            height,
            bpp,
            frame_header,
            huffman_scan_header,
            huffman_tables,
            quantization_tables: Some(quantization_tables),
            line,
            interval,
            comment,
            jpeg_app_headers,
            is_hierachical,
            adobe_color_transform,
        })
    }
}