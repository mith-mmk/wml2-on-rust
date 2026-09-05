//! PNG and APNG decoder implementation.

// use crate::color;
use crate::color::RGBA;
use crate::draw::*;
use crate::error::*;
use crate::png::header::*;
use crate::png::utils::make_metadata;
use crate::png::utils::paeth_dec;
use crate::png::warning::PngWarning;
use crate::warning::*;
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;

fn png_error(kind: ImgErrorKind, message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(kind, message.into()))
}

fn palette_entries(header: &PngHeader) -> Result<&[RGBA], Error> {
    header
        .pallete
        .as_deref()
        .ok_or_else(|| png_error(ImgErrorKind::IllegalData, "Palette data is missing."))
}

fn draw_rect(header: &PngHeader) -> (u32, u32) {
    if header.frame_controls.is_empty() {
        (header.width, header.height)
    } else {
        let last = header.frame_controls.len() - 1;
        (
            header.frame_controls[last].width,
            header.frame_controls[last].height,
        )
    }
}

// PNG filtering operates on encoded bytes, before unpacking or reducing samples.
const PASSES: [(usize, usize, usize, usize); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

fn channels(header: &PngHeader) -> Result<usize, Error> {
    match (header.color_type, header.bitpersample) {
        (0, 1 | 2 | 4 | 8 | 16) | (3, 1 | 2 | 4 | 8) => Ok(1),
        (4, 8 | 16) => Ok(2),
        (2, 8 | 16) => Ok(3),
        (6, 8 | 16) => Ok(4),
        _ => Err(png_error(
            ImgErrorKind::IllegalData,
            "invalid PNG sample format",
        )),
    }
}

fn passes(header: &PngHeader) -> &'static [(usize, usize, usize, usize)] {
    if header.interace_method == 0 {
        &[(0, 0, 1, 1)]
    } else {
        &PASSES
    }
}

fn pass_size(size: usize, start: usize, step: usize) -> usize {
    size.saturating_sub(start).div_ceil(step)
}

fn expected_length(header: &PngHeader) -> Result<usize, Error> {
    let (width, height) = draw_rect(header);
    if width == 0
        || height == 0
        || header.interace_method > 1
        || header.compression != 0
        || header.filter_method != 0
    {
        return Err(png_error(
            ImgErrorKind::IllegalData,
            "invalid PNG dimensions or methods",
        ));
    }
    let bits = channels(header)? * header.bitpersample as usize;
    let mut length = 0usize;
    for &(sx, sy, dx, dy) in passes(header) {
        let w = pass_size(width as usize, sx, dx);
        let h = pass_size(height as usize, sy, dy);
        if w == 0 || h == 0 {
            continue;
        }
        length = w
            .checked_mul(bits)
            .and_then(|v| v.checked_add(7))
            .map(|v| v / 8)
            .and_then(|v| v.checked_add(1))
            .and_then(|v| v.checked_mul(h))
            .and_then(|v| v.checked_add(length))
            .ok_or_else(|| png_error(ImgErrorKind::IllegalData, "PNG scanline length overflow"))?;
    }
    Ok(length)
}

fn inflate_image(header: &PngHeader, data: &[u8]) -> Result<Vec<u8>, Error> {
    let expected = expected_length(header)?;
    let output = crate::limits::inflate_image(data, expected)
        .map_err(|error| png_error(ImgErrorKind::DecodeError, format!("PNG inflate: {error:?}")))?;
    if output.len() != expected {
        return Err(png_error(
            ImgErrorKind::IllegalData,
            "PNG scanline data length mismatch",
        ));
    }
    Ok(output)
}

pub(crate) fn validate_frame(header: &PngHeader, frame: &FrameControl) -> Result<(), Error> {
    if frame.width == 0
        || frame.height == 0
        || frame
            .x_offset
            .checked_add(frame.width)
            .is_none_or(|x| x > header.width)
        || frame
            .y_offset
            .checked_add(frame.height)
            .is_none_or(|y| y > header.height)
        || frame.dispose_op > 2
        || frame.blend_op > 1
    {
        return Err(png_error(
            ImgErrorKind::IllegalData,
            "invalid APNG frame rectangle or control",
        ));
    }
    Ok(())
}

fn load(
    header: &mut PngHeader,
    buffer: &[u8],
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    if buffer.len() != expected_length(header)? {
        return Err(png_error(
            ImgErrorKind::IllegalData,
            "PNG scanline data length mismatch",
        ));
    }
    let (width, height) = draw_rect(header);
    let channels = channels(header)?;
    let depth = header.bitpersample as usize;
    let bpp = (channels * depth).div_ceil(8);
    let mut cursor = 0;
    for &(sx, sy, dx, dy) in passes(header) {
        let w = pass_size(width as usize, sx, dx);
        let h = pass_size(height as usize, sy, dy);
        if w == 0 || h == 0 {
            continue;
        }
        let row_bytes = (w * channels * depth).div_ceil(8);
        let mut previous = vec![0u8; row_bytes];
        let mut row = vec![0u8; row_bytes];
        let mut rgba = vec![0u8; w * 4];
        for py in 0..h {
            let filter = buffer[cursor];
            cursor += 1;
            if filter > 4 {
                return Err(png_error(ImgErrorKind::IllegalData, "invalid PNG filter"));
            }
            row.copy_from_slice(&buffer[cursor..cursor + row_bytes]);
            cursor += row_bytes;
            for i in 0..row_bytes {
                let a = if i >= bpp { row[i - bpp] } else { 0 };
                let b = previous[i];
                let c = if i >= bpp { previous[i - bpp] } else { 0 };
                row[i] = match filter {
                    0 => row[i],
                    1 => row[i].wrapping_add(a),
                    2 => row[i].wrapping_add(b),
                    3 => row[i].wrapping_add(((a as u16 + b as u16) / 2) as u8),
                    _ => paeth_dec(row[i], a as i32, b as i32, c as i32),
                };
            }
            let sample = |index: usize| -> u16 {
                match depth {
                    16 => u16::from_be_bytes([row[index * 2], row[index * 2 + 1]]),
                    8 => row[index] as u16,
                    _ => {
                        ((row[index * depth / 8] >> (8 - depth - index * depth % 8))
                            & ((1 << depth) - 1)) as u16
                    }
                }
            };
            let reduce = |value: u16| -> u8 {
                if depth == 16 {
                    (value >> 8) as u8
                } else {
                    (value as u32 * 255 / ((1 << depth) - 1)) as u8
                }
            };
            for x in 0..w {
                let i = x * channels;
                let mut pixel = match header.color_type {
                    0 | 4 => {
                        let g = reduce(sample(i));
                        [
                            g,
                            g,
                            g,
                            if channels == 2 {
                                reduce(sample(i + 1))
                            } else {
                                255
                            },
                        ]
                    }
                    2 | 6 => [
                        reduce(sample(i)),
                        reduce(sample(i + 1)),
                        reduce(sample(i + 2)),
                        if channels == 4 {
                            reduce(sample(i + 3))
                        } else {
                            255
                        },
                    ],
                    3 => {
                        let palette = palette_entries(header)?;
                        let p = palette.get(sample(i) as usize).ok_or_else(|| {
                            png_error(ImgErrorKind::IllegalData, "PNG palette index out of range")
                        })?;
                        [p.red, p.green, p.blue, p.alpha]
                    }
                    _ => unreachable!(),
                };
                if let Some(trns) = &header.transparency {
                    if header.color_type == 0
                        && trns.len() == 2
                        && sample(i) == u16::from_be_bytes([trns[0], trns[1]])
                    {
                        pixel[3] = 0;
                    }
                    if header.color_type == 2
                        && trns.len() == 6
                        && (0..3).all(|c| {
                            sample(i + c) == u16::from_be_bytes([trns[c * 2], trns[c * 2 + 1]])
                        })
                    {
                        pixel[3] = 0;
                    }
                }
                rgba[x * 4..x * 4 + 4].copy_from_slice(&pixel);
            }
            if header.interace_method == 0 {
                option.drawer.draw(0, py, w, 1, &rgba, None)?;
            } else {
                for x in 0..w {
                    option.drawer.draw(
                        sx + x * dx,
                        sy + py * dy,
                        1,
                        1,
                        &rgba[x * 4..x * 4 + 4],
                        None,
                    )?;
                }
            }
            std::mem::swap(&mut row, &mut previous);
        }
    }
    Ok(None)
}

fn next_options(frame_control: &FrameControl) -> NextOptions {
    let flag = NextOption::Continue;
    let image_rect = Some(ImageRect {
        start_x: frame_control.x_offset as i32,
        start_y: frame_control.y_offset as i32,
        width: frame_control.width as usize,
        height: frame_control.height as usize,
    });
    let delay_den = if frame_control.delay_den == 0 {
        100
    } else {
        frame_control.delay_den
    };
    let await_time = ((frame_control.delay_num as f32 / delay_den as f32) * 1000.0) as u64;
    let dispose_option = Some(match frame_control.dispose_op {
        1 => NextDispose::Background,
        2 => NextDispose::Previous,
        _ => NextDispose::None,
    });

    let blend = Some(match frame_control.blend_op {
        1 => NextBlend::Source,
        _ => NextBlend::Override,
    });
    NextOptions {
        flag,
        await_time,
        image_rect,
        dispose_option,
        blend,
    }
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    crate::decode_guard::run(
        reader,
        option,
        crate::limits::DecodeLimits::default(),
        decode_inner,
    )
}

pub(crate) fn decode_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let mut header = PngHeader::new(reader, option.debug_flag)?;
    expected_length(&header)?;
    for frame in &header.frame_controls {
        validate_frame(&header, frame)?;
    }

    let backgroud = if let Some(ref background) = header.background_color {
        let background = match background {
            BacgroundColor::Grayscale(gray) => RGBA {
                red: *gray as u8,
                green: *gray as u8,
                blue: *gray as u8,
                alpha: 0xff,
            },
            BacgroundColor::TrueColor((red, green, blue)) => RGBA {
                red: *red as u8,
                green: *green as u8,
                blue: *blue as u8,
                alpha: 0xff,
            },
            BacgroundColor::Index(index) => {
                let index = *index as usize;
                let pallete = header.pallete.as_deref().ok_or_else(|| {
                    png_error(
                        ImgErrorKind::IllegalData,
                        "background color references missing palette",
                    )
                })?;
                if index >= pallete.len() {
                    return Err(png_error(
                        ImgErrorKind::OutboundIndex,
                        format!("background palette index {} is out of range", index),
                    ));
                }
                let r = pallete[index].red;
                let g = pallete[index].green;
                let b = pallete[index].blue;
                RGBA {
                    red: r,
                    green: g,
                    blue: b,
                    alpha: 0xff,
                }
            }
        };
        Some(background) // RGBA
    } else {
        None
    };

    let opt = if header.is_apng {
        Some(InitOptions {
            loop_count: header.num_plays,
            background: backgroud,
            animation: true,
        })
    } else {
        Some(InitOptions {
            loop_count: 0,
            background: backgroud,
            animation: false,
        })
    };

    option
        .drawer
        .init(header.width as usize, header.height as usize, opt)?;
    if option.debug_flag > 0 {
        let mut s = "PNG\n".to_string();
        let s_ = format!(
            "width {} height {}  {} bits per sample\n",
            header.width, header.height, header.bitpersample
        );
        s += &s_;
        let s_ = match header.color_type {
            0 => "Color type: Glayscale\n",
            2 => "Color type: Truecolor\n",
            3 => "Color type: Index Color\n",
            4 => "Color type: Glayscale with alpha\n",
            6 => "Color type: Truecolor with alpha\n",
            _ => "Color type: unkwon\n",
        };
        s += s_;
        let s_ = format!("Transparency {:?}\n", header.transparency);
        s += &s_;
        let s_ = format!("Backgroud color {:?}\n", header.background_color);
        s += &s_;
        let s_ = format!("Pallet {:?}\n", header.pallete);
        s += &s_;

        let s_ = format!("Modified time {:?}\n", header.modified_time);
        s += &s_;
        for (key, mes) in &header.text {
            let s_ = format!("{} : {}", key, mes);
            s += &s_;
        }
        option.drawer.verbose(&s, None)?;
        if !header.frame_controls.is_empty() {
            let s = format!("{:?}", header.frame_controls[0]);
            option.drawer.verbose(&s, None)?;
        }
    }

    let mut buffer: Vec<u8> = Vec::new();
    let mut idat = true;
    let mut allow_multi_image = false;

    loop {
        let length = reader.read_u32_be()?;
        let ret_chunck = reader.read_bytes_as_vec(4);
        validate_chunk_available(reader, length)?;
        match ret_chunck {
            Ok(chunck) => {
                if chunck == IMAGE_DATA {
                    if option.debug_flag > 1 {
                        let string = format!("read compressed image data {} bytes", length);
                        option.drawer.verbose(&string, None)?;
                    }
                    let mut buf = reader.read_bytes_as_vec(length as usize)?;
                    buffer.append(&mut buf);
                    let _crc = reader.read_u32_be()?;
                } else {
                    if idat {
                        let decomressed = inflate_image(&header, &buffer);
                        match decomressed {
                            Ok(debuffer) => {
                                load(&mut header.clone(), &debuffer, option)?;
                                if !header.frame_controls.is_empty() {
                                    let frame_control = &header.frame_controls[0];
                                    let next = next_options(frame_control);
                                    let result = option.drawer.next(Some(next))?;
                                    if let Some(response) = result {
                                        if response.response == ResponseCommand::Continue {
                                            allow_multi_image = true;
                                            load(&mut header.clone(), &debuffer, option)?;
                                            // Image = Animation Frame 0
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                let message = format!("Uncompressed Error {:?}", err);
                                return Err(Box::new(ImgError::new_const(
                                    ImgErrorKind::DecodeError,
                                    message,
                                )));
                            }
                        }

                        idat = false;
                        buffer = vec![];
                    }
                    if chunck == IMAGE_END {
                        if length != 0 {
                            return Err(png_error(
                                ImgErrorKind::IllegalData,
                                "IEND length must be zero",
                            ));
                        }
                        let _crc = reader.read_u32_be()?;
                        if !buffer.is_empty() {
                            let decomressed = inflate_image(&header, &buffer);
                            match decomressed {
                                Ok(debuffer) => {
                                    load(&mut header, &debuffer, option)?;
                                }
                                Err(err) => {
                                    let message = format!("Uncompressed Error {:?}", err);
                                    return Err(Box::new(ImgError::new_const(
                                        ImgErrorKind::DecodeError,
                                        message,
                                    )));
                                }
                            }
                        }
                        break;
                    } else if chunck == TEXTDATA || chunck == I18N_TEXT {
                        crate::limits::check(
                            length as usize,
                            crate::limits::current().metadata_bytes,
                            "PNG text",
                        )?;
                        let text = reader.read_bytes_as_vec(length as usize)?;
                        let (keyword, string) = to_string(&text, false)?;
                        header.text.push((keyword, string));
                        let _crc = reader.read_u32_be()?;
                    } else if chunck == COMPRESSED_TEXTUAL_DATA {
                        crate::limits::check(
                            length as usize,
                            crate::limits::current().metadata_bytes,
                            "compressed PNG text",
                        )?;
                        let text = reader.read_bytes_as_vec(length as usize)?;
                        let (keyword, string) = to_string(&text, true)?;
                        header.text.push((keyword, string));
                        let _crc = reader.read_u32_be()?;
                    } else if chunck == C2PA_CHUNK {
                        #[cfg(feature = "c2pa")]
                        {
                            crate::limits::charge_metadata(length as usize)?;
                            let mut c2pa = reader.read_bytes_as_vec(length as usize)?;
                            header.c2pa.get_or_insert_with(Vec::new).append(&mut c2pa);
                        }
                        #[cfg(not(feature = "c2pa"))]
                        {
                            reader.skip_ptr(length as usize)?;
                        }
                        let _crc = reader.read_u32_be()?;
                    } else if chunck == ANIMATION_CONTROLE {
                        // noimpl error!
                        reader.skip_ptr(length as usize)?;
                        let _crc = reader.read_u32_be()?;
                    } else if chunck == FRAME_CONTROLE {
                        if length != 26 {
                            return Err(png_error(
                                ImgErrorKind::IllegalData,
                                "fcTL length must be 26",
                            ));
                        }
                        let frame_control = FrameControl {
                            sequence_number: reader.read_u32_be()?,
                            width: reader.read_u32_be()?,
                            height: reader.read_u32_be()?,
                            x_offset: reader.read_u32_be()?,
                            y_offset: reader.read_u32_be()?,
                            delay_num: reader.read_u16_be()?,
                            delay_den: reader.read_u16_be()?,
                            dispose_op: reader.read_byte()?,
                            blend_op: reader.read_byte()?,
                        };
                        validate_frame(&header, &frame_control)?;
                        if !buffer.is_empty() && allow_multi_image {
                            let decomressed = inflate_image(&header, &buffer);
                            match decomressed {
                                Ok(debuffer) => {
                                    load(&mut header, &debuffer, option)?;
                                }
                                Err(err) => {
                                    let message = format!("Uncompressed Error {:?}", err);
                                    return Err(Box::new(ImgError::new_const(
                                        ImgErrorKind::DecodeError,
                                        message,
                                    )));
                                }
                            }
                        }
                        buffer = vec![];

                        let next = next_options(&frame_control);
                        let result = option.drawer.next(Some(next))?;
                        if let Some(response) = result {
                            if response.response == ResponseCommand::Continue {
                                allow_multi_image = true;
                            }
                            if option.debug_flag > 0 {
                                let str = format!("{:?}", frame_control);
                                option.drawer.verbose(&str, None)?;
                            }
                        }

                        crate::limits::check(
                            header.frame_controls.len().saturating_add(1),
                            crate::limits::current().frames,
                            "APNG frame controls",
                        )?;
                        header.frame_controls.push(frame_control);

                        let _crc = reader.read_u32_be()?;
                    } else if chunck == FRAME_DATA {
                        if length < 4 {
                            return Err(png_error(
                                ImgErrorKind::IllegalData,
                                "fdAT length must be at least 4",
                            ));
                        }
                        let sequence_number = reader.read_u32_be()?;
                        if option.debug_flag > 0 {
                            let string = format!(
                                "read compressed animation image data:{} {} bytes",
                                sequence_number,
                                length - 4
                            );
                            option.drawer.verbose(&string, None)?;
                        }

                        let mut buf = reader.read_bytes_as_vec(length as usize - 4)?;
                        buffer.append(&mut buf);
                        let _crc = reader.read_u32_be()?;
                    } else {
                        reader.skip_ptr(length as usize)?;
                        let _crc = reader.read_u32_be()?;
                    }
                }
            }
            Err(_) => {
                let warnings = ImgWarnings::add(
                    None,
                    Box::new(PngWarning::new(
                        "Data crruption after image datas".to_string(),
                    )),
                );
                if option.debug_flag > 1 {
                    let string = format!("{:?}", &header);
                    option.drawer.verbose(&string, None)?;
                }
                option.drawer.terminate(None)?;
                return Ok(warnings);
            }
        }
    }
    if option.debug_flag > 1 {
        let string = format!("{:?}", &header);
        option.drawer.verbose(&string, None)?;
    }
    let map = make_metadata(&header)?;
    for (key, value) in &map {
        option.drawer.set_metadata(key, value.clone())?;
    }

    option.drawer.terminate(None)?;
    Ok(None)
}
