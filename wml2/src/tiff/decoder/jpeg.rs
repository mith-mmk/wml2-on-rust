type Error = Box<dyn std::error::Error>;
use crate::draw::ImageBuffer;
use crate::tiff::header::*;
use crate::warning::ImgWarnings;
use crate::draw::DecodeOptions;
use bin_rs::reader::BinaryReader;

// Tiff in JPEG is a multi parts image.
pub fn decode_jpeg_compresson<'decode,B: BinaryReader>(reader:&mut B,option:&mut DecodeOptions,header: &Tiff)-> Result<Option<ImgWarnings>,Error> {

    let jpeg_tables = &header.jpeg_tables;
    let metadata;
    if jpeg_tables.len() == 0 {
        metadata = vec![0xff,0xd8]; // SOI
    } else {
        let len = jpeg_tables.len() - 2;
        metadata = (&jpeg_tables[..len]).to_vec();  // remove EOI
    }
    let mut warnings:Option<ImgWarnings> = None;
    let mut y = 0;
    option.drawer.init(header.width as usize,header.height as usize,None)?;
    for (i,offset) in header.strip_offsets.iter().enumerate() {
        reader.seek(std::io::SeekFrom::Start(*offset as u64))?;
        let mut data = vec![];
        data.append(&mut metadata.to_vec());
        let buf = reader.read_bytes_as_vec(header.strip_byte_counts[i] as usize)?;
        data.append(&mut buf[2..].to_vec());    // remove SOI

        let mut image = ImageBuffer::new();       
        let mut part_option = DecodeOptions{
            debug_flag: option.debug_flag,
            drawer: &mut image,
        };
        let mut reader = bin_rs::reader::BytesReader::from_vec(data);
        let ws = crate::jpeg::decoder::decode(&mut reader,&mut part_option)?;
        let width = image.width;
        let height = image.height;

        if image.buffer.is_some() {
            option.drawer.draw(0,y,width,height,&image.buffer.unwrap(),None)?;
        }

        y += header.rows_per_strip as usize;

        warnings = ImgWarnings::append(warnings,ws);
    }

    Ok(warnings)
}