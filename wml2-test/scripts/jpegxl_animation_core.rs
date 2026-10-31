//! Trusted-fixture core experiment only. No untrusted-data workspace contract.
use jxl_codec::api::*;
type Error = Box<dyn std::error::Error>;
pub struct Frame {
    pub ticks: u32,
    pub milliseconds: f64,
    pub last: bool,
    pub pixels: Vec<u8>,
}
pub struct Decoded {
    pub size: (usize, usize),
    pub numerator: u32,
    pub denominator: u32,
    pub loops: u32,
    pub frames: Vec<Frame>,
    pub profile: Vec<u8>,
}
fn complete<T, U>(v: ProcessingResult<T, U>) -> Result<T, Error> {
    match v {
        ProcessingResult::Complete { result } => Ok(result),
        _ => Err("truncated core input".into()),
    }
}
pub fn codestream(data: &[u8]) -> Result<Vec<u8>, Error> {
    if data.len() > 1 << 20 {
        return Err("trusted fixture cap".into());
    }
    if data.starts_with(&[255, 10]) {
        return Ok(data.to_vec());
    }
    if !data.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n") {
        return Err("signature".into());
    }
    let mut at = 12usize;
    let mut out = Vec::new();
    while at < data.len() {
        let n = u32::from_be_bytes(data.get(at..at + 4).ok_or("box")?.try_into()?) as usize;
        let end = at.checked_add(n).ok_or("box overflow")?;
        if n < 8 || end > data.len() {
            return Err("box bounds".into());
        }
        let k = &data[at + 4..at + 8];
        if k == b"jxlc" {
            out.extend(&data[at + 8..end]);
        } else if k == b"jxlp" {
            out.extend(data.get(at + 12..end).ok_or("jxlp")?);
        }
        at = end;
    }
    Ok(out)
}
pub fn decode(data: &[u8]) -> Result<Decoded, Error> {
    let data = codestream(data)?;
    let mut options = JxlDecoderOptions::default();
    options.high_precision = true;
    options.sample_limit = Some(64 * 64 * 4 + 1);
    options.render_spot_colors = false;
    options.coalescing = true;
    options.skip_preview = true;
    let mut input = data.as_slice();
    let mut decoder =
        complete(JxlDecoder::<states::Initialized>::new(options).process(&mut input, None)?)?;
    let (w, h) = decoder.basic_info().size;
    if w > 64 || h > 64 {
        return Err("small trusted canvas only".into());
    }
    let animation = decoder
        .basic_info()
        .animation
        .as_ref()
        .ok_or("animation required")?
        .clone();
    let mut format = JxlPixelFormat::rgba16(decoder.basic_info().extra_channels.len());
    format.color_data_format = Some(JxlDataFormat::U16 {
        endianness: Endianness::LittleEndian,
        bit_depth: 16,
    });
    decoder.set_pixel_format(format)?;
    let profile = match decoder.output_color_profile() {
        JxlColorProfile::Icc(b) => b.clone(),
        JxlColorProfile::Simple(e) => e.maybe_create_profile()?.ok_or("no ICC")?,
    };
    let mut frames = Vec::new();
    loop {
        let frame = complete(decoder.process(&mut input, None)?)?;
        let info = frame.frame_header();
        let scan = frame
            .scanned_frames()
            .last()
            .ok_or("visible scan missing")?;
        if info.size != (w, h) {
            return Err("coalesced canvas changed".into());
        }
        let ticks = scan.duration_ticks;
        let last = scan.is_last;
        let milliseconds = info.duration.ok_or("missing duration")?;
        if milliseconds != scan.duration_ms {
            return Err("public timing disagreement".into());
        }
        let mut pixels = vec![
            0u16;
            w.checked_mul(h)
                .and_then(|p| p.checked_mul(4))
                .ok_or("canvas overflow")?
        ];
        // SAFETY: initialized aligned u16 storage; explicit little-endian output.
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast::<u8>(), pixels.len() * 2)
        };
        let mut buffers = [JxlOutputBuffer::new(bytes, h, w * 8)];
        decoder = complete(frame.process(&mut input, &mut buffers, None)?)?;
        frames.push(Frame {
            ticks,
            milliseconds,
            last,
            pixels: bytes.to_vec(),
        });
        if frames.len() > 16 {
            return Err("displayed fixture cap".into());
        }
        if !decoder.has_more_frames() {
            break;
        }
    }
    if decoder.file_length() != Some(data.len() as u64) {
        return Err("core boundary".into());
    }
    Ok(Decoded {
        size: (w, h),
        numerator: animation.tps_numerator,
        denominator: animation.tps_denominator,
        loops: animation.num_loops,
        frames,
        profile,
    })
}
