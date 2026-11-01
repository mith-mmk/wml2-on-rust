//! Buffered original-profile animation; exact rational timing, atomic publication.
use super::*;
#[non_exhaustive]
#[derive(Debug)]
pub struct JxlAnimationFrame {
    pub duration_ticks: u32,
    /// Coalesced full-canvas straight RGBA16, not a source-layer crop/blend.
    pub rgba16: Vec<u16>,
}
#[non_exhaustive]
#[derive(Debug)]
pub struct JxlAnimationImage {
    pub dimensions: crate::image_info::ImageDimensions,
    pub source_orientation: u8,
    pub bits_per_sample: u8,
    pub alpha_bits_per_sample: Option<u8>,
    pub ticks_per_second_numerator: u32,
    pub ticks_per_second_denominator: u32,
    /// Zero means indefinite repetition. Decoder does not sleep or replay.
    pub loop_count: u32,
    pub frames: Vec<JxlAnimationFrame>,
    pub source_icc: Vec<u8>,
    pub output_icc: Vec<u8>,
    pub source_header: Vec<u8>,
}
/// Decode every displayed frame privately, returning only after final validation.
/// Original-profile RGB8/16, optional matching straight full-resolution Alpha;
/// no preview, SMPTE, XYB or HDR. Exact raw ticks preserve fractional durations
/// and visible final-zero frames without inventing a millisecond display policy.
/// No poster duplicate, callbacks, playback, CMS or incremental publication.
pub fn animation_rgba16_from_with_resource_limits_and_cancel(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<JxlAnimationImage, Error> {
    animation_with_workspace(bytes, limits, policy, cancelled).map(|(image, _)| image)
}
fn animation_with_workspace(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<(JxlAnimationImage, resources::Workspace), Error> {
    let cancellation = Cancellation {
        probe: Some(cancelled),
        cancelled: std::cell::Cell::new(false),
    };
    let check = || cancellation.check();
    check()?;
    policy.validate()?;
    let limits = policy.tighten(limits);
    limits::check(
        bytes.len(),
        limits.input_bytes.min(MAX_INPUT),
        "JPEG XL animation input",
    )?;
    limits::check(
        mul(bytes.len(), 3)?,
        limits.expanded_bytes,
        "JPEG XL animation assembly workspace",
    )?;
    let input = unpack_with_resources(bytes, limits, Some(policy))?;
    check()?;
    let h = preflight_samples::<0, false, false, true>(&input, limits, policy)?;
    check()?;
    let plan = h
        .animation
        .as_ref()
        .ok_or_else(|| invalid("missing animation plan"))?;
    let pixels = mul(h.width, h.height)?;
    let frame_bytes = mul(pixels, 8)?;
    let n = plan.ticks.len();
    limits::check(
        mul(frame_bytes, n)?,
        limits.animation_bytes,
        "JPEG XL cumulative buffered animation output",
    )?;
    let descriptors = add(
        add(
            std::mem::size_of::<JxlAnimationImage>(),
            mul(n, std::mem::size_of::<JxlAnimationFrame>())?,
        )?,
        add(
            std::mem::size_of::<JxlPixelFormat>(),
            mul(
                usize::from(h.alpha_bits.is_some()),
                std::mem::size_of::<Option<JxlDataFormat>>(),
            )?,
        )?,
    )?;
    limits::check(
        add(plan.metadata, descriptors)?,
        limits.metadata_bytes,
        "JPEG XL animation descriptors",
    )?;
    let mut workspace = resources::Workspace::new(h.workspace, limits.expanded_bytes)?;
    workspace.claim(
        mul(frame_bytes, n)?,
        "JPEG XL all buffered animation canvases",
    )?;
    workspace.claim(descriptors, "JPEG XL animation descriptors")?;
    workspace.claim(64 * 1024, "JPEG XL animation profile temporary")?;
    let mut options = JxlDecoderOptions::default();
    options.sample_limit = Some(h.sample_limit);
    options.adjust_orientation = true;
    options.skip_preview = true;
    options.coalescing = true;
    options.alpha_mode = AlphaMode::AsIs;
    options.high_precision = true;
    options.render_spot_colors = false;
    options.max_profile_level = ProfileLevel::Main5;
    let mut runner = CancellableRunner(&cancellation);
    let mut decoder = JxlDecoder::new(options);
    decoder.close_input();
    let mut stream = input.codestream.as_slice();
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::BasicInfo,
    )?;
    check()?;
    let info = decoder
        .basic_info()
        .ok_or_else(|| invalid("missing animation basic info"))?;
    let a = info
        .animation
        .as_ref()
        .ok_or_else(|| invalid("missing codec animation timing"))?;
    if info.size != (h.width, h.height)
        || info.orientation as u8 != h.orientation
        || !info.uses_original_profile
        || !matches!(info.bit_depth,JxlBitDepth::Int{bits_per_sample}if bits_per_sample==u32::from(h.bits))
        || info.extra_channels.len() != usize::from(h.alpha_bits.is_some())
        || info
            .extra_channels
            .iter()
            .any(|c| c.ec_type != jxl_codec::api::ExtraChannel::Alpha || c.alpha_associated)
        || a.tps_numerator != plan.numerator
        || a.tps_denominator != plan.denominator
        || a.num_loops != plan.loops
        || a.have_timecodes
    {
        return Err(invalid("animation basic info disagrees with preflight"));
    }
    let mut formats = Vec::new();
    let ec = info.extra_channels.len();
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "animation formats",
        mul(ec, std::mem::size_of::<Option<JxlDataFormat>>())?,
    );
    formats.try_reserve_exact(ec)?;
    formats.resize(ec, None);
    decoder
        .set_pixel_format(JxlPixelFormat {
            color_type: JxlColorType::Rgba,
            color_data_format: Some(JxlDataFormat::U16 {
                endianness: Endianness::native(),
                bit_depth: 16,
            }),
            extra_channel_format: formats,
        })
        .map_err(|e| cancellation.codec_error(e))?;
    let mut metadata = add(plan.metadata, descriptors)?;
    let mut copy = |key: &str, data: &[u8]| -> Result<Vec<u8>, Error> {
        let next = add(metadata, data.len())?;
        limits::check(
            next,
            limits.metadata_bytes,
            "JPEG XL retained animation metadata",
        )?;
        workspace.claim(data.len(), "JPEG XL retained animation metadata")?;
        #[cfg(not(test))]
        let _ = key;
        #[cfg(test)]
        resources::tests::before_metadata_copy(key, data.len());
        let mut v = Vec::new();
        v.try_reserve_exact(data.len())?;
        v.extend_from_slice(data);
        metadata = next;
        check()?;
        Ok(v)
    };
    let source_header = copy(
        "animation source header",
        &input.codestream[..h.source_header_bytes],
    )?;
    let mut profile = |key: &str, p: Option<&JxlColorProfile>| -> Result<Vec<u8>, Error> {
        let p = p
            .ok_or_else(|| invalid("missing animation profile"))?
            .try_as_icc()
            .ok_or_else(|| unsupported("animation ICC unavailable"))?;
        validate_icc(&p, 3)?;
        copy(key, &p)
    };
    let source_icc = profile("animation source ICC", decoder.embedded_color_profile())?;
    let output_icc = profile("animation output ICC", decoder.output_color_profile())?;
    let mut frames = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "animation frame descriptors",
        mul(n, std::mem::size_of::<JxlAnimationFrame>())?,
    );
    frames.try_reserve_exact(n)?;
    for (index, &ticks) in plan.ticks.iter().enumerate() {
        check()?;
        expect_event(
            decoder
                .process(&mut stream, None, Some(&mut runner))
                .map_err(|e| cancellation.codec_error(e))?,
            Event::FrameHeader,
        )?;
        check()?;
        let scan = decoder
            .scanned_frames()
            .last()
            .ok_or_else(|| invalid("missing visible animation scan"))?;
        if scan.index != index || scan.duration_ticks != ticks || scan.is_last != (index + 1 == n) {
            return Err(invalid("animation visible timing disagrees with preflight"));
        }
        let mut rgba16 = Vec::new();
        #[cfg(test)]
        resources::tests::before_metadata_copy(
            match index {
                0 => "animation frame0",
                1 => "animation frame1",
                _ => "animation later frame",
            },
            frame_bytes,
        );
        rgba16.try_reserve_exact(mul(pixels, 4)?)?;
        rgba16.resize(mul(pixels, 4)?, 0u16);
        {
            // SAFETY: initialized exclusive native-endian u16 storage; checked
            // dimensions and byte extent cover exactly this owned full canvas.
            let data = unsafe {
                std::slice::from_raw_parts_mut(rgba16.as_mut_ptr().cast::<u8>(), frame_bytes)
            };
            let mut buffer = JxlOutputBuffer::new(data, h.height, mul(h.width, 8)?);
            expect_event(
                decoder
                    .process(
                        &mut stream,
                        Some(std::slice::from_mut(&mut buffer)),
                        Some(&mut runner),
                    )
                    .map_err(|e| cancellation.codec_error(e))?,
                Event::FrameComplete {
                    has_more_frames: index + 1 < n,
                },
            )?;
        }
        check()?;
        frames.push(JxlAnimationFrame {
            duration_ticks: ticks,
            rgba16,
        });
    }
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::Complete,
    )?;
    if decoder.file_length() != Some(input.codestream.len() as u64)
        || decoder.scanned_frames().len() != n
    {
        return Err(invalid("trailing/incomplete animation stream"));
    }
    check()?;
    let final_profile = decoder
        .output_color_profile()
        .ok_or_else(|| invalid("missing final animation profile"))?
        .try_as_icc()
        .ok_or_else(|| unsupported("final animation ICC unavailable"))?;
    if final_profile.as_ref() != &output_icc {
        return Err(invalid("animation output profile changed"));
    }
    drop(final_profile);
    check()?;
    Ok((
        JxlAnimationImage {
            dimensions: h.dimensions,
            source_orientation: h.orientation,
            bits_per_sample: h.bits,
            alpha_bits_per_sample: h.alpha_bits,
            ticks_per_second_numerator: plan.numerator,
            ticks_per_second_denominator: plan.denominator,
            loop_count: plan.loops,
            frames,
            source_icc,
            output_icc,
            source_header,
        },
        workspace,
    ))
}

// Common callback boundary: all codec/CMS work is completed before init/draw.
pub(super) fn deliver<const MANAGED: bool>(
    bytes: &[u8],
    option: &mut DecodeOptions,
    native: bool,
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancellation: &Cancellation<'_>,
    mut metadata: Vec<(String, DataMap)>,
    metadata_bytes: usize,
) -> Result<Option<ImgWarnings>, Error> {
    use crate::draw::{NextBlend, NextDispose, NextOption, NextOptions};
    let check = || cancellation.check();
    check()?;
    let limits = policy.tighten(limits);
    let (image, mut workspace) =
        animation_with_workspace(bytes, limits, policy, &|| cancellation.check().is_err())?;
    #[cfg(feature = "color-management")]
    let mut image = image;
    let size = image
        .dimensions
        .output
        .ok_or_else(|| invalid("missing animation geometry"))?;
    let (w, h) = (size.width as usize, size.height as usize);
    let p = mul(w, h)?;
    let n = image.frames.len();
    let source_bytes = mul(mul(p, 8)?, n)?;
    let delivered_frame = mul(p, if native { 8 } else { 4 })?;
    let delivered = mul(delivered_frame, n)?;
    let collector = mul(delivered_frame, add(n, 1)?)?; // poster + every displayed frame
    let conversion = if native { mul(p, 8)? } else { 0 }; // EX storage precision scratch
    limits::check(
        add(add(source_bytes, delivered)?, add(collector, conversion)?)?,
        limits.animation_bytes,
        "JPEG XL cumulative animation delivery",
    )?;
    workspace.claim(delivered, "JPEG XL prepared callback frames")?;
    workspace.claim(collector, "JPEG XL collector poster and frames")?;
    workspace.claim(conversion, "JPEG XL EX storage conversion scratch")?;
    workspace.claim(
        mul(n, std::mem::size_of::<Vec<u8>>())?,
        "JPEG XL prepared frame records",
    )?;
    workspace.claim(mul(add(n, 1)?, 128)?, "JPEG XL collector frame records")?;
    workspace.claim(
        add(bytes.len(), mul(metadata_bytes, 2)?)?,
        "JPEG XL reader and callback metadata",
    )?;
    let retained_metadata = add(
        metadata_bytes,
        add(
            add(image.source_icc.len(), image.output_icc.len())?,
            image.source_header.len(),
        )?,
    )?;
    let target_profile_bytes = if MANAGED {
        #[cfg(feature = "color-management")]
        {
            crate::color_convert::SRGB_ICC.len()
        }
        #[cfg(not(feature = "color-management"))]
        {
            image.output_icc.len()
        }
    } else {
        image.output_icc.len()
    };
    let callback_metadata = add(retained_metadata, add(target_profile_bytes, 4096)?)?;
    limits::check(
        callback_metadata,
        limits.metadata_bytes,
        "JPEG XL callback metadata",
    )?;
    workspace.claim(callback_metadata, "JPEG XL callback metadata copies")?;
    let mut delays = Vec::new();
    workspace.claim(mul(n, 8)?, "JPEG XL millisecond timing table")?;
    #[cfg(test)]
    resources::tests::before_metadata_copy("animation delays", mul(n, 8)?);
    delays.try_reserve_exact(n)?;
    for f in &image.frames {
        let ms =
            u128::from(f.duration_ticks) * u128::from(image.ticks_per_second_denominator) * 1000;
        let num = u128::from(image.ticks_per_second_numerator);
        if ms % num != 0 {
            return Err(unsupported(
                "animation callback requires exact integer milliseconds; use native rational-tick API",
            ));
        }
        delays.push(u64::try_from(ms / num)?);
    }
    #[cfg(feature = "color-management")]
    if MANAGED {
        let parse = resources::cms_profiles(&image.output_icc, crate::color_convert::SRGB_ICC)?;
        let scratch = 2 * 256 * 3 * std::mem::size_of::<f32>() + 256 * std::mem::size_of::<usize>();
        workspace.claim(parse, "JPEG XL animation CMS profiles")?;
        workspace.claim(scratch, "JPEG XL animation CMS batches")?;
        for f in &mut image.frames {
            check()?;
            crate::color_convert::rgba16_to_rgb_profile_with_cancel(
                &mut f.rgba16,
                &image.output_icc,
                crate::color_convert::SRGB_ICC,
                false,
                workspace.remaining(),
                &check,
            )?;
        }
        workspace.release(parse)?;
        workspace.release(scratch)?;
    }
    #[cfg(not(feature = "color-management"))]
    if MANAGED {
        return Err(unsupported("animation sRGB requires color-management"));
    }
    let mut prepared = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "animation prepared records",
        mul(n, std::mem::size_of::<Vec<u8>>())?,
    );
    prepared.try_reserve_exact(n)?;
    for (_index, f) in image.frames.iter().enumerate() {
        check()?;
        let mut out = Vec::new();
        #[cfg(test)]
        resources::tests::before_metadata_copy(
            match _index {
                0 => "animation prepared pixels",
                1 => "animation prepared second pixels",
                _ => "animation prepared later pixels",
            },
            delivered_frame,
        );
        out.try_reserve_exact(delivered_frame)?;
        for chunk in f.rgba16.chunks(256 * 4) {
            check()?;
            for &v in chunk {
                if native {
                    out.extend_from_slice(&v.to_le_bytes());
                } else {
                    out.push(((u32::from(v) * 255 + 32767) / 65535) as u8);
                }
            }
        }
        prepared.push(out);
    }
    // Reserve keys/profile copies before publication. Owned source buffers move;
    // only the output ICC is duplicated to preserve both provenance and tagging.
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "animation callback metadata",
        mul(16, std::mem::size_of::<(String, DataMap)>())?,
    );
    metadata.try_reserve_exact(16)?;
    let actual_profile: &[u8] = if MANAGED {
        #[cfg(feature = "color-management")]
        {
            crate::color_convert::SRGB_ICC
        }
        #[cfg(not(feature = "color-management"))]
        {
            &image.output_icc
        }
    } else {
        &image.output_icc
    };
    let mut icc = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy("animation callback ICC", actual_profile.len());
    icc.try_reserve_exact(actual_profile.len())?;
    icc.extend_from_slice(actual_profile);
    metadata.push(("ICC Profile".into(), DataMap::ICCProfile(icc)));
    metadata.push((
        "JPEG XL Source ICC Profile".into(),
        DataMap::ICCProfile(image.source_icc),
    ));
    metadata.push((
        "JPEG XL Reconstruction ICC Profile".into(),
        DataMap::ICCProfile(image.output_icc),
    ));
    metadata.push((
        "JPEG XL Source Header".into(),
        DataMap::Raw(image.source_header),
    ));
    metadata.push((
        "JPEG XL Source Orientation".into(),
        DataMap::UInt(image.source_orientation.into()),
    ));
    metadata.push((
        "JPEG XL Source Bits Per Sample".into(),
        DataMap::UInt(image.bits_per_sample.into()),
    ));
    metadata.push((
        "JPEG XL Animation Ticks Numerator".into(),
        DataMap::UInt(image.ticks_per_second_numerator.into()),
    ));
    metadata.push((
        "JPEG XL Animation Ticks Denominator".into(),
        DataMap::UInt(image.ticks_per_second_denominator.into()),
    ));
    for (key, value) in [
        ("coded_width", image.dimensions.coded.width),
        ("coded_height", image.dimensions.coded.height),
        ("output_width", size.width),
        ("output_height", size.height),
    ] {
        metadata.push((key.into(), DataMap::UInt(value.into())));
    }
    if let Some(bits) = image.alpha_bits_per_sample {
        metadata.push((
            "JPEG XL Source Alpha Bits Per Sample".into(),
            DataMap::UInt(bits.into()),
        ));
    }
    metadata.push(("orientation_applied".into(), DataMap::Bool(true)));
    let meta = metadata.iter().try_fold(0usize, |a, (k, v)| {
        add(
            a,
            add(
                k.len(),
                match v {
                    DataMap::Raw(v) | DataMap::ICCProfile(v) => v.len(),
                    DataMap::Ascii(v) => v.len(),
                    _ => 8,
                },
            )?,
        )
    })?;
    limits::check(
        meta,
        limits.metadata_bytes,
        "JPEG XL callback retained metadata",
    )?;
    let live = limits.expanded_bytes - workspace.remaining();
    let augmented = structured_metadata::augment(&mut metadata, limits, meta, live)?;
    workspace.claim(augmented - live, "JPEG XL structured animation metadata")?;
    check()?;
    let result = (|| {
        let init = InitOptions {
            loop_count: image.loop_count,
            background: None,
            animation: true,
        };
        #[cfg(feature = "image-buffer-ex")]
        if native {
            option.drawer.init_ex(
                w,
                h,
                Some(crate::draw::InitOptionsEx {
                    source_format: crate::draw::PixelFormatEx::Rgba16,
                    source_stride: None,
                    legacy: Some(init),
                }),
            )?;
        } else {
            option.drawer.init(w, h, Some(init))?;
        }
        #[cfg(not(feature = "image-buffer-ex"))]
        option.drawer.init(w, h, Some(init))?;
        check()?;
        for (key, value) in metadata {
            option.drawer.set_metadata(&key, value)?;
            check()?;
        }
        // Poster is the first coalesced frame; it has no extra playback delay.
        option.drawer.draw(0, 0, w, h, &prepared[0], None)?;
        check()?;
        for (index, data) in prepared.iter().enumerate() {
            option.drawer.next(Some(NextOptions {
                flag: if index == 0 {
                    NextOption::Continue
                } else {
                    NextOption::Next
                },
                await_time: delays[index],
                image_rect: None,
                dispose_option: Some(NextDispose::None),
                blend: Some(NextBlend::Override),
            }))?;
            check()?;
            option.drawer.draw(0, 0, w, h, data, None)?;
            check()?;
        }
        option.drawer.terminate(None)?;
        check()?;
        Ok(None)
    })();
    if result.is_err() {
        let _ = option.drawer.terminate(None);
    }
    result
}
