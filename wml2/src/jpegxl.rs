//! Bounded JPEG XL still-image adapter for the published `jxl 0.7.4` API.
//!
//! Enable `jpeg-xl`; add `image-buffer-ex` for native Gray/RGBA8/12/16LE.
//! See `docs/jpeg-xl.md` for precision, colour, resource and rejection contracts.
use crate::draw::{DecodeOptions, InitOptions};
use crate::error::{ImgError, ImgErrorKind};
use crate::limits::{self, DecodeLimits};
use crate::metadata::DataMap;
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
use jxl_codec::api::*;
use jxl_codec::bit_reader::BitReader;
use jxl_codec::headers::extra_channels::ExtraChannel;
use jxl_codec::headers::{FileHeader, JxlHeader};
use std::io::SeekFrom;

type Error = Box<dyn std::error::Error>;
const SIGNATURE: &[u8] = b"\0\0\0\x0cJXL \r\n\x87\n";
const MAX_INPUT: usize = 64 * 1024 * 1024;
const MAX_HEADER: usize = 64 * 1024;
const MAX_PIXELS: usize = 4 * 1024 * 1024;

fn error(kind: ImgErrorKind, message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(kind, message.into()))
}
fn unsupported(message: &str) -> Error {
    error(ImgErrorKind::UnsupportedFeature, message)
}
fn invalid(message: &str) -> Error {
    error(ImgErrorKind::IllegalData, message)
}
fn codec_error(e: jxl_codec::error::Error) -> Error {
    use jxl_codec::error::Error::*;
    let kind = match e {
        OutOfBounds(_) | SectionTooShort | UnexpectedCodestreamBoxEnd | IccEndOfStream => {
            ImgErrorKind::UnexpectedEof
        }
        OutOfMemory(_)
        | ImageOutOfMemory(..)
        | ImageSizeTooLarge(..)
        | ImageDimensionTooLarge(_) => ImgErrorKind::OutOfMemory,
        _ => ImgErrorKind::IllegalData,
    };
    error(kind, format!("JPEG XL: {e}"))
}
fn complete<T, U>(result: ProcessingResult<T, U>) -> Result<T, Error> {
    match result {
        ProcessingResult::Complete { result } => Ok(result),
        ProcessingResult::NeedsMoreInput { .. } => {
            Err(error(ImgErrorKind::UnexpectedEof, "truncated JPEG XL"))
        }
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b)
        .ok_or_else(|| invalid("JPEG XL size overflow"))
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("JPEG XL size overflow"))
}
fn append(out: &mut Vec<u8>, data: &[u8]) -> Result<(), Error> {
    out.try_reserve_exact(data.len())?;
    out.extend_from_slice(data);
    Ok(())
}

struct Input {
    codestream: Vec<u8>,
    metadata: Vec<(String, DataMap)>,
    metadata_bytes: usize,
}
// Validate every box, including trailing boxes, before the codec sees input.
// Never pass brob to an unbounded decompressor or discard unknown metadata.
// jbrd is retained as opaque source bytes; pixel decoding does not reconstruct
// JPEG files or parse/decompress/validate the reconstruction payload.
fn unpack(input: &[u8], limits: DecodeLimits) -> Result<Input, Error> {
    let mut out = Input {
        codestream: Vec::new(),
        metadata: Vec::new(),
        metadata_bytes: 0,
    };
    if input.starts_with(&[0xff, 0x0a]) {
        append(&mut out.codestream, input)?;
        return Ok(out);
    }
    if !input.starts_with(SIGNATURE) {
        return Err(invalid("invalid JPEG XL signature"));
    }
    let mut pos = SIGNATURE.len();
    let mut box_count = 0;
    let mut partial_index = 0u32;
    let mut finished = false;
    let mut partial = false;
    let mut seen_level = false;
    while pos < input.len() {
        if input.len() - pos < 8 {
            return Err(invalid("truncated JPEG XL box header"));
        }
        let size = u32::from_be_bytes(input[pos..pos + 4].try_into()?) as u64;
        let kind = &input[pos + 4..pos + 8];
        let (size, header) = if size == 1 {
            let bytes = input
                .get(pos + 8..pos + 16)
                .ok_or_else(|| invalid("truncated JPEG XL extended box"))?;
            (u64::from_be_bytes(bytes.try_into()?), 16)
        } else if size == 0 {
            ((input.len() - pos) as u64, 8)
        } else {
            (size, 8)
        };
        let size = usize::try_from(size)?;
        if size < header {
            return Err(invalid("invalid JPEG XL box length"));
        }
        let end = add(pos, size)?;
        let data = input
            .get(add(pos, header)?..end)
            .ok_or_else(|| invalid("truncated JPEG XL box"))?;
        if box_count == 0 && kind != b"ftyp" {
            return Err(invalid("JPEG XL container requires ftyp first"));
        }
        match kind {
            b"ftyp" => {
                if box_count != 0
                    || data.len() < 12
                    || data.len() % 4 != 0
                    || &data[..4] != b"jxl "
                    || data[4..8] != [0; 4]
                    || !data[8..].chunks_exact(4).any(|v| v == b"jxl ")
                {
                    return Err(invalid("invalid JPEG XL ftyp"));
                }
            }
            b"jxlc" => {
                if finished || partial {
                    return Err(invalid("duplicate JPEG XL codestream"));
                }
                append(&mut out.codestream, data)?;
                finished = true;
            }
            b"jxlp" => {
                if finished || data.len() < 4 {
                    return Err(invalid("invalid JPEG XL partial codestream"));
                }
                let index = u32::from_be_bytes(data[..4].try_into()?);
                if index & 0x7fff_ffff != partial_index {
                    return Err(invalid("out-of-order JPEG XL partial codestream"));
                }
                partial_index = partial_index
                    .checked_add(1)
                    .ok_or_else(|| invalid("too many JPEG XL chunks"))?;
                partial = true;
                finished = index & 0x8000_0000 != 0;
                append(&mut out.codestream, &data[4..])?;
            }
            b"jxll" => {
                // Both standardized container levels are accepted. Complexity
                // remains bounded by our independent dimensions/workspace caps.
                if seen_level || finished || partial || !matches!(data, [5] | [10]) {
                    return Err(invalid("invalid JPEG XL level box"));
                }
                seen_level = true;
            }
            b"brob" => {
                return Err(unsupported(
                    "JPEG XL compressed metadata boxes are unsupported",
                ));
            }
            b"JXL " => return Err(invalid("duplicate JPEG XL signature")),
            _ => {
                let key = format!(
                    "JPEG XL Source Box {box_count} {}",
                    kind.iter().map(|v| format!("{v:02x}")).collect::<String>()
                );
                out.metadata_bytes = add(out.metadata_bytes, add(data.len(), key.len())?)?;
                limits::check(
                    out.metadata_bytes,
                    limits.metadata_bytes,
                    "JPEG XL container metadata",
                )?;
                limits::check(box_count, 1024, "JPEG XL box count")?;
                let mut raw = Vec::new();
                append(&mut raw, data)?;
                out.metadata.push((key, DataMap::Raw(raw)));
            }
        }
        box_count += 1;
        pos = end;
    }
    if !finished || !out.codestream.starts_with(&[0xff, 0x0a]) {
        return Err(invalid("missing or incomplete JPEG XL codestream"));
    }
    Ok(out)
}

// ICC entropy contexts are pinned to jxl0.7.4 icc/mod.rs. Inspect the advertised
// expanded size before profile allocation, then advance to the frame headers
// using the same histogram/reader, without building an expanded ICC twice.
fn icc_budget(br: &mut BitReader<'_>, budget: usize) -> Result<usize, Error> {
    use jxl_codec::entropy_coding::decode::{Histograms, SymbolReader};
    use jxl_codec::headers::encodings::{Empty, UnconditionalCoder};
    let coded = u64::read_unconditional(&(), br, &Empty {}).map_err(codec_error)?;
    let coded = usize::try_from(coded)?;
    limits::check(coded, budget.min(1 << 24), "JPEG XL coded ICC")?;
    let hist = Histograms::decode(41, br, true).map_err(codec_error)?;
    let mut reader = SymbolReader::new(&hist, br, None).map_err(codec_error)?;
    let mut prefix = [0u8; 20];
    for b in prefix.iter_mut().take(coded.min(20)) {
        let sym = reader.read_unsigned(&hist, br, 0);
        br.check_for_error().map_err(codec_error)?;
        *b = u8::try_from(sym).map_err(|_| invalid("invalid JPEG XL ICC symbol"))?;
    }
    fn varint(data: &[u8], pos: &mut usize) -> Result<u64, Error> {
        let mut n = 0u64;
        for i in 0..9 {
            let b = *data
                .get(*pos)
                .ok_or_else(|| invalid("truncated JPEG XL ICC preamble"))?;
            *pos += 1;
            n |= u64::from(b & 127) << (i * 7);
            if b & 128 == 0 {
                return Ok(n);
            }
        }
        Err(invalid("invalid JPEG XL ICC varint"))
    }
    let mut pos = 0;
    let size = varint(&prefix[..coded.min(20)], &mut pos)?;
    let commands = varint(&prefix[..coded.min(20)], &mut pos)?;
    if commands.saturating_add(pos as u64) > coded as u64
        || size > 1 << 28
        || size.saturating_add(65536) < coded as u64
    {
        return Err(invalid("invalid JPEG XL ICC preamble"));
    }
    let size = usize::try_from(size)?;
    limits::check(size, budget, "JPEG XL expanded ICC")?;
    // Two retained profiles may both contain this ICC, plus generated output
    // profile/metadata overhead reserved by the caller.
    limits::check(mul(size, 2)?, budget, "JPEG XL retained ICC profiles")?;
    let decoded = coded.min(prefix.len());
    let tail = prefix[..decoded]
        .last_chunk::<2>()
        .ok_or_else(|| invalid("truncated JPEG XL ICC preamble"))?;
    let mut previous = [tail[1], tail[0]];
    for index in decoded..coded {
        let context = if index <= 128 {
            0
        } else {
            let first = match previous[0] {
                b'a'..=b'z' | b'A'..=b'Z' => 0,
                b'0'..=b'9' | b'.' | b',' => 1,
                0 => 2,
                1 => 3,
                2..=15 => 4,
                241..=254 => 5,
                255 => 6,
                _ => 7,
            };
            let second = match previous[1] {
                b'a'..=b'z' | b'A'..=b'Z' => 0,
                b'0'..=b'9' | b'.' | b',' => 1,
                0..=15 => 2,
                241..=255 => 3,
                _ => 4,
            };
            1 + first + 8 * second
        };
        let byte = u8::try_from(reader.read_unsigned(&hist, br, context))
            .map_err(|_| invalid("invalid JPEG XL ICC symbol"))?;
        br.check_for_error().map_err(codec_error)?;
        previous = [byte, previous[0]];
    }
    reader.check_final_state(&hist, br).map_err(codec_error)?;
    add(coded, size)
}

struct Header {
    width: usize,
    height: usize,
    bits: u8,
    alpha_bits: Option<u8>,
    native_bits: u8,
    orientation: u8,
    gray: bool,
    color_channels: usize,
    source_header_bytes: usize,
    workspace: usize,
    sample_limit: usize,
}

// Use the pinned codec's header/TOC parsers, not a second pixel decoder. Internal
// reference layers may exceed or lie outside the displayed canvas. Admit their
// actual padded storage before the codec allocates pixels, then leave cropping,
// orientation and blending to its coalescing render pipeline.
fn frame_workspace(
    input: &[u8],
    header: &FileHeader,
    frame_bits: usize,
    mut max_pixels: usize,
    base: usize,
    limits: DecodeLimits,
) -> Result<(usize, usize), Error> {
    use jxl_codec::headers::encodings::UnconditionalCoder;
    use jxl_codec::headers::frame_header::FrameHeader;
    use jxl_codec::headers::toc::IncrementalTocReader;
    let mut br = BitReader::new(input);
    br.skip_bits(frame_bits).map_err(codec_error)?;
    br.jump_to_byte_boundary().map_err(codec_error)?;
    let mut pos = br.total_bits_read() / 8;
    let nonserialized = header.frame_header_nonserialized();
    let channels = add(3, header.image_metadata.extra_channel_info.len())?;
    let mut retained = 0;
    let mut peak = add(base, mul(max_pixels, 128)?)?;
    let mut visible = 0;
    let mut toc_workspace = 0;
    for _ in 0..64 {
        let end = input.len().min(add(pos, MAX_HEADER)?);
        let data = input
            .get(pos..end)
            .ok_or_else(|| invalid("invalid JPEG XL frame offset"))?;
        let mut br = BitReader::new(data);
        let mut frame =
            FrameHeader::read_unconditional(&(), &mut br, &nonserialized).map_err(codec_error)?;
        frame.postprocess(&nonserialized);
        let (width, height) = frame.size_upsampled();
        let (padded_width, padded_height) = frame.size_padded_upsampled();
        let width = width.max(padded_width).max(16);
        let height = height.max(padded_height).max(16);
        limits::check(width, 16384, "JPEG XL internal layer width")?;
        limits::check(height, 16384, "JPEG XL internal layer height")?;
        // The codec clips signed layer rectangles in isize coordinates. Check
        // both ends and their negation before handing them to that pipeline,
        // including on 32-bit targets; negative crop origins remain valid.
        for (origin, extent) in [(frame.x0, width), (frame.y0, height)] {
            let origin = isize::try_from(origin)?;
            origin
                .checked_neg()
                .ok_or_else(|| invalid("JPEG XL crop origin overflow"))?;
            origin
                .checked_add(isize::try_from(extent)?)
                .and_then(isize::checked_neg)
                .ok_or_else(|| invalid("JPEG XL crop extent overflow"))?;
        }
        let pixels = mul(width, height)?;
        limits::check(pixels, MAX_PIXELS, "JPEG XL internal layer pixels")?;
        max_pixels = max_pixels.max(pixels);
        // Conservatively retain every preceding layer's f32 colour/extra planes,
        // even if a reference slot is replaced or the codec can free it sooner.
        retained = add(retained, mul(pixels, mul(channels, 4)?)?)?;
        let entries = frame.num_toc_entries();
        limits::check(entries, 4096, "JPEG XL frame TOC entries")?;
        if br.peek(1) != 0 {
            toc_workspace = 8 * 1024 * 1024;
        }
        br.check_for_error().map_err(codec_error)?;
        peak = peak.max(add(
            add(add(base, mul(max_pixels, 128)?)?, retained)?,
            toc_workspace,
        )?);
        limits::check(
            peak,
            limits.expanded_bytes,
            "JPEG XL internal layer workspace",
        )?;
        let mut toc =
            IncrementalTocReader::new(u32::try_from(entries)?, &mut br).map_err(codec_error)?;
        while !toc.is_complete() {
            toc.read_step(&mut br).map_err(codec_error)?;
        }
        let toc = toc.finalize();
        br.jump_to_byte_boundary().map_err(codec_error)?;
        let payload = toc.entries.iter().try_fold(0, |n, &v| add(n, v as usize))?;
        pos = add(add(pos, br.total_bits_read() / 8)?, payload)?;
        if pos > input.len() {
            return Err(error(
                ImgErrorKind::UnexpectedEof,
                "truncated JPEG XL frame payload",
            ));
        }
        visible += usize::from(frame.is_visible());
        if visible > 1 {
            return Err(unsupported(
                "JPEG XL multiple visible frames are unsupported",
            ));
        }
        if frame.is_last {
            if pos != input.len() || visible != 1 {
                return Err(invalid("trailing or incomplete JPEG XL frame sequence"));
            }
            // Upstream rejects >=limit and counts three colour channels even
            // for Gray. Keep its runtime check, using the scanned layer bound.
            return Ok((peak, add(mul(max_pixels, channels)?, 1)?));
        }
    }
    Err(unsupported("JPEG XL exceeds64 internal layers"))
}

fn preflight(input: &Input, limits: DecodeLimits) -> Result<Header, Error> {
    // FileHeader has bounded channel tables, but does not read ICC/pixel data.
    limits::check(
        1024 * 1024,
        limits.expanded_bytes,
        "JPEG XL header workspace",
    )?;
    let mut br = BitReader::new(&input.codestream[..input.codestream.len().min(MAX_HEADER)]);
    let header = FileHeader::read(&mut br).map_err(codec_error)?;
    let m = &header.image_metadata;
    if m.animation.is_some() || m.preview.is_some() || m.intrinsic_size.is_some() {
        return Err(unsupported(
            "JPEG XL animations, previews and intrinsic sizes are unsupported",
        ));
    }
    let bits = m.bit_depth.bits_per_sample();
    if m.bit_depth.floating_point_sample() || !matches!(bits, 8 | 9 | 10 | 12 | 16) {
        return Err(unsupported(
            "JPEG XL supports integer8/9/10/12/16-bit sources only",
        ));
    }
    if m.extra_channel_info.len() > 1
        || m.extra_channel_info.iter().any(|c| {
            c.ec_type != ExtraChannel::Alpha
                || c.alpha_associated()
                || c.dim_shift() != 0
                || c.bit_depth().floating_point_sample()
                || !matches!(c.bit_depth().bits_per_sample(), 8 | 9 | 10 | 12 | 16)
        })
    {
        return Err(unsupported(
            "JPEG XL requires at most one full-resolution straight integer alpha channel with 8/9/10/12/16-bit depth",
        ));
    }
    let alpha_bits = m
        .extra_channel_info
        .first()
        .map(|c| c.bit_depth().bits_per_sample() as u8);
    // One output format cannot express independent channel depths. Normalize
    // mixed integer channels to 16 bits before EX storage/CMS/legacy rounding.
    let native_bits = if matches!(bits, 9 | 10) || alpha_bits.is_some_and(|b| u32::from(b) != bits)
    {
        16
    } else {
        bits as u8
    };
    use jxl_codec::headers::color_encoding::{ColorSpace, TransferFunction};
    if !matches!(
        m.color_encoding.color_space,
        ColorSpace::Gray | ColorSpace::RGB
    ) || (!m.color_encoding.want_icc
        && matches!(
            m.color_encoding.tf.transfer_function,
            TransferFunction::PQ | TransferFunction::HLG
        ))
        || !m.tone_mapping.all_default
    {
        return Err(unsupported(
            "JPEG XL HDR/tone mapping and non-Gray/RGB colour are unsupported",
        ));
    }
    let (width, height) = m
        .orientation
        .map_size((header.size.xsize() as usize, header.size.ysize() as usize));
    limits::check(width, 16384, "JPEG XL width")?;
    limits::check(height, 16384, "JPEG XL height")?;
    let pixels = mul(width, height)?;
    limits::check(pixels, limits.pixels.min(MAX_PIXELS), "JPEG XL pixels")?;
    limits::check(1, limits.frames, "JPEG XL still frames")?;
    limits::check(
        mul(pixels, if native_bits == 8 { 4 } else { 8 })?,
        limits.animation_bytes,
        "JPEG XL canvas",
    )?;
    let header_bits = br.total_bits_read();
    let header_bytes = header_bits.div_ceil(8);
    // Conservative allowance for two small generated matrix/TRC profiles and
    // metadata keys; checked before profile generation or ICC parsing.
    let meta = add(add(input.metadata_bytes, header_bytes)?, 16 * 1024)?;
    limits::check(meta, limits.metadata_bytes, "JPEG XL source metadata")?;
    // Upstream sample_limit also applies to cropped/internal frame dimensions.
    // Cover its16-pixel padding and the4/3 RGB-vs-RGBA sample-count ratio.
    let padded = mul(width.max(16), height.max(16))?;
    let frame_pixels = add(mul(padded, 4)?, 2)? / 3;
    limits::check(frame_pixels, MAX_PIXELS, "JPEG XL padded frame pixels")?;
    let mut base = add(1024 * 1024, mul(input.codestream.len(), 2)?)?;
    let mut frame_bits = header_bits;
    if m.color_encoding.want_icc {
        // Histograms/SymbolReader can allocate a 4 MiB LZ77 ring before a
        // profile exists. Reserve that allowance before inspecting its prefix.
        base = add(base, 8 * 1024 * 1024)?;
        limits::check(
            add(base, mul(frame_pixels, 128)?)?,
            limits.expanded_bytes,
            "JPEG XL ICC workspace",
        )?;
        let mut icc_reader = BitReader::new(&input.codestream);
        icc_reader.skip_bits(header_bits).map_err(codec_error)?;
        let icc = icc_budget(&mut icc_reader, limits.metadata_bytes.saturating_sub(meta))?;
        frame_bits = icc_reader.total_bits_read();
        base = add(base, mul(icc, 4)?)?;
    }
    limits::check(
        add(base, mul(frame_pixels, 128)?)?,
        limits.expanded_bytes,
        "JPEG XL decode workspace",
    )?;
    let (workspace, sample_limit) = frame_workspace(
        &input.codestream,
        &header,
        frame_bits,
        frame_pixels,
        base,
        limits,
    )?;
    limits::check(workspace, limits.expanded_bytes, "JPEG XL decode workspace")?;
    Ok(Header {
        width,
        height,
        bits: bits as u8,
        alpha_bits,
        native_bits,
        orientation: m.orientation as u8,
        gray: m.color_encoding.color_space == ColorSpace::Gray && m.extra_channel_info.is_empty(),
        color_channels: if m.color_encoding.color_space == ColorSpace::Gray {
            1
        } else {
            3
        },
        source_header_bytes: header_bytes,
        workspace,
        sample_limit,
    })
}

// Validate the profile envelope instead of trusting channels(), which defaults
// unknown/truncated ICC signatures toRGB. This does not implement a CMS.
fn validate_icc(bytes: &[u8], channels: usize) -> Result<(), Error> {
    if bytes.len() < 132
        || bytes[36..40] != *b"acsp"
        || u32::from_be_bytes(bytes[..4].try_into()?) as usize != bytes.len()
    {
        return Err(invalid("invalid JPEG XL ICC header"));
    }
    if (channels == 1 && bytes[16..20] != *b"GRAY") || (channels == 3 && bytes[16..20] != *b"RGB ")
    {
        return Err(unsupported(
            "JPEG XL ICC colour space must match Gray/RGB source channels",
        ));
    }
    let count = u32::from_be_bytes(bytes[128..132].try_into()?) as usize;
    let end = add(132, mul(count, 12)?)?;
    if end > bytes.len() {
        return Err(invalid("invalid JPEG XL ICC tag table"));
    }
    for tag in bytes[132..end].chunks_exact(12) {
        let offset = u32::from_be_bytes(tag[4..8].try_into()?) as usize;
        let len = u32::from_be_bytes(tag[8..12].try_into()?) as usize;
        if offset < end || add(offset, len)? > bytes.len() {
            return Err(invalid("invalid JPEG XL ICC tag extent"));
        }
        if &tag[..4] == b"cicp" {
            let data = &bytes[offset..offset + len];
            if data.len() < 12 || &data[..4] != b"cicp" {
                return Err(invalid("invalid JPEG XL ICC CICP tag"));
            }
            if matches!(data[9], 16 | 18) {
                return Err(unsupported(
                    "JPEG XL ICC PQ/HLG requires explicit HDR processing",
                ));
            }
        }
    }
    Ok(())
}

fn profile_metadata(
    key: &str,
    profile: &JxlColorProfile,
    channels: usize,
) -> Result<(String, DataMap), Error> {
    // Avoid as_icc(), which panics for unsupported simple colour encodings.
    let icc = match profile {
        JxlColorProfile::Icc(bytes) => {
            validate_icc(bytes, channels)?;
            bytes.clone()
        }
        JxlColorProfile::Simple(enc) => enc
            .maybe_create_profile()
            .map_err(codec_error)?
            .ok_or_else(|| unsupported("JPEG XL output profile cannot be represented as ICC"))?,
    };
    validate_icc(&icc, channels)?;
    Ok((key.into(), DataMap::ICCProfile(icc)))
}

pub(crate) fn decode_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    decode::<B, false>(reader, option, false)
}
#[cfg(feature = "image-buffer-ex")]
pub(crate) fn decode_ex_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    decode::<B, false>(reader, option, true)
}
/// Decode a supported JPEG XL still into sRGB RGBA8 for display.
///
/// Unlike the source-coordinate ordinary/EX APIs, this explicitly applies the
/// own ICC engine to the codec's actual reconstructed output profile. Alpha is
/// excluded; ICC conversion precedes final RGBA8 rounding. Requires
/// `jpeg-xl,color-management`. Source and reconstruction profiles are retained.
#[cfg(feature = "color-management")]
pub fn image_from_srgb(input: &[u8]) -> Result<crate::draw::ImageBuffer, Error> {
    image_from_srgb_with_limits(input, DecodeLimits::default())
}

/// The sRGB display route with explicit input, pixel and workspace limits.
#[cfg(feature = "color-management")]
pub fn image_from_srgb_with_limits(
    input: &[u8],
    limits: DecodeLimits,
) -> Result<crate::draw::ImageBuffer, Error> {
    let mut image = crate::draw::ImageBuffer::new();
    crate::decode_guard::run(
        &mut bin_rs::reader::BytesReader::new(input),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
        |reader, options| decode::<_, true>(reader, options, false),
    )?;
    Ok(image)
}

fn decode<B: BinaryReader, const MANAGED: bool>(
    reader: &mut B,
    option: &mut DecodeOptions,
    native: bool,
) -> Result<Option<ImgWarnings>, Error> {
    // Compile the explicit display path separately so its CMS work cannot
    // obscure optimization of existing source-coordinate entry points.
    let managed = MANAGED;
    let limits = limits::current();
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let len = usize::try_from(
        end.checked_sub(start)
            .ok_or_else(|| invalid("invalid JPEG XL input position"))?,
    )?;
    limits::check(len, limits.input_bytes.min(MAX_INPUT), "JPEG XL input")?;
    limits::check(
        mul(len, 3)?,
        limits.expanded_bytes,
        "JPEG XL input assembly workspace",
    )?;
    let mut raw = crate::draw::zeroed_bytes(len)?;
    reader.read_exact(&mut raw)?;
    let mut input = unpack(&raw, limits)?;
    drop(raw);
    let h = preflight(&input, limits)?;
    let mut options = JxlDecoderOptions::default();
    options.sample_limit = Some(h.sample_limit);
    options.high_precision = true;
    options.render_spot_colors = false;
    let mut stream = input.codestream.as_slice();
    let decoder = JxlDecoder::<states::Initialized>::new(options);
    let mut decoder = complete(decoder.process(&mut stream, None).map_err(codec_error)?)?;
    if decoder.basic_info().size != (h.width, h.height) {
        return Err(invalid("JPEG XL dimensions changed after preflight"));
    }
    // Decode at source precision even for the legacy path: upstream U8
    // narrowing dithers RGB, whereas WML2 promises deterministic rounding.
    let bits = if managed { 16 } else { h.native_bits };
    let gray = native && h.gray;
    let channels = if gray { 1 } else { 4 };
    let format = JxlPixelFormat {
        color_type: if gray {
            JxlColorType::Grayscale
        } else {
            JxlColorType::Rgba
        },
        color_data_format: Some(if bits == 8 {
            JxlDataFormat::U8 { bit_depth: 8 }
        } else {
            JxlDataFormat::U16 {
                endianness: Endianness::LittleEndian,
                bit_depth: bits,
            }
        }),
        extra_channel_format: vec![None; decoder.basic_info().extra_channels.len()],
    };
    decoder.set_pixel_format(format).map_err(codec_error)?;
    input.metadata.push(profile_metadata(
        "JPEG XL Source ICC Profile",
        decoder.embedded_color_profile(),
        h.color_channels,
    )?);
    let mut output_profile = decoder.output_color_profile().clone();
    if !managed && !gray && decoder.output_color_profile().channels() == 1 {
        output_profile = match output_profile {
            JxlColorProfile::Simple(JxlColorEncoding::GrayscaleColorSpace {
                white_point,
                transfer_function,
                rendering_intent,
            }) => JxlColorProfile::Simple(JxlColorEncoding::RgbColorSpace {
                white_point,
                transfer_function,
                rendering_intent,
                primaries: JxlPrimaries::SRGB,
            }),
            _ => {
                return Err(unsupported(
                    "JPEG XL Gray ICC expansion to RGBA requires an explicit CMS",
                ));
            }
        };
    }
    input.metadata.push(profile_metadata(
        if managed {
            "JPEG XL Reconstruction ICC Profile"
        } else {
            "ICC Profile"
        },
        &output_profile,
        if managed {
            output_profile.channels()
        } else if gray {
            1
        } else {
            3
        },
    )?);
    #[cfg(feature = "color-management")]
    if managed {
        let mut profile = Vec::new();
        append(&mut profile, crate::color_convert::SRGB_ICC)?;
        input
            .metadata
            .push(("ICC Profile".into(), DataMap::ICCProfile(profile)));
    }
    input
        .metadata
        .push(("orientation_applied".into(), DataMap::Bool(true)));
    input.metadata.push((
        "JPEG XL Source Orientation".into(),
        DataMap::UInt(h.orientation.into()),
    ));
    input
        .metadata
        .push(("Format".into(), DataMap::Ascii("JPEG XL".into())));
    input.metadata.push((
        "JPEG XL Source Bit Depth".into(),
        DataMap::UInt(h.bits.into()),
    ));
    // A mixed-depth image has one normalized 16-bit native layout; retain the
    // independent source alpha depth instead of relabeling it as color depth.
    if let Some(alpha_bits) = h.alpha_bits.filter(|&b| b != h.bits) {
        input.metadata.push((
            "JPEG XL Source Alpha Bit Depth".into(),
            DataMap::UInt(alpha_bits.into()),
        ));
    }
    input.metadata.push((
        "JPEG XL Output Bit Depth".into(),
        DataMap::UInt(if native { bits.into() } else { 8 }),
    ));
    input.metadata.push((
        "JPEG XL Source Header".into(),
        DataMap::Raw(input.codestream[..h.source_header_bytes].to_vec()),
    ));
    let metadata_bytes = input.metadata.iter().try_fold(0usize, |n, (k, v)| {
        let bytes = match v {
            DataMap::Raw(b) | DataMap::ICCProfile(b) => b.len(),
            DataMap::Ascii(s) => s.len(),
            _ => 8,
        };
        add(n, add(k.len(), bytes)?)
    })?;
    limits::check(metadata_bytes, limits.metadata_bytes, "JPEG XL metadata")?;
    let retained_workspace = add(h.workspace, mul(metadata_bytes, 2)?)?;
    limits::check(
        retained_workspace,
        limits.expanded_bytes,
        "JPEG XL metadata workspace",
    )?;
    let decoder = complete(decoder.process(&mut stream, None).map_err(codec_error)?)?;
    let row = mul(h.width, mul(channels, if bits == 8 { 1 } else { 2 })?)?;
    let bytes = mul(row, h.height)?;
    // Vec<u16> guarantees the upstream row alignment even on WASM. All rows
    // have an even byte stride for U16; the U8 path has no alignment requirement.
    let mut aligned: Vec<u16> = Vec::new();
    aligned.try_reserve_exact(bytes.div_ceil(2))?;
    aligned.resize(bytes.div_ceil(2), 0);
    // SAFETY: u16 has no invalid bit patterns; the exclusively borrowed,
    // initialized allocation covers bytes.len(), and out does not outlive it.
    let out = unsafe { std::slice::from_raw_parts_mut(aligned.as_mut_ptr().cast::<u8>(), bytes) };
    let mut buffers = [JxlOutputBuffer::new(out, h.height, row)];
    let decoder = complete(
        decoder
            .process(&mut stream, &mut buffers, None)
            .map_err(codec_error)?,
    )?;
    if decoder.has_more_frames() {
        return Err(unsupported(
            "JPEG XL multiple visible frames are unsupported",
        ));
    }
    // The codestream parser may buffer ahead; file_length is its authoritative
    // boundary and catches trailing junk/multiple raw streams without guessing.
    if decoder.file_length() != Some(input.codestream.len() as u64) {
        return Err(invalid("trailing or incomplete JPEG XL codestream"));
    }
    let legacy = if !managed && !native && bits != 8 {
        let max = (1u32 << bits) - 1;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(out.len() / 2)?;
        for word in out.chunks_exact(2) {
            let v = u32::from(u16::from_le_bytes([word[0], word[1]]));
            bytes.push(((v * 255 + max / 2) / max) as u8);
        }
        Some(bytes)
    } else {
        None
    };
    #[cfg(feature = "color-management")]
    let managed_output = if managed {
        let profile = input
            .metadata
            .iter()
            .find_map(|(key, value)| {
                if key == "JPEG XL Reconstruction ICC Profile"
                    && let DataMap::ICCProfile(bytes) = value
                {
                    return Some(bytes.as_slice());
                }
                None
            })
            .ok_or_else(|| invalid("missing JPEG XL reconstruction profile"))?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(out.len() / 2)?;
        rgba.extend(
            out.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        );
        crate::color_convert::rgba16_to_rgb_profile(
            &mut rgba,
            profile,
            crate::color_convert::SRGB_ICC,
            h.color_channels == 1,
            limits.expanded_bytes.saturating_sub(retained_workspace),
        )?;
        let mut bytes = crate::draw::zeroed_bytes(rgba.len())?;
        for (dst, sample) in bytes.iter_mut().zip(rgba) {
            *dst = ((u32::from(sample) * 255 + 32767) / 65535) as u8;
        }
        Some(bytes)
    } else {
        None
    };
    #[cfg(not(feature = "color-management"))]
    let managed_output: Option<Vec<u8>> = None;
    #[cfg(feature = "image-buffer-ex")]
    if native {
        use crate::draw::{InitOptionsEx, PixelFormatEx};
        let source_format = match (gray, bits) {
            (true, 8) => PixelFormatEx::Gray8,
            (true, 12) => PixelFormatEx::Gray12,
            (true, 16) => PixelFormatEx::Gray16,
            (false, 8) => PixelFormatEx::Rgba8,
            (false, 12) => PixelFormatEx::Rgba12,
            (false, 16) => PixelFormatEx::Rgba16,
            _ => return Err(unsupported("unsupported JPEG XL EX layout")),
        };
        option.drawer.init_ex(
            h.width,
            h.height,
            Some(InitOptionsEx {
                source_format,
                source_stride: None,
                legacy: Some(InitOptions {
                    loop_count: 0,
                    background: None,
                    animation: false,
                }),
            }),
        )?;
    } else {
        option.drawer.init(
            h.width,
            h.height,
            Some(InitOptions {
                loop_count: 0,
                background: None,
                animation: false,
            }),
        )?;
    }
    #[cfg(not(feature = "image-buffer-ex"))]
    option.drawer.init(
        h.width,
        h.height,
        Some(InitOptions {
            loop_count: 0,
            background: None,
            animation: false,
        }),
    )?;
    option.drawer.draw(
        0,
        0,
        h.width,
        h.height,
        managed_output
            .as_deref()
            .or(legacy.as_deref())
            .unwrap_or(out),
        None,
    )?;
    for (key, value) in input.metadata {
        option.drawer.set_metadata(&key, value)?;
    }
    option.drawer.terminate(None)?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_icc_cursor_matches_pinned_codec_before_frame_headers() {
        for source in [
            include_bytes!("../tests/fixtures/review/jxl-icc-lossless.jxl").as_slice(),
            include_bytes!("../tests/fixtures/review/jxl-icc-gray16.jxl").as_slice(),
            include_bytes!("../tests/fixtures/review/jxl-jbrd-bench.jxl").as_slice(),
            include_bytes!("../tests/fixtures/review/jxl-jbrd-gray.jxl").as_slice(),
        ] {
            let input = unpack(source, DecodeLimits::default()).unwrap();
            let mut actual = BitReader::new(&input.codestream);
            let header = FileHeader::read(&mut actual).unwrap();
            assert!(header.image_metadata.color_encoding.want_icc);
            let mut reference = BitReader::new(&input.codestream);
            reference.skip_bits(actual.total_bits_read()).unwrap();
            icc_budget(&mut actual, 32 << 20).unwrap();
            let mut reader = jxl_codec::icc::IncrementalIccReader::new(&mut reference).unwrap();
            reader.read_all(&mut reference).unwrap();
            let profile = reader.finalize(&mut reference).unwrap();
            assert!(profile.len() >= 128);
            assert_eq!(actual.total_bits_read(), reference.total_bits_read());
        }
    }

    #[test]
    fn icc_envelopes_cannot_relabel_unknown_or_hdr_profiles() {
        let valid = include_bytes!("../tests/fixtures/review/jxl-gamma2.icc");
        validate_icc(valid, 3).unwrap();
        assert!(validate_icc(valid, 1).is_err());
        for (offset, bytes) in [
            (0, b"\0\0\0\x80".as_slice()),
            (16, b"CMYK"),
            (36, b"junk"),
            (128, b"\xff\xff\xff\xff"),
        ] {
            let mut corrupt = valid.to_vec();
            corrupt[offset..offset + 4].copy_from_slice(bytes);
            assert!(validate_icc(&corrupt, 3).is_err());
        }
        assert!(validate_icc(&valid[..127], 3).is_err());
        // Independently construct a minimal CICP envelope; no transfer-function
        // interpretation beyond the explicit PQ/HLG signal is claimed.
        let mut hdr = vec![0; 156];
        hdr[..4].copy_from_slice(&156u32.to_be_bytes());
        hdr[16..20].copy_from_slice(b"RGB ");
        hdr[36..40].copy_from_slice(b"acsp");
        hdr[128..132].copy_from_slice(&1u32.to_be_bytes());
        hdr[132..136].copy_from_slice(b"cicp");
        hdr[136..140].copy_from_slice(&144u32.to_be_bytes());
        hdr[140..144].copy_from_slice(&12u32.to_be_bytes());
        hdr[144..148].copy_from_slice(b"cicp");
        for tf in [16, 18] {
            hdr[153] = tf;
            assert!(validate_icc(&hdr, 3).is_err());
        }
        hdr[153] = 13;
        validate_icc(&hdr, 3).unwrap();
    }
}
