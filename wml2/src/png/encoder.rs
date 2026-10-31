//! PNG and APNG encoder implementation.

use crate::draw::{
    ENCODE_ANIMATION_FRAMES_KEY, ENCODE_ANIMATION_LOOP_COUNT_KEY, EncodeOptions, ImageProfiles,
    encode_animation_frame_key,
};
use crate::error::*;
use crate::metadata::{DataMap, get_exif_option};
use crate::png::header::*;
use crate::png::utils::*;
use bin_rs::io::*;
type Error = Box<dyn std::error::Error>;

struct ApngFrame {
    width: u32,
    height: u32,
    x_offset: u32,
    y_offset: u32,
    delay_num: u16,
    delay_den: u16,
    dispose_op: u8,
    blend_op: u8,
    buffer: Vec<u8>,
}

struct ApngInfo {
    loop_count: u32,
    frames: Vec<ApngFrame>,
}

fn source_over(dst: &mut [u8], src: &[u8]) {
    let src_alpha = src[3] as u32;
    if src_alpha == 0 {
        return;
    }
    if src_alpha == 255 {
        dst.copy_from_slice(src);
        return;
    }

    let dst_alpha = dst[3] as u32;
    let out_alpha = src_alpha + ((dst_alpha * (255 - src_alpha) + 127) / 255);
    if out_alpha == 0 {
        dst.copy_from_slice(&[0, 0, 0, 0]);
        return;
    }

    for channel in 0..3 {
        let src_premul = src[channel] as u32 * src_alpha;
        let dst_premul = dst[channel] as u32 * dst_alpha;
        let out_premul = src_premul + ((dst_premul * (255 - src_alpha) + 127) / 255);
        dst[channel] = ((out_premul * 255 + (out_alpha / 2)) / out_alpha) as u8;
    }
    dst[3] = out_alpha as u8;
}

fn fill_canvas(width: u32, height: u32, background: Option<crate::color::RGBA>) -> Vec<u8> {
    let background = background.unwrap_or(crate::color::RGBA {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    });
    let mut canvas = vec![0u8; width as usize * height as usize * 4];
    for pixel in canvas.chunks_exact_mut(4) {
        pixel[0] = background.red;
        pixel[1] = background.green;
        pixel[2] = background.blue;
        pixel[3] = background.alpha;
    }
    canvas
}

fn apply_frame(canvas: &mut [u8], canvas_width: u32, frame: &ApngFrame) {
    let canvas_width = canvas_width as usize;
    for y in 0..frame.height as usize {
        let src_row = y * frame.width as usize * 4;
        let dst_row = (frame.y_offset as usize + y) * canvas_width * 4;
        for x in 0..frame.width as usize {
            let src_offset = src_row + x * 4;
            let dst_offset = dst_row + (frame.x_offset as usize + x) * 4;
            if frame.blend_op == 1 {
                source_over(
                    &mut canvas[dst_offset..dst_offset + 4],
                    &frame.buffer[src_offset..src_offset + 4],
                );
            } else {
                canvas[dst_offset..dst_offset + 4]
                    .copy_from_slice(&frame.buffer[src_offset..src_offset + 4]);
            }
        }
    }
}

fn clear_frame(
    canvas: &mut [u8],
    canvas_width: u32,
    frame: &ApngFrame,
    background: Option<crate::color::RGBA>,
) {
    let background = background.unwrap_or(crate::color::RGBA {
        red: 0,
        green: 0,
        blue: 0,
        alpha: 0,
    });
    let canvas_width = canvas_width as usize;
    for y in 0..frame.height as usize {
        let dst_row = (frame.y_offset as usize + y) * canvas_width * 4;
        for x in 0..frame.width as usize {
            let dst_offset = dst_row + (frame.x_offset as usize + x) * 4;
            canvas[dst_offset] = background.red;
            canvas[dst_offset + 1] = background.green;
            canvas[dst_offset + 2] = background.blue;
            canvas[dst_offset + 3] = background.alpha;
        }
    }
}

fn normalize_apng_frames(
    width: u32,
    height: u32,
    background: Option<crate::color::RGBA>,
    apng: ApngInfo,
) -> Result<ApngInfo, Error> {
    let mut canvas = fill_canvas(width, height, background.clone());
    let mut normalized = Vec::with_capacity(apng.frames.len());

    for frame in apng.frames {
        let previous = (frame.dispose_op == 2).then(|| canvas.clone());
        apply_frame(&mut canvas, width, &frame);
        normalized.push(ApngFrame {
            width,
            height,
            x_offset: 0,
            y_offset: 0,
            delay_num: frame.delay_num,
            delay_den: frame.delay_den,
            dispose_op: 0,
            blend_op: 0,
            buffer: canvas.clone(),
        });

        match frame.dispose_op {
            1 => clear_frame(&mut canvas, width, &frame, background.clone()),
            2 => {
                canvas = previous.ok_or_else(|| {
                    Box::new(ImgError::new_const(
                        ImgErrorKind::EncodeError,
                        "missing previous canvas for dispose=previous".to_string(),
                    )) as Error
                })?;
            }
            _ => {}
        }
    }

    Ok(ApngInfo {
        loop_count: apng.loop_count,
        frames: normalized,
    })
}

fn write_chunk(write_buffer: &mut Vec<u8>, crc32: &CRC32, chunk_type: &[u8; 4], data: &[u8]) {
    write_u32_be(data.len() as u32, write_buffer);
    let mut temp_buffer = Vec::with_capacity(4 + data.len());
    write_bytes(chunk_type, &mut temp_buffer);
    write_bytes(data, &mut temp_buffer);
    write_bytes(&temp_buffer, write_buffer);
    let crc = crc32.crc32(&temp_buffer);
    write_u32_be(crc, write_buffer);
}

fn to_u32(value: Option<&DataMap>, key: &str) -> Result<u32, Error> {
    match value {
        Some(DataMap::UInt(value)) => u32::try_from(*value).map_err(|_| {
            Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("{key} exceeds u32"),
            )) as Error
        }),
        Some(_) => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} is not UInt metadata"),
        ))),
        None => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} metadata not found"),
        ))),
    }
}

fn to_i32(value: Option<&DataMap>, key: &str) -> Result<i32, Error> {
    match value {
        Some(DataMap::SInt(value)) => i32::try_from(*value).map_err(|_| {
            Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("{key} exceeds i32"),
            )) as Error
        }),
        Some(DataMap::UInt(value)) => i32::try_from(*value).map_err(|_| {
            Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("{key} exceeds i32"),
            )) as Error
        }),
        Some(_) => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} is not integer metadata"),
        ))),
        None => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} metadata not found"),
        ))),
    }
}

fn to_raw(value: Option<&DataMap>, key: &str) -> Result<Vec<u8>, Error> {
    match value {
        Some(DataMap::Raw(value)) => Ok(value.clone()),
        Some(_) => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} is not Raw metadata"),
        ))),
        None => Err(Box::new(ImgError::new_const(
            ImgErrorKind::EncodeError,
            format!("{key} metadata not found"),
        ))),
    }
}

fn encode_delay(delay_ms: u64) -> (u16, u16) {
    if delay_ms <= u16::MAX as u64 {
        (delay_ms as u16, 1000)
    } else {
        let seconds = ((delay_ms + 500) / 1000).min(u16::MAX as u64);
        (seconds as u16, 1)
    }
}

fn parse_apng_info(profile: &ImageProfiles) -> Result<Option<ApngInfo>, Error> {
    let Some(metadata) = &profile.metadata else {
        return Ok(None);
    };
    let Some(DataMap::UInt(frame_count)) = metadata.get(ENCODE_ANIMATION_FRAMES_KEY) else {
        return Ok(None);
    };
    if *frame_count == 0 {
        return Ok(None);
    }

    let loop_count = match metadata.get(ENCODE_ANIMATION_LOOP_COUNT_KEY) {
        Some(DataMap::UInt(loop_count)) => *loop_count as u32,
        Some(_) => {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                "wml2.animation.loop_count is not UInt metadata".to_string(),
            )));
        }
        None => 0,
    };

    let mut frames = Vec::with_capacity(*frame_count as usize);
    for index in 0..*frame_count as usize {
        let width_key = encode_animation_frame_key(index, "width");
        let height_key = encode_animation_frame_key(index, "height");
        let start_x_key = encode_animation_frame_key(index, "start_x");
        let start_y_key = encode_animation_frame_key(index, "start_y");
        let delay_key = encode_animation_frame_key(index, "delay_ms");
        let dispose_key = encode_animation_frame_key(index, "dispose");
        let blend_key = encode_animation_frame_key(index, "blend");
        let buffer_key = encode_animation_frame_key(index, "buffer");

        let width = to_u32(metadata.get(&width_key), &width_key)?;
        let height = to_u32(metadata.get(&height_key), &height_key)?;
        let x_offset = to_i32(metadata.get(&start_x_key), &start_x_key)?;
        let y_offset = to_i32(metadata.get(&start_y_key), &start_y_key)?;
        let delay_ms = to_u32(metadata.get(&delay_key), &delay_key)? as u64;
        let dispose_op = to_u32(metadata.get(&dispose_key), &dispose_key)? as u8;
        let blend_op = to_u32(metadata.get(&blend_key), &blend_key)? as u8;
        let buffer = to_raw(metadata.get(&buffer_key), &buffer_key)?;

        if width == 0 || height == 0 {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("animation frame {index} has zero size"),
            )));
        }
        if x_offset < 0 || y_offset < 0 {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("animation frame {index} has negative offset"),
            )));
        }
        let expected_len = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                Box::new(ImgError::new_const(
                    ImgErrorKind::EncodeError,
                    format!("animation frame {index} buffer size overflows"),
                )) as Error
            })?;
        if buffer.len() != expected_len {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("animation frame {index} buffer size mismatch"),
            )));
        }

        let (delay_num, delay_den) = encode_delay(delay_ms);
        frames.push(ApngFrame {
            width,
            height,
            x_offset: x_offset as u32,
            y_offset: y_offset as u32,
            delay_num,
            delay_den,
            dispose_op,
            blend_op,
            buffer,
        });
    }

    let apng = ApngInfo { loop_count, frames };
    let needs_normalization = apng.frames.first().is_some_and(|frame| {
        frame.x_offset != 0
            || frame.y_offset != 0
            || frame.width != profile.width as u32
            || frame.height != profile.height as u32
    });

    if needs_normalization {
        Ok(Some(normalize_apng_frames(
            profile.width as u32,
            profile.height as u32,
            profile.background.clone(),
            apng,
        )?))
    } else {
        Ok(Some(apng))
    }
}

fn filtered_scanlines<F>(width: u32, height: u32, bpp: usize, mut row: F) -> Result<Vec<u8>, Error>
where
    F: FnMut(u32) -> Result<Vec<u8>, Error>,
{
    let row_bytes = (width as usize)
        .checked_mul(bpp)
        .ok_or("PNG row overflow")?;
    let mut prev_buf: Vec<u8> = Vec::new();
    let mut data = Vec::new();
    for y in 0..height {
        let buf = row(y)?;
        if buf.len() < row_bytes {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::EncodeError,
                format!("data shotage width {} but {}", width, buf.len()),
            )));
        }
        data.push(4);
        for i in 0..row_bytes {
            let left = if i >= bpp { buf[i - bpp] as i32 } else { 0 };
            let above = if y > 0 { prev_buf[i] as i32 } else { 0 };
            let upper_left = if y > 0 && i >= bpp {
                prev_buf[i - bpp] as i32
            } else {
                0
            };
            data.push(paeth_enc(buf[i], left, above, upper_left));
        }
        prev_buf = buf;
    }
    Ok(miniz_oxide::deflate::compress_to_vec_zlib(&data, 8))
}

fn encode_main_idat(
    image: &mut EncodeOptions<'_>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Error> {
    filtered_scanlines(width, height, 4, |y| {
        Ok(image
            .drawer
            .encode_pick(0, y as usize, width as usize, 1, None)?
            .unwrap_or(vec![0]))
    })
}

fn encode_frame_data(width: u32, height: u32, buffer: &[u8]) -> Result<Vec<u8>, Error> {
    filtered_scanlines(width, height, 4, |y| {
        let start = y as usize * width as usize * 4;
        let end = start + width as usize * 4;
        Ok(buffer[start..end].to_vec())
    })
}

fn write_ihdr(
    write_buffer: &mut Vec<u8>,
    crc32: &CRC32,
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
) {
    let mut temp_buffer: Vec<u8> = Vec::with_capacity(20);
    write_bytes(&IMAGE_HEADER, &mut temp_buffer);
    write_u32_be(width, &mut temp_buffer);
    write_u32_be(height, &mut temp_buffer);
    write_byte(depth, &mut temp_buffer);
    write_byte(color, &mut temp_buffer);
    write_byte(0, &mut temp_buffer);
    write_byte(0, &mut temp_buffer);
    write_byte(0, &mut temp_buffer);

    write_u32_be(13, write_buffer);
    write_bytes(&temp_buffer, write_buffer);
    let crc = crc32.crc32(&temp_buffer);
    write_u32_be(crc, write_buffer);
}

fn write_background(write_buffer: &mut Vec<u8>, crc32: &CRC32, background: crate::color::RGBA) {
    let red = background.red as u16;
    let green = background.green as u16;
    let blue = background.blue as u16;
    let mut temp_buffer: Vec<u8> = Vec::with_capacity(10);
    write_u16_be(red, &mut temp_buffer);
    write_u16_be(green, &mut temp_buffer);
    write_u16_be(blue, &mut temp_buffer);
    write_chunk(write_buffer, crc32, &BACKGROUND_COLOR, &temp_buffer);
}

fn write_actl(write_buffer: &mut Vec<u8>, crc32: &CRC32, frame_count: u32, loop_count: u32) {
    let mut temp_buffer = Vec::with_capacity(8);
    write_u32_be(frame_count, &mut temp_buffer);
    write_u32_be(loop_count, &mut temp_buffer);
    write_chunk(write_buffer, crc32, &ANIMATION_CONTROLE, &temp_buffer);
}

fn write_fctl(
    write_buffer: &mut Vec<u8>,
    crc32: &CRC32,
    sequence_number: u32,
    width: u32,
    height: u32,
    x_offset: u32,
    y_offset: u32,
    delay_num: u16,
    delay_den: u16,
    dispose_op: u8,
    blend_op: u8,
) {
    let mut temp_buffer = Vec::with_capacity(26);
    write_u32_be(sequence_number, &mut temp_buffer);
    write_u32_be(width, &mut temp_buffer);
    write_u32_be(height, &mut temp_buffer);
    write_u32_be(x_offset, &mut temp_buffer);
    write_u32_be(y_offset, &mut temp_buffer);
    write_u16_be(delay_num, &mut temp_buffer);
    write_u16_be(delay_den, &mut temp_buffer);
    write_byte(dispose_op, &mut temp_buffer);
    write_byte(blend_op, &mut temp_buffer);
    write_chunk(write_buffer, crc32, &FRAME_CONTROLE, &temp_buffer);
}

/// Encodes a still PNG or APNG stream.
///
/// Supported `EncodeOptions.options` keys:
/// - `exif`: `Raw(bytes)`, `Exif(headers)`, or `Ascii("copy")`
pub fn encode(image: &mut EncodeOptions<'_>) -> Result<Vec<u8>, Error> {
    let profile = image.drawer.encode_start(None)?;
    let (width, height, background, apng_info, exif) = if let Some(profile) = profile {
        let apng_info = parse_apng_info(&profile)?;
        let exif = get_exif_option(image.options.as_ref(), profile.metadata.as_ref())?;
        (
            profile.width as u32,
            profile.height as u32,
            profile.background,
            apng_info,
            exif,
        )
    } else {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::OutboundIndex,
            "Image profiles nothing".to_string(),
        )));
    };

    let crc32 = CRC32::new();
    let mut write_buffer: Vec<u8> = Vec::new();
    write_bytes(&SIGNATURE, &mut write_buffer);

    write_ihdr(&mut write_buffer, &crc32, width, height, 8, 6);
    if let Some(exif) = exif {
        write_chunk(&mut write_buffer, &crc32, &EXIF_PROFILE, &exif);
    }

    if let Some(background) = background {
        write_background(&mut write_buffer, &crc32, background);
    }

    if let Some(apng) = apng_info {
        write_actl(
            &mut write_buffer,
            &crc32,
            apng.frames.len() as u32,
            apng.loop_count,
        );

        let mut sequence_number = 0;
        let first_frame = &apng.frames[0];
        write_fctl(
            &mut write_buffer,
            &crc32,
            sequence_number,
            first_frame.width,
            first_frame.height,
            first_frame.x_offset,
            first_frame.y_offset,
            first_frame.delay_num,
            first_frame.delay_den,
            first_frame.dispose_op,
            first_frame.blend_op,
        );
        sequence_number += 1;

        let idat = encode_frame_data(first_frame.width, first_frame.height, &first_frame.buffer)?;
        write_chunk(&mut write_buffer, &crc32, &IMAGE_DATA, &idat);

        for frame in apng.frames.iter().skip(1) {
            if frame.x_offset + frame.width > width || frame.y_offset + frame.height > height {
                return Err(Box::new(ImgError::new_const(
                    ImgErrorKind::EncodeError,
                    "animation frame exceeds PNG canvas".to_string(),
                )));
            }
            write_fctl(
                &mut write_buffer,
                &crc32,
                sequence_number,
                frame.width,
                frame.height,
                frame.x_offset,
                frame.y_offset,
                frame.delay_num,
                frame.delay_den,
                frame.dispose_op,
                frame.blend_op,
            );
            sequence_number += 1;

            let fd_at = encode_frame_data(frame.width, frame.height, &frame.buffer)?;
            let mut temp_buffer = Vec::with_capacity(fd_at.len() + 4);
            write_u32_be(sequence_number, &mut temp_buffer);
            write_bytes(&fd_at, &mut temp_buffer);
            write_chunk(&mut write_buffer, &crc32, &FRAME_DATA, &temp_buffer);
            sequence_number += 1;
        }
    } else {
        let idat = encode_main_idat(image, width, height)?;
        write_chunk(&mut write_buffer, &crc32, &IMAGE_DATA, &idat);
    }

    write_chunk(&mut write_buffer, &crc32, &IMAGE_END, &[]);
    image.drawer.encode_end(None)?;
    Ok(write_buffer)
}

/// Encodes a static PNG from an explicit Gray8/16 or straight RGBA8/16 pick.
/// EX 16-bit bytes are little endian; PNG filtering uses big-endian bytes.
/// `ICC Profile` describes the output; `Source ICC Profile` is provenance only.
/// No color transformation, alpha compositing, or APNG export is performed.
#[cfg(feature = "image-buffer-ex")]
pub fn encode_ex(option: &mut crate::draw::EncodeOptionsEx<'_>) -> Result<Vec<u8>, Error> {
    use crate::draw::{PixelFormatEx, image_buffer_ex::validate_bytes};
    let format = option.output.format;
    if !matches!(
        format,
        PixelFormatEx::Gray8 | PixelFormatEx::Gray16 | PixelFormatEx::Rgba8 | PixelFormatEx::Rgba16
    ) {
        return Err("PNG EX output requires Gray/RGBA8 or16; explicitly request conversion from12-bit storage".into());
    }
    let profile = option.drawer.encode_start_ex(None)?;
    let result = (|| {
        let profile = profile.ok_or("PNG EX image profiles missing")?;
        let width = profile.legacy.width;
        let height = profile.legacy.height;
        // PNG dimensions are limited to 2^31-1, even on 64-bit platforms.
        if width > i32::MAX as usize || height > i32::MAX as usize {
            return Err("PNG dimensions exceed 2^31-1".into());
        }
        let (stride, length) = format.extent(width, height, option.output.stride)?;
        let limits = crate::limits::current();
        crate::limits::check(
            width.checked_mul(height).ok_or("PNG pixel overflow")?,
            limits.pixels,
            "PNG encode pixels",
        )?;
        crate::limits::check(1, limits.frames, "PNG encode frames")?;
        let row = format.row_bytes(width)?;
        let filtered = row
            .checked_add(1)
            .and_then(|n| n.checked_mul(height))
            .ok_or("PNG scanline overflow")?;
        // Account for picked pixels, filtered input, compressed output and chunk
        // assembly copies, plus two endian-conversion rows. Codec internals and
        // caller-owned storage are outside this adapter's workspace estimate.
        let workspace = filtered
            .checked_mul(4)
            .and_then(|n| n.checked_add(length))
            .and_then(|n| n.checked_add(row.checked_mul(2)?))
            .and_then(|n| n.checked_add(1024))
            .ok_or("PNG workspace overflow")?;
        crate::limits::check(workspace, limits.expanded_bytes, "PNG EX encode workspace")?;
        let mut metadata = profile.legacy.metadata.unwrap_or_default();
        if let Some(tags) = option.drawer.metadata_ex()? {
            metadata.extend(tags);
        }
        if metadata
            .get(ENCODE_ANIMATION_FRAMES_KEY)
            .is_some_and(|value| !matches!(value, DataMap::UInt(0)))
        {
            return Err(
                "PNG EX supports static images only; use the legacy APNG encoder for animation"
                    .into(),
            );
        }
        let icc_bytes = crate::draw::image_buffer_ex::output_icc_ex(
            &metadata,
            option.options.as_ref(),
            profile.format,
            format,
        )?;
        let exif_plan = crate::metadata::exif_output_ex(option.options.as_ref(), Some(&metadata))?;
        let exif_bound = exif_plan
            .as_ref()
            .map(|plan| plan.size_bound())
            .transpose()?
            .unwrap_or(0);
        let exif_workspace = exif_plan
            .as_ref()
            .map(|plan| plan.workspace_bound(exif_bound))
            .transpose()?
            .unwrap_or(0);
        let metadata_bytes = icc_bytes
            .map_or(0, <[u8]>::len)
            .checked_add(exif_bound)
            .ok_or("PNG metadata overflow")?;
        crate::limits::check(metadata_bytes, limits.metadata_bytes, "PNG encode metadata")?;
        crate::limits::check(
            workspace
                .checked_add(exif_workspace)
                .and_then(|n| n.checked_add(metadata_bytes.checked_mul(4)?))
                .ok_or("PNG workspace overflow")?,
            limits.expanded_bytes,
            "PNG EX encode workspace",
        )?;
        let exif = exif_plan
            .as_ref()
            .map(|plan| plan.materialize())
            .transpose()?;
        let picked = option
            .drawer
            .encode_pick_ex(0, 0, width, height, option.output, None)?
            .ok_or("PNG EX pixels missing")?;
        if picked.width() != width
            || picked.height() != height
            || picked.format() != format
            || picked.stride() != stride
        {
            return Err("PNG EX picker returned a different layout than requested".into());
        }
        validate_bytes(picked.bytes(), width, height, format, stride, length)?;
        let crc = CRC32::new();
        let mut output = SIGNATURE.to_vec();
        write_ihdr(
            &mut output,
            &crc,
            width as u32,
            height as u32,
            format.bits(),
            if format.channels() == 1 { 0 } else { 6 },
        );
        if let Some(bytes) = icc_bytes {
            let mut payload = b"Output ICC\0\0".to_vec();
            payload.extend(miniz_oxide::deflate::compress_to_vec_zlib(bytes, 8));
            if payload.len() > i32::MAX as usize {
                return Err("PNG ICC exceeds chunk size limit".into());
            }
            write_chunk(&mut output, &crc, b"iCCP", &payload);
        }
        if let Some(bytes) = exif {
            if bytes.len() > i32::MAX as usize {
                return Err("PNG EXIF exceeds chunk size limit".into());
            }
            write_chunk(&mut output, &crc, &EXIF_PROFILE, &bytes);
        }
        let idat =
            filtered_scanlines(width as u32, height as u32, format.bytes_per_pixel(), |y| {
                let start = y as usize * stride;
                let mut bytes = picked.bytes()[start..start + row].to_vec();
                if format.bits() == 16 {
                    for sample in bytes.chunks_exact_mut(2) {
                        sample.swap(0, 1);
                    }
                }
                Ok(bytes)
            })?;
        if idat.len() > i32::MAX as usize {
            return Err("PNG IDAT exceeds chunk size limit".into());
        }
        write_chunk(&mut output, &crc, &IMAGE_DATA, &idat);
        write_chunk(&mut output, &crc, &IMAGE_END, &[]);
        crate::limits::check(output.len(), limits.expanded_bytes, "PNG encoded output")?;
        Ok(output)
    })();
    let ended = option.drawer.encode_end_ex(None);
    match (result, ended) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(data), Ok(())) => Ok(data),
    }
}
