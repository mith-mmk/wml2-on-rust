/*
 * jpeg/header.rs  Mith@mmk (C) 2022
 * use MIT License
 */

use crate::error::ImgError::Custom;
use crate::error::{ImgError,ErrorKind};
use crate::error::ImgError::{SimpleAddMessage};
use crate::io::{read_byte, read_bytes, read_string, read_u128be, read_u16be, read_u32be, read_u64be};


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
    pub imageoffset: usize,
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

fn read_app(num: usize,tag :&String,buffer :&[u8],mut ptr :usize,mut len :usize) -> Result<JpegAppHeaders,ImgError> {
    match num {
        0 => {
            match tag.as_str() {
                "JFIF" => {
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
                    ptr = ptr + 1; // start 6byte
                    len = len - 1;
                    let buf :Vec<u8> = (0..len)
                        .map(|i| {buffer[ptr + i]})
                        .collect();

                    let exif = super::super::tiff::header::read_tags(&buf)?;
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
    pub fn new(buffer :&[u8],opt :usize) -> Result<Self,ImgError> {
        let mut offset = 0;

        while offset < 16 { //SOI check
            let soi = read_u16be(buffer,offset);
            if soi == 0xffd8 {break};
            offset = offset + 1;
        }

        if offset >= 16 {
            return Err(Custom("Not Jpeg".to_string()))
        }

        return Self::read_makers(&buffer[offset..],opt,true,false)
    }

    /* 
     * is_only_tables = only allow DQT,DHT,DAC,DRI,COM,APPn
     */

    pub(crate) fn read_makers(buffer :&[u8],opt :usize,include_soi:bool,is_only_tables:bool) ->  Result<Self,ImgError> {
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
        let mut huffman_scan_header:Option<HuffmanScanHeader> = None;
        let mut quantization_tables:Vec<QuantizationTable> = Vec::new();
        let mut line: usize = 0;
        let mut interval :usize = 0;
        let mut frame_header:Option<FrameHeader> = None;
        let mut comment: Option<String> = None;
        let mut _jpeg_app_headers: Vec<JpegAppHeaders> = Vec::new();
        let jpeg_app_headers: Option<Vec<JpegAppHeaders>>;
        let mut adobe_color_transform = 0;
        let mut offset = 0;

        while offset < buffer.len() {
            let byte = buffer[offset];  // read byte
            if byte == 0xff { // header head
                let nextbyte :u8 = read_byte(&buffer,offset + 1);
                offset = offset + 2;

                match nextbyte {
                    0xc4 => { // DHT maker
                        _dht_flag = true;
                        let length = read_u16be(&buffer,offset) as usize;

                        let mut size :usize = 2;
                        while size < length {
                            let tc = read_byte(&buffer,offset + size) >> 4;
                            let th = read_byte(&buffer,offset + size) & 0x0f;

                            let ac = if tc == 0 { false } else { true };
                            let no = th as usize;
                            size = size + 1;
                            let mut pss :usize = 0;
                            let mut len :Vec<usize> = Vec::with_capacity(16);
                            let mut p :Vec<usize> = Vec::with_capacity(16);
                            let mut val :Vec<usize> = Vec::new();
                            let mut vlen = 0;
                            for i in 0..16 {
                                let l = read_byte(&buffer,offset + size + i) as usize;
                                p.push(pss);
                                vlen = vlen + l;
                                len.push(l);
                                for _ in 0..l {
                                    val.push(read_byte(&buffer,offset + size + 16 + pss) as usize);
                                    pss =  pss + 1;
                                }
                            }
                            size = size + 16;

                            _huffman_tables.push(HuffmanTable::new(ac,no,len,p,val));
                            size = size + vlen;
                        }

                        offset = offset + length; // skip
                    },
                    0xcc => {   //DAC no impl
                        let length = read_u16be(&buffer,offset) as usize;
                        offset = offset + length; // skip
                    },
                    0xc0..=0xcf => {  // SOF Frame Headers;
                        if !_sof_flag  {
                            _sof_flag = true;
                            let num = (nextbyte & 0x0f) as usize;
                            let length = read_u16be(&buffer,offset) as usize;
                            let buf = read_bytes(&buffer,offset + 2,length - 2);
                            let fh = FrameHeader::new(num,&buf);
                            width = fh.width;
                            height = fh.height;
                            bpp = fh.bitperpixel * fh.plane;
                            frame_header = Some(fh);
                            offset = offset + length; //skip
                        } else {
                            return Err(SimpleAddMessage(ErrorKind::DecodeError,"SOF Header Multiple".to_string()))
                        }
                    },
                    0xd8 => { // Start of Image
                        if include_soi {
                            _flag = true;
                        } else {
                            return Err(SimpleAddMessage(ErrorKind::DecodeError,"SOI Header Mutiple".to_string()))
                        }
                    },
                    0xd9=> { // End of Image
                        return Err(SimpleAddMessage(ErrorKind::DecodeError ,"Unexpect EOI".to_string()));
                    },
                    0xda=> { // SOS Scan header
                        _sos_flag = true;
                        let length: usize = read_u16be(&buffer,offset) as usize;
                        let mut ptr = offset + 2;
                        let ns = read_byte(&buffer,ptr) as usize;
                        ptr = ptr + 1;
                        let mut csn: Vec<usize> = Vec::with_capacity(ns);
                        let mut tdn: Vec<usize> = Vec::with_capacity(ns);
                        let mut tan: Vec<usize> = Vec::with_capacity(ns);
                        for _i in 0..ns {
                            csn.push(read_byte(&buffer,ptr) as usize);
                            tdn.push((read_byte(&buffer,ptr + 1) >> 4) as usize);
                            tan.push((read_byte(&buffer,ptr + 1) & 0xf ) as usize);
                            ptr = ptr + 2;
                        }
                        let ss = read_byte(&buffer,ptr) as usize;
                        let se = read_byte(&buffer,ptr) as usize;
                        let ah = (read_byte(&buffer,ptr + 2) >> 4) as usize;
                        let al = (read_byte(&buffer,ptr + 2) & 0xf ) as usize;
                        huffman_scan_header = Some(HuffmanScanHeader::new(ns,csn,tdn,tan,ss,se,ah,al));

                        offset = offset + length; //skip
                        break; // next is imagedata
                    },
                    0xdb =>{ // Define Quantization Table
                        _dqt_flag = true;
                        let length: usize = read_u16be(&buffer,offset) as usize;
                        // read_dqt;
                        let mut pos :usize = 2;
                        while pos < length {
                            let mut quantizations :Vec<usize> = Vec::with_capacity(64);
                            let presision :usize;
                            let p = read_byte(&buffer,pos + offset) >> 4;
                            let no = (read_byte(&buffer,pos + offset) & 0x0f) as usize;
                            pos = pos + 1;
                            if p == 0 {
                                presision = 8;
                                for _ in 0..64 {
                                    quantizations.push(read_byte(&buffer,pos + offset) as usize);
                                    pos = pos + 1;
                                }
                            } else {
                                presision = 16;
                                for _ in 0..64 {
                                    quantizations.push(read_u16be(&buffer,pos + offset) as usize);
                                    pos = pos + 2;
                                }
                            }
                            quantization_tables.push(QuantizationTable::new(presision,no,quantizations));
                        }
                        offset = offset + length; // skip
                    },
                    0xdc =>{ // DNL Define Number Lines
                        _dqt_flag = true;
                        if is_only_tables {
                            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Disallow DNL Header".to_string()))
                        }
                        let length: usize = read_u16be(&buffer,offset) as usize;
                        let nl = read_u16be(&buffer,offset) as usize;
                        line = nl;
                        // read_dqt;
                        offset = offset + length; // skip
                    },
                    0xdd => { // Define Restart Interval
                        let length = read_u16be(&buffer,offset) as usize;
                        let ri = read_u16be(&buffer,offset + 2);
                        interval = ri as usize;
                        offset = offset + length; // skip
                    },
                    0xde => {   // DHP Hierachical mode
                        if is_only_tables {
                            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Disallow DNP Header".to_string()))
                        }
                        let length = read_u16be(&buffer,offset) as usize;
                        is_hierachical = true;
                        offset = offset + length; // skip
                    },
                    0xdf => {   //EXP
                        if is_only_tables {
                            return Err(SimpleAddMessage(ErrorKind::DecodeError,"Disallow EXP Header".to_string()))
                        }
                        let length = read_u16be(&buffer,offset) as usize;
                        offset = offset + length; // skip
                    },
                    0xfe => { // Comment
                        let length = read_u16be(&buffer,offset) as usize;
                        comment = Some(read_string(buffer, offset, length- 2));
                        offset = offset + length; // skip
                    },
                    0xe0..=0xef => { // Applications 
                        let num = (nextbyte & 0xf) as usize;
                        let length = read_u16be(&buffer,offset) as usize;
                        let tag = read_string(buffer,offset + 2,length -2);
                        let len = length - 2 - tag.len() + 1;
                        let ptr = 2 + tag.len() + 1 + offset;
                        let result = read_app(num , &tag, &buffer[ptr..len+ptr], 0, len)?;
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
                        offset = offset + 1;
                    },
                    0x00 => { //data
                        // skip
                    },
                    0xd0..=0xd7 => {   // REST0-7
                    },
                    _ => {
                        let length = read_u16be(&buffer,offset) as usize;
                        offset = offset + length;
                    }
                }
            } else {
                return Err(SimpleAddMessage(ErrorKind::UnknownFormat,"Not Jpeg".to_string()));
            }

        }

        if _sof_flag && _sos_flag && _dht_flag && _dqt_flag == false {
            return Err(SimpleAddMessage(ErrorKind::IlligalData,"Maker is shortage".to_string()));
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
            imageoffset:  offset,
            comment,
            jpeg_app_headers,
            is_hierachical,
            adobe_color_transform,
        })
    }
}