//! Explicit color plus separately retained extra-plane output.
use super::*;
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlExtraChannelKind {
    Depth,
}
/// Decoded integer sample codes at source depth; no distance-unit interpretation.
/// Source and output geometry are separate; output samples are already oriented.
#[non_exhaustive]
#[derive(Debug)]
pub struct JxlExtraPlane {
    pub source_index: usize,
    pub kind: JxlExtraChannelKind,
    pub bits_per_sample: u8,
    pub dim_shift: u8,
    pub source_dimensions: crate::image_info::ImageSize,
    pub dimensions: crate::image_info::ImageSize,
    pub samples: Vec<u16>,
}
/// Original-profile normalized RGBA16 color plus ALL admitted extra planes.
/// Gray coordinates are replicated, identified by color_channels=1. Alpha is
/// opaque because this initial route refuses Alpha channels. Extras never pass
/// through ICC/CMS or get blended into color. No display conversion is implied.
#[non_exhaustive]
#[derive(Debug)]
pub struct JxlExtraImage {
    pub dimensions: crate::image_info::ImageDimensions,
    pub source_orientation: u8,
    pub color_channels: u8,
    pub color_bits_per_sample: u8,
    pub rgba16: Vec<u16>,
    pub extra_planes: Vec<JxlExtraPlane>,
    pub source_icc: Vec<u8>,
    pub output_icc: Vec<u8>,
    /// Bounded exact codestream header, including channel-name bytes. The pinned
    /// public descriptors lack a name accessor; absence of a name API does not
    /// mean the source name is empty. Container boxes are not stored here.
    pub source_header: Vec<u8>,
}
/// Zero or one full-resolution integer Depth plane, original-profile Gray/RGB
/// SDR, one unresampled full-canvas Modular frame, no preview. Other/multiple
/// extras (including Alpha) reject rather than being discarded. Output is
/// published only after complete input and final profile/boundary checks.
pub fn image_with_extra_channels_from_with_resource_limits_and_cancel(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<JxlExtraImage, Error> {
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
        "JPEG XL input",
    )?;
    limits::check(
        mul(bytes.len(), 3)?,
        limits.expanded_bytes,
        "JPEG XL input assembly workspace",
    )?;
    let input = unpack_with_resources(bytes, limits, Some(policy))?;
    check()?;
    let h = preflight_samples::<0, false, true>(&input, limits, policy)?;
    check()?;
    let count = usize::from(h.depth_bits.is_some());
    let pixels = mul(h.width, h.height)?;
    let color_bytes = mul(pixels, 8)?;
    let depth_bytes = mul(mul(pixels, 2)?, count)?;
    limits::check(
        add(color_bytes, depth_bytes)?,
        limits.animation_bytes,
        "JPEG XL cumulative color/extra output",
    )?;
    let mut workspace = resources::Workspace::new(h.workspace, limits.expanded_bytes)?;
    workspace.claim(
        add(color_bytes, depth_bytes)?,
        "JPEG XL color and extra buffers",
    )?;
    let descriptors = add(
        add(
            std::mem::size_of::<JxlExtraImage>(),
            std::mem::size_of::<JxlPixelFormat>(),
        )?,
        add(
            mul(
                count,
                add(
                    std::mem::size_of::<JxlExtraPlane>(),
                    std::mem::size_of::<Option<JxlDataFormat>>(),
                )?,
            )?,
            add(
                std::mem::size_of::<Vec<JxlOutputBuffer>>(),
                mul(1 + count, std::mem::size_of::<JxlOutputBuffer>())?,
            )?,
        )?,
    )?;
    workspace.claim(descriptors, "JPEG XL extra descriptors")?;
    workspace.claim(64 * 1024, "JPEG XL generated profile temporary")?;
    let mut options = JxlDecoderOptions::default();
    options.sample_limit = Some(h.sample_limit);
    options.adjust_orientation = true;
    options.skip_preview = true;
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
        .ok_or_else(|| invalid("missing extra-plane basic info"))?;
    if info.size != (h.width, h.height)
        || info.orientation as u8 != h.orientation
        || !info.uses_original_profile
        || info.animation.is_some()
        || !matches!(info.bit_depth,JxlBitDepth::Int{bits_per_sample}if bits_per_sample==u32::from(h.bits))
        || info.extra_channels.len() != count
        || info
            .extra_channels
            .iter()
            .any(|c| c.ec_type != jxl_codec::api::ExtraChannel::Depth || c.alpha_associated)
    {
        return Err(invalid("extra-plane basic info disagrees with preflight"));
    }
    let mut formats = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "extra formats",
        count * std::mem::size_of::<Option<JxlDataFormat>>(),
    );
    formats.try_reserve_exact(count)?;
    if let Some(bits) = h.depth_bits {
        formats.push(Some(JxlDataFormat::U16 {
            endianness: Endianness::native(),
            bit_depth: bits.into(),
        }));
    }
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
    let mut metadata = add(input.metadata_bytes, descriptors)?;
    let mut copy = |key: &str, data: &[u8]| -> Result<Vec<u8>, Error> {
        let next = add(metadata, data.len())?;
        limits::check(
            next,
            limits.metadata_bytes,
            "JPEG XL retained extra metadata",
        )?;
        workspace.claim(data.len(), "JPEG XL retained extra metadata")?;
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
        "extra source header",
        &input.codestream[..h.source_header_bytes],
    )?;
    let mut profile = |key: &str, p: Option<&JxlColorProfile>| -> Result<Vec<u8>, Error> {
        let p = p
            .ok_or_else(|| invalid("missing extra-plane profile"))?
            .try_as_icc()
            .ok_or_else(|| unsupported("extra-plane ICC unavailable"))?;
        validate_icc(&p, h.color_channels)?;
        copy(key, &p)
    };
    let source_icc = profile("extra source ICC", decoder.embedded_color_profile())?;
    let output_icc = profile("extra output ICC", decoder.output_color_profile())?;
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::FrameHeader,
    )?;
    check()?;
    let mut rgba16 = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy("extra color buffer", color_bytes);
    rgba16.try_reserve_exact(mul(pixels, 4)?)?;
    rgba16.resize(mul(pixels, 4)?, 0u16);
    let mut depth = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy("extra Depth buffer", depth_bytes);
    depth.try_reserve_exact(mul(pixels, count)?)?;
    depth.resize(mul(pixels, count)?, 0u16);
    {
        // SAFETY: initialized, exclusive, disjoint native-endian u16 vectors;
        // byte views cover exactly the pre-admitted logical row storage.
        let rgb = unsafe {
            std::slice::from_raw_parts_mut(rgba16.as_mut_ptr().cast::<u8>(), color_bytes)
        };
        let mut buffers = Vec::new();
        #[cfg(test)]
        resources::tests::before_metadata_copy(
            "extra output buffers",
            (1 + count) * std::mem::size_of::<JxlOutputBuffer>(),
        );
        buffers.try_reserve_exact(1 + count)?;
        buffers.push(JxlOutputBuffer::new(rgb, h.height, mul(h.width, 8)?));
        if count != 0 {
            let extra = unsafe {
                std::slice::from_raw_parts_mut(depth.as_mut_ptr().cast::<u8>(), depth_bytes)
            };
            buffers.push(JxlOutputBuffer::new(extra, h.height, mul(h.width, 2)?));
        }
        expect_event(
            decoder
                .process(&mut stream, Some(&mut buffers), Some(&mut runner))
                .map_err(|e| cancellation.codec_error(e))?,
            Event::FrameComplete {
                has_more_frames: false,
            },
        )?;
    }
    check()?;
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::Complete,
    )?;
    if decoder.file_length() != Some(input.codestream.len() as u64) {
        return Err(invalid("trailing/incomplete extra-plane stream"));
    }
    check()?;
    let final_profile = decoder
        .output_color_profile()
        .ok_or_else(|| invalid("missing final extra profile"))?
        .try_as_icc()
        .ok_or_else(|| unsupported("final extra ICC unavailable"))?;
    if final_profile.as_ref() != &output_icc {
        return Err(invalid("extra-plane output profile changed"));
    }
    drop(final_profile);
    for chunk in rgba16.chunks(256 * 4) {
        check()?;
        if chunk.chunks_exact(4).any(|p| p[3] != 65535) {
            return Err(invalid("nonopaque color without source alpha"));
        }
    }
    if let Some(bits) = h.depth_bits {
        let max = (1u32 << bits) - 1;
        for chunk in depth.chunks(256) {
            check()?;
            if chunk.iter().any(|&v| u32::from(v) > max) {
                return Err(invalid("extra source code out of range"));
            }
        }
    }
    let mut extra_planes = Vec::new();
    #[cfg(test)]
    resources::tests::before_metadata_copy(
        "extra plane descriptors",
        count * std::mem::size_of::<JxlExtraPlane>(),
    );
    extra_planes.try_reserve_exact(count)?;
    if let Some(bits) = h.depth_bits {
        extra_planes.push(JxlExtraPlane {
            source_index: 0,
            kind: JxlExtraChannelKind::Depth,
            bits_per_sample: bits,
            dim_shift: 0,
            source_dimensions: h.dimensions.coded,
            dimensions: h.dimensions.output.unwrap(),
            samples: depth,
        });
    }
    check()?;
    Ok(JxlExtraImage {
        dimensions: h.dimensions,
        source_orientation: h.orientation,
        color_channels: h.color_channels as u8,
        color_bits_per_sample: h.bits,
        rgba16,
        extra_planes,
        source_icc,
        output_icc,
        source_header,
    })
}
