type Error = Box<dyn std::error::Error>;

use super::header::*;
use crate::color::RGBA;
use crate::decoder::lzw::Lzwdecode;
use crate::draw::*;
use crate::error::ImgError;
use crate::error::ImgErrorKind;
use crate::metadata::DataMap;
use crate::warning::ImgWarnings;
use bin_rs::io::read_ascii_string;
use bin_rs::reader::BinaryReader;

const SEPARATER: u8 = b','; // 0x2c
const EXTEND_BLOCK: u8 = b'!'; // 0x21
const COMMENT_LABEL: u8 = 0xfe;
const GRAPHIC_CONTROLE: u8 = 0xf9;
const END_MARKER: u8 = b';'; // 0x3c
const END: u8 = 0x00;

pub fn decode<'decode, B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let mut header = GifHeader::new(reader, option.debug_flag)?;
    let mut ptr = header.header_size;
    let mut comment = "".to_string();
    let mut is_transpearent = false;
    let mut transperarent_color = 0x00;
    let mut delay_time = 0;
    let mut loop_count = 0;
    let mut is_inited = false;
    let mut is_first = true;
    let warnings: Option<ImgWarnings> = None;

    if option.debug_flag > 0 {
        option.drawer.verbose(&format!("{:?}", &header), None)?;
    }
    option
        .drawer
        .set_metadata("Format", DataMap::Ascii("GIF".to_string()))?;
    option
        .drawer
        .set_metadata("version", DataMap::Ascii(header.version.to_string()))?;
    option
        .drawer
        .set_metadata("width", DataMap::UInt(header.width as u64))?;
    option
        .drawer
        .set_metadata("heigth", DataMap::UInt(header.height as u64))?;
    let mut comment_count = 0;

    loop {
        let c = reader.read_byte()?;

        match c {
            // BLOCK LOOP
            END => {}
            EXTEND_BLOCK => {
                let ext = reader.read_byte()?;
                match ext {
                    END => {}
                    COMMENT_LABEL => {
                        let mut s = "".to_string();
                        comment_count += 1;
                        loop {
                            let len = reader.read_byte()? as usize;
                            if len == 0 {
                                break;
                            }
                            if ext == COMMENT_LABEL {
                                let comment = reader.read_ascii_string(len)?;
                                s = s.to_owned() + &comment;
                            }
                            ptr += len;
                        }
                        if option.debug_flag > 0 {
                            option
                                .drawer
                                .verbose(&("Comment: ".to_owned() + &s), None)?;
                        }
                        let string = format!("comment:{}", comment_count);
                        option
                            .drawer
                            .set_metadata(&string, DataMap::Ascii(s.to_string()))?;
                        comment += &s;
                    }
                    GRAPHIC_CONTROLE => {
                        let _len = reader.read_byte()? as usize; //5
                        let flag = reader.read_byte()?;
                        delay_time = reader.read_u16_le()?;

                        if is_transpearent {
                            header.color_table[transperarent_color].alpha = 0xff;
                        }

                        is_transpearent = flag & 0x1 == 1;

                        transperarent_color = reader.read_byte()? as usize;
                        if option.debug_flag > 0 {
                            let s = format!(
                                "Grahic Controle {} delay {}ms  transpearent {:?}",
                                flag, delay_time, is_transpearent
                            );
                            option.drawer.verbose(&s, None)?;
                        }
                        reader.read_byte()?; // is 00
                    }
                    0xff => {
                        // Netscape2.0 (Animation Flag)
                        let len = reader.read_byte()? as usize;
                        if len == 0 {
                            break;
                        }
                        let buf = reader.read_bytes_as_vec(len)?;
                        let s = read_ascii_string(&buf, 0, len);
                        loop {
                            let len = reader.read_byte()? as usize;
                            if len == 0 {
                                break;
                            }
                            let id = reader.read_byte()?;

                            if s == "NETSCAPE2.0" {
                                option.drawer.set_metadata(
                                    "Animation GIF",
                                    DataMap::Ascii("NETSCAPE2.0".to_string()),
                                )?;
                                if option.debug_flag > 0 {
                                    option
                                        .drawer
                                        .verbose(&("Animation tag: ".to_owned() + &s), None)?;
                                }
                                if id == 0x01 {
                                    loop_count = reader.read_u16_le()? as u32;
                                } else {
                                    reader.skip_ptr(len)?;
                                }
                            } else {
                                reader.skip_ptr(len)?;
                            }
                        }
                    }
                    _ => loop {
                        let len = reader.read_byte()? as usize;
                        if len == 0 {
                            break;
                        }
                        ptr += len;
                    },
                }
            }

            SEPARATER => {
                let lscd = GifLscd::new(reader)?;
                if !is_inited {
                    let init = InitOptions {
                        loop_count,
                        background: None,
                        animation: true,
                    };
                    option
                        .drawer
                        .init(header.width, header.height, Some(init))?;
                    is_inited = true;
                }

                if !is_first {
                    let rect = ImageRect {
                        width: lscd.xsize as usize,
                        height: lscd.ysize as usize,
                        start_x: lscd.xstart as i32,
                        start_y: lscd.ystart as i32,
                    };

                    let await_time = (delay_time * 10) as u64;

                    let dispose_option: NextDispose = match lscd.field {
                        0 => NextDispose::None,
                        1 => NextDispose::Override,
                        2 => NextDispose::Background,
                        3 => NextDispose::Previous,
                        _ => NextDispose::None,
                    };

                    let opt = NextOptions {
                        flag: NextOption::Next,
                        await_time,
                        image_rect: Some(rect),
                        dispose_option: Some(dispose_option),
                        blend: Some(NextBlend::Override),
                    };

                    let result = option.drawer.next(Some(opt))?;
                    if let Some(response) = result {
                        if response.response == ResposeCommand::Abort {
                            return Ok(None);
                        }
                    }
                }
                ptr += 9;
                let has_local_pallet;
                let mut local_color_table = Vec::new();
                if lscd.field & 0x80 == 0x80 {
                    has_local_pallet = true;
                    let color_table_size = (1 << ((lscd.field & 0x07) + 1)) as usize;
                    for _ in 0..color_table_size {
                        let color = RGBA {
                            red: reader.read_byte()?,
                            green: reader.read_byte()?,
                            blue: reader.read_byte()?,
                            alpha: 0xff,
                        };
                        local_color_table.push(color);
                    }
                    if is_transpearent {
                        local_color_table[transperarent_color].alpha = 0x00;
                    }
                } else {
                    has_local_pallet = false;
                    if is_transpearent {
                        header.color_table[transperarent_color].alpha = 0x00;
                    }
                    header.color_table[transperarent_color].alpha = 0xff;
                }
                // LZW block
                let lzw_min_bits = reader.read_byte()? as usize;
                let mut buf: Vec<u8> = Vec::new();
                'lzw_read: loop {
                    let len = reader.read_byte()? as usize;
                    if len == 0 {
                        break 'lzw_read;
                    }
                    buf.append(&mut reader.read_bytes_as_vec(len)?);
                }
                let mut decoder = Lzwdecode::gif(lzw_min_bits);
                let data = decoder.decode(&buf)?;
                let color_table = if has_local_pallet {
                    &local_color_table
                } else {
                    &header.color_table
                };

                let width = lscd.xsize as usize;
                let height = lscd.ysize as usize;
                if option.debug_flag > 0 {
                    option.drawer.verbose(&format!("{:?}", lscd), None)?;
                    option.drawer.verbose(
                        &format!(
                            "{} {} {} data length {}",
                            width,
                            height,
                            width * height,
                            data.len()
                        ),
                        None,
                    )?;
                }

                let interlace_start_y = [0, 4, 2, 1];
                let interlace_delta_y = [8, 8, 4, 2];
                let mut interlace_mode = 0;
                let mut interlace_y = interlace_start_y[interlace_mode];
                let is_interlace = (lscd.field & 0x40) == 0x40;

                for y in 0..height {
                    let mut line: Vec<u8> = vec![0; width * 4];
                    let offset = y * width;
                    for x in 0..width {
                        let color = data[offset + x] as usize;
                        line[x * 4] = color_table[color].red;
                        line[x * 4 + 1] = color_table[color].green;
                        line[x * 4 + 2] = color_table[color].blue;
                        line[x * 4 + 3] = color_table[color].alpha;
                    }
                    if is_interlace {
                        if is_first {
                            let y = interlace_y + lscd.ystart as usize;
                            option
                                .drawer
                                .draw(lscd.xstart as usize, y, width, 1, &line, None)?;
                        } else {
                            option.drawer.draw(0, interlace_y, width, 1, &line, None)?;
                        }
                        if interlace_y == 16 {
                            interlace_y += 8;
                        } else {
                            interlace_y += interlace_delta_y[interlace_mode];
                        }
                        if interlace_y >= height {
                            interlace_mode += 1;
                            if interlace_mode >= interlace_start_y.len() {
                                break;
                            }
                            interlace_y = interlace_start_y[interlace_mode];
                        }
                    } else if is_first {
                        let y = y + lscd.ystart as usize;
                        option
                            .drawer
                            .draw(lscd.xstart as usize, y, width, 1, &line, None)?;
                    } else {
                        option.drawer.draw(0, y, width, 1, &line, None)?;
                    }
                }

                if is_first {
                    is_first = false;
                }
            }
            END_MARKER => {
                break;
            }
            _ => {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::IllegalData,
                    "read error in gif decode".to_string(),
                )))
            }
        };
    }

    Ok(warnings)
}
