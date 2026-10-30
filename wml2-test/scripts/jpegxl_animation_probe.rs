//! Trusted-small-fixture feasibility probe, never a production admission API.
#[path = "jpegxl_animation_core.rs"]
mod core;
use jxl_codec::bit_reader::BitReader;
use jxl_codec::headers::encodings::UnconditionalCoder;
use jxl_codec::headers::frame_header::FrameHeader;
use jxl_codec::headers::toc::IncrementalTocReader;
use jxl_codec::headers::{FileHeader, JxlHeader};
type Error = Box<dyn std::error::Error>;
fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    let data = std::fs::read(&args[1])?;
    if data.len() > 1 << 20 {
        return Err("trusted fixture cap".into());
    }
    let data = core::codestream(&data)?;
    let mut br = BitReader::new(&data);
    let header = FileHeader::read(&mut br)?;
    let m = &header.image_metadata;
    let a = m.animation.as_ref().ok_or("animation required")?;
    println!(
        "header {} {} {} {} {} {} {} {} {}",
        br.total_bits_read(),
        header.size.xsize(),
        header.size.ysize(),
        m.orientation as u8,
        a.tps_numerator,
        a.tps_denominator,
        a.num_loops,
        a.have_timecodes,
        m.preview.is_some()
    );
    if header.size.xsize() > 64 || header.size.ysize() > 64 || m.extra_channel_info.len() > 1 {
        return Err("small fixture only".into());
    }
    if m.color_encoding.want_icc {
        let mut r = jxl_codec::icc::IncrementalIccReader::new(&mut br)?;
        r.read_all(&mut br)?;
        let _ = r.finalize(&mut br)?;
    }
    br.jump_to_byte_boundary()?;
    let mut at = br.total_bits_read() / 8;
    let mut preview = m.preview.is_some();
    let mut finished = false;
    for index in 0..64 {
        let mut br = BitReader::new(data.get(at..).ok_or("frame offset")?);
        let non = if preview {
            header
                .preview_frame_header_nonserialized()
                .ok_or("preview")?
        } else {
            header.frame_header_nonserialized()
        };
        let mut f = FrameHeader::read_unconditional(&(), &mut br, &non)?;
        f.postprocess(&non);
        println!(
            "coded {index} {at} {} {} {} {} {} {} {} {} {} {} {} {}",
            preview,
            f.is_visible(),
            f.duration,
            f.is_last,
            f.x0,
            f.y0,
            f.size_upsampled().0,
            f.size_upsampled().1,
            f.save_as_reference,
            f.name.len(),
            f.blending_info.mode as u8,
            f.blending_info.source
        );
        let mut toc = IncrementalTocReader::new(f.num_toc_entries().try_into()?, &mut br)?;
        while !toc.is_complete() {
            toc.read_step(&mut br)?;
        }
        let toc = toc.finalize();
        br.jump_to_byte_boundary()?;
        let bytes = toc.entries.iter().try_fold(0usize, |n, v| {
            n.checked_add(*v as usize).ok_or("payload overflow")
        })?;
        at = at
            .checked_add(br.total_bits_read() / 8)
            .and_then(|v| v.checked_add(bytes))
            .ok_or("offset overflow")?;
        if at > data.len() {
            return Err("truncated frame".into());
        }
        if f.is_last {
            if preview {
                preview = false;
            } else {
                finished = true;
                break;
            }
        }
    }
    if !finished || at != data.len() {
        return Err("unbounded/trailing sequence".into());
    }
    let decoded = core::decode(&data)?;
    println!(
        "animation {} {} {}",
        decoded.numerator, decoded.denominator, decoded.loops
    );
    for (index, frame) in decoded.frames.iter().enumerate() {
        println!(
            "visible {index} {} {} {} {} {}",
            frame.milliseconds, frame.ticks, frame.last, decoded.size.0, decoded.size.1
        );
        std::fs::write(format!("{}.{}.rgba16le", args[2], index), &frame.pixels)?;
    }
    std::fs::write(format!("{}.icc", args[2]), &decoded.profile)?;
    if args.len() > 3 {
        use icc_profile::{ParseLimits, Transform, TransformLimits, TransformOptions};
        let transform = Transform::from_bytes_with_limits(
            &decoded.profile,
            &std::fs::read(&args[3])?,
            TransformOptions::default(),
            ParseLimits::default(),
            TransformLimits::default(),
        )?;
        if transform.input_channels() != 3 {
            return Err("RGB fixture profile required".into());
        }
        for (i, frame) in decoded.frames.iter().enumerate() {
            let words: Vec<u16> = frame
                .pixels
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            let rgb: Vec<f32> = words
                .chunks_exact(4)
                .flat_map(|p| p[..3].iter().map(|&v| f32::from(v) / 65535.0))
                .collect();
            let mut output = vec![0.0; rgb.len()];
            transform.transform_f32(&rgb, &mut output)?;
            let mut rgba = Vec::new();
            for (rgb, p) in output.chunks_exact(3).zip(words.chunks_exact(4)) {
                for &v in rgb {
                    let n = (v.clamp(0.0, 1.0) * 65535.0).round() as u32;
                    rgba.push(((n * 255 + 32767) / 65535) as u8);
                }
                rgba.push(((u32::from(p[3]) * 255 + 32767) / 65535) as u8);
            }
            std::fs::write(format!("{}.{}.srgb.rgba", args[2], i), rgba)?;
        }
    }

    println!("frames {}", decoded.frames.len());
    Ok(())
}
