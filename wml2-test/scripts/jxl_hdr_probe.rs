//! Trusted-fixture core probe, separate from the bounded production adapter.
use jxl_codec::api::*;
use std::error::Error;

fn complete<T, U>(p: ProcessingResult<T, U>) -> T {
    match p {
        ProcessingResult::Complete { result } => result,
        _ => panic!("incomplete fixture"),
    }
}
fn profile(p: &JxlColorProfile, path: &str) -> String {
    match p {
        JxlColorProfile::Icc(bytes) => {
            std::fs::write(path, bytes).unwrap();
            format!("ICC {} bytes", bytes.len())
        }
        JxlColorProfile::Simple(enc) => format!("{enc:?}"),
    }
}
fn quoted(s: &str) -> String {
    format!("{s:?}")
}
fn contracts(profile_path: &str) -> Result<(), Box<dyn Error>> {
    use wml2::highres::*;
    let layout = PlaneLayout::interleaved(1, 1, 4, Subsampling::FULL)?;
    let descriptor = ImageDescriptor::new(
        1,
        1,
        ChannelModel::RGB,
        vec![PlaneDescriptor::new(
            layout.clone(),
            vec![
                ChannelRole::Red,
                ChannelRole::Green,
                ChannelRole::Blue,
                ChannelRole::Alpha,
            ],
            32,
        )?],
    )?
    .with_alpha(AlphaAssociation::Straight)?;
    for (name, values, valid) in [
        ("extended", [-0.25, 0.5, 1.5, 0.25], true),
        ("nan", [f32::NAN, 0.5, 1.5, 0.25], false),
        ("positive-infinity", [f32::INFINITY, 0.5, 1.5, 0.25], false),
        (
            "negative-infinity",
            [f32::NEG_INFINITY, 0.5, 1.5, 0.25],
            false,
        ),
        ("alpha-negative", [0.0, 0.5, 1.0, -0.1], false),
        ("alpha-high", [0.0, 0.5, 1.0, 1.1], false),
    ] {
        let result = ImageFrame::new(
            descriptor.clone(),
            PixelBuffer::f32(vec![Plane::new(layout.clone(), values.to_vec())?])?,
        );
        assert_eq!(result.is_ok(), valid, "{name}");
        println!(
            "storage {name} {}",
            if result.is_ok() {
                "accepted"
            } else {
                "rejected"
            }
        );
    }
    let frame = ImageFrame::new(
        descriptor,
        PixelBuffer::f32(vec![Plane::new(layout, vec![-0.25, 0.5, 1.5, 0.25])?])?,
    )?;
    let profile = std::fs::read(profile_path)?;
    let transformed = icc::transform_frame(&frame, &profile, &profile)?;
    let values = transformed.pixels().f32_planes().unwrap()[0].samples();
    println!("default_cms {values:?}");
    assert_eq!(values[3], 0.25);
    assert_eq!(values[0], 0.0);
    assert_eq!(values[2], 1.0);
    let strict = icc::transform_frame_with_options(
        &frame,
        &profile,
        &profile,
        icc::TransformOptions {
            clamp: false,
            ..Default::default()
        },
    );
    assert!(strict.is_err());
    println!("unclamped_cms {}", strict.unwrap_err());
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a[1] == "contracts" {
        return contracts(&a[2]);
    }
    let raw = std::fs::read(&a[1])?;
    assert!(raw.len() <= 1024 * 1024);
    let mut opt = JxlDecoderOptions::default();
    opt.high_precision = true;
    opt.render_spot_colors = false;
    opt.sample_limit = Some(4096);
    if a[4] != "0" {
        opt.desired_intensity_target = Some(a[4].parse()?);
    }
    let mut stream = raw.as_slice();
    let mut dec = complete(JxlDecoder::<states::Initialized>::new(opt).process(&mut stream, None)?);
    let (w, h) = dec.basic_info().size;
    assert!(w <= 32 && h <= 32);
    let channels = dec.embedded_color_profile().channels()
        + usize::from(!dec.basic_info().extra_channels.is_empty());
    let kind = match channels {
        1 => JxlColorType::Grayscale,
        2 => JxlColorType::GrayscaleAlpha,
        3 => JxlColorType::Rgb,
        4 => JxlColorType::Rgba,
        _ => panic!("channels"),
    };
    let (format, bps) = match a[3].as_str() {
        "f32" => (
            JxlDataFormat::F32 {
                endianness: Endianness::LittleEndian,
            },
            4,
        ),
        "f16" => (
            JxlDataFormat::F16 {
                endianness: Endianness::LittleEndian,
            },
            2,
        ),
        _ => (
            JxlDataFormat::U16 {
                endianness: Endianness::LittleEndian,
                bit_depth: 16,
            },
            2,
        ),
    };
    dec.set_pixel_format(JxlPixelFormat {
        color_type: kind,
        color_data_format: Some(format),
        extra_channel_format: vec![None; dec.basic_info().extra_channels.len()],
    })?;
    let src = profile(
        dec.embedded_color_profile(),
        &(a[2].clone() + ".source.icc"),
    );
    let dst = profile(dec.output_color_profile(), &(a[2].clone() + ".output.icc"));
    let dec = complete(dec.process(&mut stream, None)?);
    let row = w * channels * bps;
    let bytes = row * h;
    let mut aligned = vec![0u32; bytes.div_ceil(4)];
    // u32 guarantees alignment for every tested output representation.
    let output =
        unsafe { std::slice::from_raw_parts_mut(aligned.as_mut_ptr().cast::<u8>(), bytes) };
    let mut buffers = [JxlOutputBuffer::new(output, h, row)];
    let dec = complete(dec.process(&mut stream, &mut buffers, None)?);
    assert!(!dec.has_more_frames());
    std::fs::write(&a[2], output)?;
    let legacy = wml2::draw::image_from(&raw)
        .map(|_| "accepted".to_string())
        .unwrap_or_else(|e| e.to_string());
    let ex = wml2::draw::image_from_ex(
        &raw,
        wml2::draw::PixelFormatEx::Rgba16,
        wml2::draw::PrecisionConversion::Round,
    )
    .map(|_| "accepted".to_string())
    .unwrap_or_else(|e| e.to_string());
    let display = wml2::jpegxl::image_from_srgb(&raw)
        .map(|_| "accepted".to_string())
        .unwrap_or_else(|e| e.to_string());
    println!(
        "{{\"width\":{w},\"height\":{h},\"channels\":{channels},\"bytes\":{bytes},\"source_profile\":{},\"output_profile\":{},\"legacy\":{},\"ex\":{},\"display\":{}}}",
        quoted(&src),
        quoted(&dst),
        quoted(&legacy),
        quoted(&ex),
        quoted(&display)
    );
    Ok(())
}
