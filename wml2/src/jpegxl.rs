//! Bounded still-image decoding with published `jxl 0.8.0`.
//! Published 0.7.4 is isolated to pre-allocation parser compatibility.
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
use jxl_preflight::bit_reader::BitReader;
use jxl_preflight::headers::extra_channels::ExtraChannel;
use jxl_preflight::headers::{FileHeader, JxlHeader};
use std::io::SeekFrom;

#[path = "jpegxl_brob.rs"]
mod brob;
#[path = "jpegxl_dimensions.rs"]
pub(crate) mod dimensions;
#[path = "jpegxl_resources.rs"]
mod resources;
#[path = "jpegxl_metadata.rs"]
mod structured_metadata;
pub use resources::JxlResourceLimits;
#[path = "jpegxl_hdr_linear.rs"]
mod hdr_linear;
pub use hdr_linear::{JxlHdrLinearization, JxlLinearHdrImage};

type Error = Box<dyn std::error::Error>;
const SIGNATURE: &[u8] = b"\0\0\0\x0cJXL \r\n\x87\n";
const MAX_INPUT: usize = 64 * 1024 * 1024;
// Separate from the file cap: caller unlimited() cannot admit unbounded brob expansion.
const MAX_BROB_METADATA: usize = 64 * 1024 * 1024;
const MAX_HEADER: usize = 64 * 1024;
const MAX_PIXELS: usize = 4 * 1024 * 1024;

/// Cancellation returned by the cancellable sRGB API; downcast the boxed error
/// to this type to distinguish it from malformed input and resource limits.
#[derive(Debug)]
pub struct DecodeCancelled;
impl std::fmt::Display for DecodeCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JPEG XL decode cancelled")
    }
}
impl std::error::Error for DecodeCancelled {}

struct Cancellation<'a> {
    probe: Option<&'a dyn Fn() -> bool>,
    cancelled: std::cell::Cell<bool>,
}
impl Cancellation<'_> {
    fn check(&self) -> Result<(), Error> {
        if self.cancelled.get() || self.probe.is_some_and(|probe| probe()) {
            self.cancelled.set(true);
            return Err(Box::new(DecodeCancelled));
        }
        Ok(())
    }
    fn codec_error(&self, e: jxl_codec::error::Error) -> Error {
        if self.cancelled.get() {
            Box::new(DecodeCancelled)
        } else {
            codec_error(e)
        }
    }
}
struct CancellableRunner<'a>(&'a Cancellation<'a>);
impl JxlParallelRunner for CancellableRunner<'_> {
    fn num_threads(&self) -> usize {
        1
    }
    fn run(&mut self, num: usize, fun: &JxlParallelRunnerFun<'_>) -> jxl_codec::error::Result<()> {
        for i in 0..num {
            self.0
                .check()
                .map_err(|_| std::io::Error::other(DecodeCancelled))?;
            fun(i)?;
        }
        self.0
            .check()
            .map_err(|_| std::io::Error::other(DecodeCancelled))?;
        Ok(())
    }
    // The default run_ordered bypasses run when num_threads is one.
    fn run_ordered(
        &mut self,
        num: usize,
        _: Option<usize>,
        fun: &JxlParallelRunnerFun<'_>,
    ) -> jxl_codec::error::Result<()> {
        self.run(num, fun)
    }
}

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
// The auxiliary crate supplies only existing bounded preflight readers.
fn preflight_error(e: jxl_preflight::error::Error) -> Error {
    use jxl_preflight::error::Error::*;
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
fn expect_event(event: Event, expected: Event) -> Result<(), Error> {
    if matches!(event, Event::NeedMoreInput { .. }) {
        Err(error(ImgErrorKind::UnexpectedEof, "truncated JPEG XL"))
    } else if event == expected {
        Ok(())
    } else {
        Err(invalid("unexpected JPEG XL decoder event"))
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

fn box_key(stage: &str, index: usize, kind: &[u8]) -> String {
    format!(
        "JPEG XL {stage} Box {index} {}",
        kind.iter().map(|v| format!("{v:02x}")).collect::<String>()
    )
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
    unpack_with_resources(input, limits, None)
}
fn unpack_with_resources(
    input: &[u8],
    limits: DecodeLimits,
    resources: Option<JxlResourceLimits>,
) -> Result<Input, Error> {
    let policy = resources.unwrap_or_else(JxlResourceLimits::legacy);
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
    let mut expanded_brob = 0usize;
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
                let kind: &[u8; 4] = data
                    .get(..4)
                    .ok_or_else(|| invalid("truncated JPEG XL brob type"))?
                    .try_into()?;
                // ISO container compression excludes codec/control data and
                // recursive wrappers. Retaining opaque jbrd is not reconstruction.
                if kind.starts_with(b"jxl") || matches!(kind, b"jbrd" | b"brob") {
                    return Err(invalid("forbidden JPEG XL brob box type"));
                }
                if kind.starts_with(b"JXL") || kind == b"ftyp" {
                    return Err(unsupported("JPEG XL brob control box is unsupported"));
                }
                limits::check(box_count, 1024, "JPEG XL box count")?;
                let source_key = box_key("Source", box_count, b"brob");
                let decoded_key = box_key("Decoded", box_count, kind);
                let retained = add(
                    out.metadata_bytes,
                    add(add(data.len(), source_key.len())?, decoded_key.len())?,
                )?;
                limits::check(
                    retained,
                    limits.metadata_bytes,
                    "JPEG XL container metadata",
                )?;
                let maximum = limits
                    .metadata_bytes
                    .saturating_sub(retained)
                    .min(policy.brob_bytes.saturating_sub(expanded_brob));
                // The original input/codestream assembly and metadata copies
                // remain live while Brotli runs. Output growth and all codec
                // allocator requests are charged by the shared bounded helper.
                let base = add(mul(input.len(), 3)?, mul(retained, 2)?)?;
                let decoded = brob::decompress(&data[4..], maximum, base, limits.expanded_bytes)?;
                expanded_brob = add(expanded_brob, decoded.len())?;
                out.metadata_bytes = add(retained, decoded.len())?;
                let mut raw = Vec::new();
                append(&mut raw, data)?;
                out.metadata.push((source_key, DataMap::Raw(raw)));
                out.metadata.push((decoded_key, DataMap::Raw(decoded)));
            }
            b"JXL " => return Err(invalid("duplicate JPEG XL signature")),
            _ => {
                let key = box_key("Source", box_count, kind);
                out.metadata_bytes = add(out.metadata_bytes, add(data.len(), key.len())?)?;
                limits::check(
                    out.metadata_bytes,
                    limits.metadata_bytes,
                    "JPEG XL container metadata",
                )?;
                if resources.is_some() {
                    limits::check(
                        add(mul(input.len(), 3)?, mul(out.metadata_bytes, 2)?)?,
                        limits.expanded_bytes,
                        "JPEG XL container workspace",
                    )?;
                }
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
fn icc_budget(
    br: &mut BitReader<'_>,
    budget: usize,
    policy: JxlResourceLimits,
) -> Result<usize, Error> {
    use jxl_preflight::entropy_coding::decode::{Histograms, SymbolReader};
    use jxl_preflight::headers::encodings::{Empty, UnconditionalCoder};
    let coded = u64::read_unconditional(&(), br, &Empty {}).map_err(preflight_error)?;
    let coded = usize::try_from(coded)?;
    limits::check(
        coded,
        budget.min(policy.icc_coded_bytes),
        "JPEG XL coded ICC",
    )?;
    let hist = Histograms::decode(41, br, true).map_err(preflight_error)?;
    let mut reader = SymbolReader::new(&hist, br, None).map_err(preflight_error)?;
    let mut prefix = [0u8; 20];
    for b in prefix.iter_mut().take(coded.min(20)) {
        let sym = reader.read_unsigned(&hist, br, 0);
        br.check_for_error().map_err(preflight_error)?;
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
    limits::check(
        size,
        policy.icc_expanded_bytes,
        "JPEG XL expanded ICC policy",
    )?;
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
        br.check_for_error().map_err(preflight_error)?;
        previous = [byte, previous[0]];
    }
    reader
        .check_final_state(&hist, br)
        .map_err(preflight_error)?;
    add(coded, size)
}

struct Header {
    dimensions: crate::image_info::ImageDimensions,
    source_intrinsic: Option<crate::image_info::ImageSize>,
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
    preview_workspace: usize,
    hdr: Option<JxlHdrSignal>,
    sample_limit: usize,
}

// Cross-check the public 0.8 header projection against the independent bounded
// preflight. This is defense in depth; ICC admission still precedes BasicInfo.
fn validate_basic_info(info: &JxlBasicInfo, h: &Header) -> Result<(), Error> {
    if info.size != (h.width, h.height)
        || info.orientation as u8 != h.orientation
        || !matches!(info.bit_depth, JxlBitDepth::Int { bits_per_sample } if bits_per_sample == u32::from(h.bits))
        || info.animation.is_some()
        || info.extra_channels.len() != usize::from(h.alpha_bits.is_some())
        || info.extra_channels.iter().any(|channel| {
            channel.ec_type != jxl_codec::api::ExtraChannel::Alpha || channel.alpha_associated
        })
    {
        return Err(invalid(
            "JPEG XL basic info disagrees with bounded preflight",
        ));
    }
    Ok(())
}

// Use the parser-only compatibility alias; pixel decoding is exclusively 0.8.0. Internal
// reference layers may exceed or lie outside the displayed canvas. Admit their
// actual padded storage before the codec allocates pixels, then leave cropping,
// orientation and blending to its coalescing render pipeline.
fn frame_workspace<const FLOAT_BITS: u32, const HDR: bool>(
    input: &[u8],
    header: &FileHeader,
    frame_bits: usize,
    mut max_pixels: usize,
    base: usize,
    limits: DecodeLimits,
    policy: JxlResourceLimits,
) -> Result<(usize, usize, usize), Error> {
    use jxl_preflight::headers::encodings::UnconditionalCoder;
    use jxl_preflight::headers::frame_header::FrameHeader;
    use jxl_preflight::headers::toc::IncrementalTocReader;
    let mut br = BitReader::new(input);
    br.skip_bits(frame_bits).map_err(preflight_error)?;
    br.jump_to_byte_boundary().map_err(preflight_error)?;
    let mut pos = br.total_bits_read() / 8;
    let main_nonserialized = header.frame_header_nonserialized();
    let preview_nonserialized = header.preview_frame_header_nonserialized();
    let mut preview = preview_nonserialized.is_some();
    let channels = add(3, header.image_metadata.extra_channel_info.len())?;
    // The codec checks preview samples even while skipping. Admit their
    // dimensions independently, without reserving nonexistent preview planes.
    let mut sample_pixels = max_pixels;
    if let Some(size) = &header.image_metadata.preview {
        let pixels = mul(
            (size.xsize() as usize).max(16),
            (size.ysize() as usize).max(16),
        )?;
        limits::check(pixels, policy.internal_pixels, "JPEG XL preview pixels")?;
        sample_pixels = sample_pixels.max(pixels);
    }
    let mut preview_workspace = 0;
    let mut retained = 0;
    let mut peak = add(base, mul(max_pixels, 128)?)?;
    let mut visible = 0;
    let mut toc_workspace = 0;
    let mut main_layers = 0;
    // One independently skipped preview does not consume a main-image layer
    // or caller frame. Main sequence complexity remains capped at64 layers.
    for _ in 0..(64 + usize::from(preview)) {
        if !preview {
            main_layers += 1;
            limits::check(
                main_layers,
                policy.internal_layers,
                "JPEG XL internal layers",
            )?;
        }
        let nonserialized = if preview {
            preview_nonserialized.as_ref().unwrap()
        } else {
            &main_nonserialized
        };
        let end = input.len().min(add(pos, MAX_HEADER)?);
        let data = input
            .get(pos..end)
            .ok_or_else(|| invalid("invalid JPEG XL frame offset"))?;
        let mut br = BitReader::new(data);
        let mut frame = FrameHeader::read_unconditional(&(), &mut br, nonserialized)
            .map_err(preflight_error)?;
        if FLOAT_BITS != 0
            && frame.encoding != jxl_preflight::headers::frame_header::Encoding::Modular
        {
            return Err(unsupported(
                "JPEG XL native F32 currently requires Modular frames",
            ));
        }
        frame.postprocess(nonserialized);
        // In 0.7.4 a non-visible preview overrides Skip with SkipOutput and
        // would allocate pixel/reference state. Only a terminal visible preview
        // reaches its section-skip path without constructing pixel state.
        if preview && (!frame.is_last || !frame.is_visible()) {
            return Err(unsupported(
                "JPEG XL preview must be one terminal visible frame for bounded skipping",
            ));
        }
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
        limits::check(
            pixels,
            policy.internal_pixels,
            "JPEG XL internal layer pixels",
        )?;
        sample_pixels = sample_pixels.max(pixels);
        if preview {
            preview_workspace = add(mul(pixels, 128)?, mul(pixels, mul(channels, 4)?)?)?;
        }
        if !preview {
            max_pixels = max_pixels.max(pixels);
        }
        // Conservatively retain every preceding layer's f32 colour/extra planes,
        // even if a reference slot is replaced or the codec can free it sooner.
        if !preview {
            retained = add(retained, mul(pixels, mul(channels, 4)?)?)?;
        }
        let entries = frame.num_toc_entries();
        limits::check(entries, policy.toc_entries, "JPEG XL frame TOC entries")?;
        if br.peek(1) != 0 {
            toc_workspace = 8 * 1024 * 1024;
        }
        br.check_for_error().map_err(preflight_error)?;
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
            IncrementalTocReader::new(u32::try_from(entries)?, &mut br).map_err(preflight_error)?;
        while !toc.is_complete() {
            toc.read_step(&mut br).map_err(preflight_error)?;
        }
        let toc = toc.finalize();
        br.jump_to_byte_boundary().map_err(preflight_error)?;
        let payload = toc.entries.iter().try_fold(0, |n, &v| add(n, v as usize))?;
        pos = add(add(pos, br.total_bits_read() / 8)?, payload)?;
        if pos > input.len() {
            return Err(error(
                ImgErrorKind::UnexpectedEof,
                "truncated JPEG XL frame payload",
            ));
        }
        if preview {
            // No preview entropy decode, output allocation, callback or frame
            // publication. The codec also drops this TOC before the main frame.
            preview = false;
            continue;
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
            return Ok((
                peak,
                add(mul(sample_pixels, channels)?, 1)?,
                preview_workspace,
            ));
        }
    }
    Err(unsupported("JPEG XL exceeds64 internal layers"))
}

fn preflight(input: &Input, limits: DecodeLimits) -> Result<Header, Error> {
    preflight_with_resources(input, limits, JxlResourceLimits::legacy())
}
fn preflight_with_resources(
    input: &Input,
    limits: DecodeLimits,
    policy: JxlResourceLimits,
) -> Result<Header, Error> {
    preflight_samples::<0, false>(input, limits, policy)
}

// 0 retains the integer contract; 16/32 select exact floating source layouts.
fn preflight_samples<const FLOAT_BITS: u32, const HDR: bool>(
    input: &Input,
    limits: DecodeLimits,
    policy: JxlResourceLimits,
) -> Result<Header, Error> {
    // FileHeader has bounded channel tables, but does not read ICC/pixel data.
    // Retained container metadata is already live, including expanded brob.
    // Reserve its existing 2x allowance before header/ICC/TOC allocation.
    let header_workspace = add(1024 * 1024, mul(input.metadata_bytes, 2)?)?;
    limits::check(
        header_workspace,
        limits.expanded_bytes,
        "JPEG XL header workspace",
    )?;
    let mut br = BitReader::new(&input.codestream[..input.codestream.len().min(MAX_HEADER)]);
    let header = FileHeader::read(&mut br).map_err(preflight_error)?;
    let m = &header.image_metadata;
    if m.animation.is_some() {
        return Err(unsupported("JPEG XL animations are unsupported"));
    }
    if HDR
        && (m.bit_depth.floating_point_sample()
            || m.bit_depth.bits_per_sample() != 16
            || m.xyb_encoded
            || m.color_encoding.want_icc)
    {
        return Err(unsupported(
            "encoded HDR currently requires structured original-profile integer16",
        ));
    }
    let bits = m.bit_depth.bits_per_sample();
    if FLOAT_BITS != 0
        && (!m.bit_depth.floating_point_sample()
            || bits != FLOAT_BITS
            || m.bit_depth.exponent_bits_per_sample() != if FLOAT_BITS == 16 { 5 } else { 8 }
            || m.xyb_encoded)
    {
        return Err(unsupported(if FLOAT_BITS == 16 {
            "JPEG XL native F16 requires original-profile float samples (16 bits, 5 exponent bits)"
        } else {
            "JPEG XL native F32 requires original-profile float samples (32 bits, 8 exponent bits)"
        }));
    }
    if FLOAT_BITS == 0
        && (m.bit_depth.floating_point_sample() || !matches!(bits, 8 | 9 | 10 | 12 | 16))
    {
        return Err(unsupported(
            "JPEG XL supports integer8/9/10/12/16-bit sources only",
        ));
    }
    if m.extra_channel_info.len() > 1
        || m.extra_channel_info.iter().any(|c| {
            c.ec_type != ExtraChannel::Alpha
                || c.alpha_associated()
                || c.dim_shift() != 0
                || (HDR
                    && (c.bit_depth().floating_point_sample()
                        || c.bit_depth().bits_per_sample() != 16))
                || if FLOAT_BITS != 0 {
                    !c.bit_depth().floating_point_sample()
                        || c.bit_depth().bits_per_sample() != FLOAT_BITS
                        || c.bit_depth().exponent_bits_per_sample()
                            != if FLOAT_BITS == 16 { 5 } else { 8 }
                } else {
                    c.bit_depth().floating_point_sample()
                        || !matches!(c.bit_depth().bits_per_sample(), 8 | 9 | 10 | 12 | 16)
                }
        })
    {
        return Err(unsupported(if HDR {
            "encoded HDR requires at most one full-resolution straight integer16 alpha"
        } else if FLOAT_BITS != 0 {
            if FLOAT_BITS == 16 {
                "JPEG XL F16 requires matching full-resolution straight float alpha (16 bits, 5 exponent bits)"
            } else {
                "JPEG XL F32 requires at most one full-resolution straight float alpha (32 bits, 8 exponent bits)"
            }
        } else {
            "JPEG XL requires at most one full-resolution straight integer alpha channel with 8/9/10/12/16-bit depth"
        }));
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
    use jxl_preflight::headers::color_encoding::{ColorSpace, TransferFunction};
    let hdr = if HDR {
        use jxl_preflight::headers::color_encoding::{Primaries, WhitePoint};
        let c = &m.color_encoding;
        let t = &m.tone_mapping;
        let transfer = match c.tf.transfer_function {
            TransferFunction::PQ => JxlHdrTransfer::Pq,
            TransferFunction::HLG => JxlHdrTransfer::Hlg,
            _ => return Err(unsupported("encoded HDR requires PQ or HLG")),
        };
        if c.color_space != ColorSpace::RGB
            || c.primaries != Primaries::BT2100
            || c.white_point != WhitePoint::D65
        {
            return Err(unsupported(
                "encoded HDR currently requires Rec.2020 RGB with D65",
            ));
        }
        if ![t.intensity_target, t.min_nits, t.linear_below]
            .iter()
            .all(|v| v.is_finite())
            || t.intensity_target <= 0.0
            || t.min_nits < 0.0
            || t.min_nits > t.intensity_target
            || t.linear_below < 0.0
            || (t.relative_to_max_display && t.linear_below > 1.0)
        {
            return Err(invalid("invalid encoded HDR luminance metadata"));
        }
        Some(JxlHdrSignal {
            transfer,
            primaries_xy: [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]],
            white_xy: [0.3127, 0.3290],
            rendering_intent: c.rendering_intent as u8,
            intensity_target: t.intensity_target,
            min_nits: t.min_nits,
            relative_to_max_display: t.relative_to_max_display,
            linear_below: t.linear_below,
        })
    } else {
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
        None
    };
    let dimensions = dimensions::mapped(&header.size, m.orientation, m.intrinsic_size.as_ref());
    let output = dimensions.output.unwrap();
    let (width, height) = (output.width as usize, output.height as usize);
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
    limits::check(
        frame_pixels,
        policy.effective_pixels,
        "JPEG XL padded frame pixels",
    )?;
    let mut base = add(header_workspace, mul(input.codestream.len(), 2)?)?;
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
        icc_reader.skip_bits(header_bits).map_err(preflight_error)?;
        let icc = icc_budget(
            &mut icc_reader,
            limits.metadata_bytes.saturating_sub(meta),
            policy,
        )?;
        frame_bits = icc_reader.total_bits_read();
        base = add(base, mul(icc, 4)?)?;
    }
    limits::check(
        add(base, mul(frame_pixels, 128)?)?,
        limits.expanded_bytes,
        "JPEG XL decode workspace",
    )?;
    let (workspace, sample_limit, preview_workspace) = frame_workspace::<FLOAT_BITS, HDR>(
        &input.codestream,
        &header,
        frame_bits,
        frame_pixels,
        base,
        limits,
        policy,
    )?;
    limits::check(workspace, limits.expanded_bytes, "JPEG XL decode workspace")?;
    Ok(Header {
        dimensions,
        source_intrinsic: m.intrinsic_size.as_ref().map(dimensions::size),
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
        preview_workspace,
        hdr,
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
    copies: &mut Option<resources::MetadataCopies>,
) -> Result<(String, DataMap), Error> {
    // Avoid as_icc(), which panics for unsupported simple colour encodings.
    let icc = match profile {
        JxlColorProfile::Icc(bytes) => {
            validate_icc(bytes, channels)?;
            metadata_copy(key, bytes, copies)?
        }
        JxlColorProfile::Simple(enc) => {
            let icc = enc
                .maybe_create_profile()
                .map_err(codec_error)?
                .ok_or_else(|| {
                    unsupported("JPEG XL output profile cannot be represented as ICC")
                })?;
            // Generation belongs to the pinned codec, under its existing
            // preflight allowance. Record its retained result before subsequent
            // payload copies; this does not claim a fallible codec factory.
            if let Some(copies) = copies {
                copies.claim(key, icc.len())?;
            }
            icc
        }
    };
    validate_icc(&icc, channels)?;
    Ok((key.into(), DataMap::ICCProfile(icc)))
}

fn metadata_copy(
    key: &str,
    payload: &[u8],
    copies: &mut Option<resources::MetadataCopies>,
) -> Result<Vec<u8>, Error> {
    if let Some(copies) = copies {
        copies.claim(key, payload.len())?;
    }
    #[cfg(test)]
    resources::tests::before_metadata_copy(key, payload.len());
    let mut result = Vec::new();
    append(&mut result, payload)?;
    Ok(result)
}

fn metadata_output_profile(
    profile: &JxlColorProfile,
    expand_gray: bool,
) -> Result<std::borrow::Cow<'_, JxlColorProfile>, Error> {
    if !expand_gray {
        return Ok(std::borrow::Cow::Borrowed(profile));
    }
    match profile {
        JxlColorProfile::Simple(JxlColorEncoding::GrayscaleColorSpace {
            white_point,
            transfer_function,
            rendering_intent,
        }) => Ok(std::borrow::Cow::Owned(JxlColorProfile::Simple(
            JxlColorEncoding::RgbColorSpace {
                white_point: white_point.clone(),
                transfer_function: transfer_function.clone(),
                rendering_intent: *rendering_intent,
                primaries: JxlPrimaries::SRGB,
            },
        ))),
        _ => Err(unsupported(
            "JPEG XL Gray ICC expansion to RGBA requires an explicit CMS",
        )),
    }
}

pub(crate) fn decode_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    decode::<B, false, false>(reader, option, false, None, None)
}
#[cfg(feature = "image-buffer-ex")]
pub(crate) fn decode_ex_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    decode::<B, false, false>(reader, option, true, None, None)
}

/// Transfer of unlinearized normalized Rec.2020/D65 RGB code values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlHdrTransfer {
    Pq,
    Hlg,
}
/// Matching source/output color encoding plus verbatim source luminance metadata.
/// No ICC is fabricated; luminance fields are not selected display conditions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JxlHdrSignal {
    pub transfer: JxlHdrTransfer,
    pub primaries_xy: [[f32; 2]; 3],
    pub white_xy: [f32; 2],
    /// 0 perceptual,1 relative colorimetric,2 saturation,3 absolute colorimetric.
    pub rendering_intent: u8,
    pub intensity_target: f32,
    pub min_nits: f32,
    pub relative_to_max_display: bool,
    pub linear_below: f32,
}
/// Oriented encoded HDR RGB code coordinates in0..=1, with linear straight alpha.
/// PQ is absolute-luminance signaling; HLG is relative scene signaling. Neither
/// EOTF/OOTF, display peak/system gamma nor a tone/gamut operator is applied.
#[derive(Debug)]
pub struct JxlEncodedHdrImage {
    pub dimensions: crate::image_info::ImageDimensions,
    pub source_orientation: u8,
    pub rgba_encoded: Vec<f32>,
    pub signal: JxlHdrSignal,
}
/// Preserve structured original-profile Modular integer16 PQ/HLG Rec.2020/D65.
/// Source/output structured descriptors must agree. Embedded ICC, float HDR,
/// arbitrary primaries and display rendering remain unsupported.
pub fn image_hdr_encoded_from_with_resource_limits_and_cancel(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<JxlEncodedHdrImage, Error> {
    let image = image_signal_f32::<0, true>(bytes, limits, policy, cancelled)?;
    Ok(JxlEncodedHdrImage {
        dimensions: image.dimensions,
        source_orientation: image.source_orientation,
        rgba_encoded: image.rgba,
        signal: image
            .hdr
            .ok_or_else(|| invalid("missing encoded HDR signal"))?,
    })
}
fn validate_hdr_profile(
    profile: Option<&JxlColorProfile>,
    signal: &JxlHdrSignal,
) -> Result<(), Error> {
    match profile {
        Some(JxlColorProfile::Simple(JxlColorEncoding::RgbColorSpace {
            white_point: JxlWhitePoint::D65,
            primaries: JxlPrimaries::BT2100,
            transfer_function,
            rendering_intent,
        })) if *rendering_intent as u8 == signal.rendering_intent
            && matches!(
                (signal.transfer, transfer_function),
                (JxlHdrTransfer::Pq, JxlTransferFunction::PQ)
                    | (JxlHdrTransfer::Hlg, JxlTransferFunction::HLG)
            ) =>
        {
            Ok(())
        }
        _ => Err(invalid(
            "encoded HDR public profile disagrees with bounded source signal",
        )),
    }
}

/// Native original-profile SDR samples in F32 storage, with orientation applied.
///
/// RGBA samples are native f32, not normalized integer output. RGB may be
/// negative or above1 but must be finite; straight alpha must be finite in0..=1.
/// Gray coordinates are replicated into RGB when color_channels is1.
/// Profiles describe encoded sample coordinates: F32 storage does not imply
/// linear light. This is not HDR display, tone mapping or an extended-range CMS.
#[derive(Debug)]
pub struct JxlF32Image {
    pub dimensions: crate::image_info::ImageDimensions,
    pub source_orientation: u8,
    pub color_channels: u8,
    pub rgba: Vec<f32>,
    pub source_icc: Vec<u8>,
    pub output_icc: Vec<u8>,
}

/// Decode original-profile Modular32/8 float SDR stills without clipping RGB.
///
/// Explicit companion API; ordinary/EX/sRGB entry points keep their refusals.
/// F16, VarDCT, XYB, PQ/HLG, tone mapping, animation and extra-channel restrictions are
/// unchanged. Errors/cancellation return no image. Samples/ICC storage are
/// admitted cumulatively on top of the existing conservative codec estimate.
pub fn image_f32_from_with_resource_limits_and_cancel(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<JxlF32Image, Error> {
    image_signal_f32::<32, false>(bytes, limits, policy, cancelled).map(SignalF32::into_sdr)
}

/// Decode original-profile Modular F16 input into exact F32 storage.
///
/// The same JxlF32Image shape/ICC/orientation and finite/straight-alpha policy
/// applies. This explicitly promotes IEEE16/5 source samples into F32; it never
/// requests backend F16 output (which is not subnormal-qualified). Existing F32
/// input and ordinary/EX/sRGB APIs retain their source acceptance contracts.
pub fn image_f16_as_f32_from_with_resource_limits_and_cancel(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<JxlF32Image, Error> {
    image_signal_f32::<16, false>(bytes, limits, policy, cancelled).map(SignalF32::into_sdr)
}

// Internal carrier: structured HDR intentionally has no fabricated ICC bytes.
struct SignalF32 {
    dimensions: crate::image_info::ImageDimensions,
    source_orientation: u8,
    color_channels: u8,
    rgba: Vec<f32>,
    source_icc: Vec<u8>,
    output_icc: Vec<u8>,
    hdr: Option<JxlHdrSignal>,
}
impl SignalF32 {
    fn into_sdr(self) -> JxlF32Image {
        JxlF32Image {
            dimensions: self.dimensions,
            source_orientation: self.source_orientation,
            color_channels: self.color_channels,
            rgba: self.rgba,
            source_icc: self.source_icc,
            output_icc: self.output_icc,
        }
    }
}

fn image_signal_f32<const SOURCE_BITS: u32, const HDR: bool>(
    bytes: &[u8],
    limits: DecodeLimits,
    policy: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<SignalF32, Error> {
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
    let h = preflight_samples::<SOURCE_BITS, HDR>(&input, limits, policy)?;
    check()?;
    let pixels = mul(h.width, h.height)?;
    let output_bytes = mul(pixels, 16)?;
    limits::check(
        output_bytes,
        limits.animation_bytes,
        "JPEG XL F32 output bytes",
    )?;
    let mut workspace = resources::Workspace::new(h.workspace, limits.expanded_bytes)?;
    workspace.claim(output_bytes, "JPEG XL F32 output workspace")?;
    if HDR {
        workspace.claim(
            std::mem::size_of::<JxlHdrSignal>(),
            "JPEG XL structured HDR signal workspace",
        )?;
    } else {
        workspace.claim(64 * 1024, "JPEG XL F32 generated profile workspace")?;
    }
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
        .ok_or_else(|| invalid("missing JPEG XL F32 basic info"))?;
    if info.size != (h.width, h.height)
        || info.orientation as u8 != h.orientation
        || if HDR {
            !matches!(
                info.bit_depth,
                JxlBitDepth::Int {
                    bits_per_sample: 16
                }
            )
        } else {
            !matches!(
                info.bit_depth,
                JxlBitDepth::Float {
                    bits_per_sample,
                    exponent_bits_per_sample
                } if bits_per_sample == SOURCE_BITS && exponent_bits_per_sample == if SOURCE_BITS == 16 {5} else {8}
            )
        }
        || !info.uses_original_profile
        || info.animation.is_some()
        || info.extra_channels.len() != usize::from(h.alpha_bits.is_some())
        || info
            .extra_channels
            .iter()
            .any(|a| a.ec_type != jxl_codec::api::ExtraChannel::Alpha || a.alpha_associated)
    {
        return Err(invalid(
            "JPEG XL F32 basic info disagrees with bounded preflight",
        ));
    }
    if let Some(signal) = h.hdr {
        let t = &info.tone_mapping;
        if [t.intensity_target, t.min_nits, t.linear_below]
            != [
                signal.intensity_target,
                signal.min_nits,
                signal.linear_below,
            ]
            || t.relative_to_max_display != signal.relative_to_max_display
        {
            return Err(invalid(
                "encoded HDR tone metadata disagrees with bounded preflight",
            ));
        }
        validate_hdr_profile(decoder.embedded_color_profile(), &signal)?;
    }
    let extras = info.extra_channels.len();
    let format = JxlPixelFormat {
        color_type: JxlColorType::Rgba,
        color_data_format: Some(JxlDataFormat::F32 {
            endianness: Endianness::native(),
        }),
        extra_channel_format: vec![None; extras],
    };
    decoder
        .set_pixel_format(format)
        .map_err(|e| cancellation.codec_error(e))?;
    let (source_icc, output_icc) = if HDR {
        validate_hdr_profile(decoder.output_color_profile(), &h.hdr.unwrap())?;
        // Do not call the HDR ICC factory: its generated LUT can include tone mapping.
        (Vec::new(), Vec::new())
    } else {
        let mut retained_metadata = input.metadata_bytes;
        let mut copy_profile = |profile: Option<&JxlColorProfile>| -> Result<Vec<u8>, Error> {
            check()?;
            let profile = profile
                .ok_or_else(|| invalid("missing JPEG XL F32 profile"))?
                .try_as_icc()
                .ok_or_else(|| unsupported("JPEG XL F32 ICC unavailable"))?;
            validate_icc(&profile, h.color_channels)?;
            let next = add(retained_metadata, profile.len())?;
            limits::check(next, limits.metadata_bytes, "JPEG XL F32 retained metadata")?;
            workspace.claim(profile.len(), "JPEG XL F32 retained profile workspace")?;
            retained_metadata = next;
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(profile.len())?;
            bytes.extend_from_slice(&profile);
            check()?;
            Ok(bytes)
        };
        let source_icc = copy_profile(decoder.embedded_color_profile())?;
        let output_icc = copy_profile(decoder.output_color_profile())?;
        limits::check(
            add(
                add(input.metadata_bytes, source_icc.len())?,
                output_icc.len(),
            )?,
            limits.metadata_bytes,
            "JPEG XL F32 retained metadata",
        )?;
        (source_icc, output_icc)
    };
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::FrameHeader,
    )?;
    check()?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(mul(pixels, 4)?)?;
    rgba.resize(mul(pixels, 4)?, 0.0f32);
    // SAFETY: initialized f32 storage, aligned/exclusive and covering output_bytes.
    let out =
        unsafe { std::slice::from_raw_parts_mut(rgba.as_mut_ptr().cast::<u8>(), output_bytes) };
    let mut buffers = [JxlOutputBuffer::new(out, h.height, mul(h.width, 16)?)];
    expect_event(
        decoder
            .process(&mut stream, Some(&mut buffers), Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::FrameComplete {
            has_more_frames: false,
        },
    )?;
    check()?;
    expect_event(
        decoder
            .process(&mut stream, None, Some(&mut runner))
            .map_err(|e| cancellation.codec_error(e))?,
        Event::Complete,
    )?;
    if decoder.file_length() != Some(input.codestream.len() as u64) {
        return Err(invalid("JPEG XL F32 trailing/incomplete codestream"));
    }
    check()?;
    if HDR {
        validate_hdr_profile(decoder.output_color_profile(), &h.hdr.unwrap())?;
    } else {
        // Observe DATA after completion as well; do not infer it from ORIGINAL.
        let final_profile = decoder
            .output_color_profile()
            .ok_or_else(|| invalid("missing completed JPEG XL F32 profile"))?
            .try_as_icc()
            .ok_or_else(|| unsupported("completed JPEG XL F32 ICC unavailable"))?;
        if final_profile.as_ref() != &output_icc {
            return Err(invalid(
                "JPEG XL F32 output profile changed during decoding",
            ));
        }
        drop(final_profile);
        check()?;
    }
    for chunk in rgba.chunks(256 * 4) {
        check()?;
        validate_f32_samples(chunk)?;
        if HDR
            && chunk
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| !(0.0..=1.0).contains(v)))
        {
            return Err(invalid(
                "encoded HDR RGB must remain normalized code coordinates",
            ));
        }
    }
    check()?;
    Ok(SignalF32 {
        dimensions: h.dimensions,
        source_orientation: h.orientation,
        color_channels: h.color_channels as u8,
        rgba,
        source_icc,
        output_icc,
        hdr: h.hdr,
    })
}

fn validate_f32_samples(rgba: &[f32]) -> Result<(), Error> {
    if rgba
        .chunks_exact(4)
        .any(|p| p.iter().any(|x| !x.is_finite()) || !(0.0..=1.0).contains(&p[3]))
    {
        return Err(invalid(
            "JPEG XL F32 requires finite RGB and straight alpha in0..=1",
        ));
    }
    Ok(())
}

/// Independently returned embedded preview in coded coordinates.
///
/// `rgba16` contains normalized native-endian u16 RGBA samples with straight
/// alpha. RGB uses `icc_profile`; alpha is never color transformed. Orientation
/// is the source EXIF-style value (1..=8), intentionally not applied to samples.
/// No main-image pixels or metadata are substituted into this result.
#[derive(Debug)]
pub struct JxlPreview {
    pub width: u32,
    pub height: u32,
    pub orientation: u8,
    pub rgba16: Vec<u16>,
    pub icc_profile: Vec<u8>,
    /// 1 means Gray coordinates replicated into R/G/B; 3 means RGB coordinates.
    pub color_channels: u8,
}

/// Decode only the independently admitted terminal-visible embedded preview.
///
/// Full existing container/ICC/TOC/layer admission precedes codec allocation.
/// Returns `None` only for an admitted still without a preview. Preview pixels
/// and workspace are additionally budgeted; main pixel entropy is not decoded
/// or validated. This leaves all existing main-image entry points unchanged.
pub fn preview_from_with_resource_limits(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
) -> Result<Option<JxlPreview>, Error> {
    Ok(preview_with_workspace(input, limits, resources, None)?.map(|(preview, _)| preview))
}

fn preview_with_workspace(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
    cancellation: Option<&Cancellation<'_>>,
) -> Result<Option<(JxlPreview, resources::Workspace)>, Error> {
    let check = || cancellation.map_or(Ok(()), Cancellation::check);
    let map_codec_error = |e| {
        if let Some(c) = cancellation {
            c.codec_error(e)
        } else {
            codec_error(e)
        }
    };
    let mut runner = cancellation.map(CancellableRunner);
    check()?;
    resources.validate()?;
    let limits = resources.tighten(limits);
    limits::check(
        input.len(),
        limits.input_bytes.min(MAX_INPUT),
        "JPEG XL input",
    )?;
    limits::check(
        mul(input.len(), 3)?,
        limits.expanded_bytes,
        "JPEG XL input assembly workspace",
    )?;
    let input = unpack_with_resources(input, limits, Some(resources))?;
    check()?;
    let mut h = preflight_with_resources(&input, limits, resources)?;
    check()?;
    let mut br = BitReader::new(&input.codestream[..input.codestream.len().min(MAX_HEADER)]);
    let header = FileHeader::read(&mut br).map_err(preflight_error)?;
    let Some(size) = header.image_metadata.preview.as_ref() else {
        return Ok(None);
    };
    let (width, height) = (size.xsize() as usize, size.ysize() as usize);
    let pixels = mul(width, height)?;
    limits::check(
        pixels,
        limits.pixels.min(MAX_PIXELS),
        "JPEG XL preview output pixels",
    )?;
    let bytes = mul(pixels, 8)?;
    limits::check(
        bytes,
        limits.animation_bytes,
        "JPEG XL preview output bytes",
    )?;
    let mut workspace = resources::Workspace::new(h.workspace, limits.expanded_bytes)?;
    workspace.claim(h.preview_workspace, "JPEG XL preview decode workspace")?;
    workspace.claim(bytes, "JPEG XL preview output workspace")?;
    // Bound canonical simple-profile generation before calling the factory.
    workspace.claim(64 * 1024, "JPEG XL preview generated profile workspace")?;
    drop(header);
    let mut options = JxlDecoderOptions::default();
    options.sample_limit = Some(h.sample_limit);
    options.adjust_orientation = false;
    options.skip_preview = false;
    options.alpha_mode = AlphaMode::AsIs;
    options.high_precision = true;
    options.render_spot_colors = false;
    options.max_profile_level = ProfileLevel::Main5;
    check()?;
    let mut decoder = JxlDecoder::new(options);
    decoder.close_input();
    let mut stream = input.codestream.as_slice();
    expect_event(
        decoder
            .process(
                &mut stream,
                None,
                runner.as_mut().map(|r| r as &mut dyn JxlParallelRunner),
            )
            .map_err(map_codec_error)?,
        Event::BasicInfo,
    )?;
    check()?;
    h.width = h.dimensions.coded.width as usize;
    h.height = h.dimensions.coded.height as usize;
    let info = decoder
        .basic_info()
        .ok_or_else(|| invalid("missing preview basic info"))?;
    validate_basic_info(info, &h)?;
    if info.preview_size != Some((width, height)) {
        return Err(invalid("preview geometry disagrees with bounded preflight"));
    }
    let extras = info.extra_channels.len();
    decoder
        .set_pixel_format(JxlPixelFormat::rgba16(extras))
        .map_err(map_codec_error)?;
    let profile = decoder
        .output_color_profile()
        .ok_or_else(|| invalid("missing preview output profile"))?
        .try_as_icc()
        .ok_or_else(|| unsupported("preview output ICC is unavailable"))?;
    let color_channels = if profile.get(16..20) == Some(b"GRAY".as_slice()) {
        1
    } else {
        3
    };
    validate_icc(&profile, color_channels)?;
    limits::check(
        add(input.metadata_bytes, profile.len())?,
        limits.metadata_bytes,
        "JPEG XL preview retained metadata",
    )?;
    workspace.claim(profile.len(), "JPEG XL preview retained profile workspace")?;
    check()?;
    let mut icc_profile = Vec::new();
    icc_profile.try_reserve_exact(profile.len())?;
    icc_profile.extend_from_slice(&profile);
    drop(profile);
    check()?;
    expect_event(
        decoder
            .process(
                &mut stream,
                None,
                runner.as_mut().map(|r| r as &mut dyn JxlParallelRunner),
            )
            .map_err(map_codec_error)?,
        Event::FrameHeader,
    )?;
    check()?;
    let mut rgba16 = aligned_output(bytes)?;
    // SAFETY: initialized u16 storage is aligned, exclusively borrowed and covers bytes.
    let output = unsafe { std::slice::from_raw_parts_mut(rgba16.as_mut_ptr().cast::<u8>(), bytes) };
    let mut buffers = [JxlOutputBuffer::new(output, height, mul(width, 8)?)];
    expect_event(
        decoder
            .process(
                &mut stream,
                Some(&mut buffers),
                runner.as_mut().map(|r| r as &mut dyn JxlParallelRunner),
            )
            .map_err(map_codec_error)?,
        Event::FrameComplete {
            has_more_frames: true,
        },
    )?;
    check()?;
    Ok(Some((
        JxlPreview {
            width: width as u32,
            height: height as u32,
            orientation: h.orientation,
            rgba16,
            icc_profile,
            color_channels: color_channels as u8,
        },
        workspace,
    )))
}

/// Decode an embedded preview to oriented, straight-alpha sRGB RGBA8.
///
/// Requires `color-management`. Uses the same own-library ICC transform and
/// final16-to8 rounding as main-image sRGB output. Raw preview remains unchanged.
/// Returns None only for an admitted preview-free still. No partial output is
/// published on errors. The conservative raw decode reservation remains charged
/// during CMS and output creation; source samples are converted in place.
#[cfg(feature = "color-management")]
pub fn preview_from_srgb_with_resource_limits(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
) -> Result<Option<crate::draw::ImageBuffer>, Error> {
    preview_display(input, limits, resources, None)
}

/// Cancellable oriented sRGB preview; the probe semantics and DecodeCancelled
/// error are identical to the cancellable main-image sRGB API. Probe boundaries
/// cover admission, codec work groups, CMS batches, orientation rows and output
/// publication. This is cooperative cancellation, not a hard time deadline.
#[cfg(feature = "color-management")]
pub fn preview_from_srgb_with_resource_limits_and_cancel(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<crate::draw::ImageBuffer>, Error> {
    preview_display(input, limits, resources, Some(cancelled))
}

#[cfg(feature = "color-management")]
fn preview_display(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
    probe: Option<&dyn Fn() -> bool>,
) -> Result<Option<crate::draw::ImageBuffer>, Error> {
    let cancellation = Cancellation {
        probe,
        cancelled: std::cell::Cell::new(false),
    };
    let check = || cancellation.check();
    check()?;
    resources.validate()?;
    let limits = resources.tighten(limits);
    limits::scope(limits, || {
        let Some((mut preview, mut workspace)) =
            preview_with_workspace(input, limits, resources, probe.map(|_| &cancellation))?
        else {
            return Ok(None);
        };
        check()?;
        let (w, h) = (preview.width as usize, preview.height as usize);
        let output_bytes = mul(mul(w, h)?, 4)?;
        limits::check(
            output_bytes,
            limits.animation_bytes,
            "JPEG XL managed preview output",
        )?;
        // Both native16 and final oriented8 output are charged while CMS runs,
        // so compiled transforms cannot consume the later output reservation.
        workspace.claim(output_bytes, "JPEG XL managed preview output workspace")?;
        let metadata = add(
            add(
                preview.icc_profile.len(),
                crate::color_convert::SRGB_ICC.len(),
            )?,
            512,
        )?;
        limits::check(
            metadata,
            limits.metadata_bytes,
            "JPEG XL managed preview metadata",
        )?;
        workspace.claim(
            mul(add(crate::color_convert::SRGB_ICC.len(), 512)?, 2)?,
            "JPEG XL managed preview metadata workspace",
        )?;
        let parse = resources::cms_profiles(&preview.icc_profile, crate::color_convert::SRGB_ICC)?;
        let scratch = add(
            2 * 256 * 3 * std::mem::size_of::<f32>(),
            256 * std::mem::size_of::<usize>(),
        )?;
        workspace.claim(parse, "JPEG XL preview CMS profile workspace")?;
        workspace.claim(scratch, "JPEG XL preview CMS batch workspace")?;
        crate::color_convert::rgba16_to_rgb_profile_with_cancel(
            &mut preview.rgba16,
            &preview.icc_profile,
            crate::color_convert::SRGB_ICC,
            preview.color_channels == 1,
            workspace.remaining(),
            &check,
        )?;
        workspace.release(parse)?;
        workspace.release(scratch)?;
        let (ow, oh) = if preview.orientation >= 5 {
            (h, w)
        } else {
            (w, h)
        };
        check()?;
        let mut output = crate::draw::zeroed_bytes(output_bytes)?;
        for y in 0..h {
            check()?;
            for x in 0..w {
                let (dx, dy) = preview_oriented_position(x, y, w, h, preview.orientation);
                for c in 0..4 {
                    output[(dy * ow + dx) * 4 + c] =
                        ((u32::from(preview.rgba16[(y * w + x) * 4 + c]) * 255 + 32767) / 65535)
                            as u8;
                }
            }
        }
        check()?;
        let mut profile = Vec::new();
        profile.try_reserve_exact(crate::color_convert::SRGB_ICC.len())?;
        profile.extend_from_slice(crate::color_convert::SRGB_ICC);
        let mut metadata = std::collections::HashMap::new();
        metadata.try_reserve(4)?;
        metadata.insert("ICC Profile".into(), DataMap::ICCProfile(profile));
        metadata.insert(
            "JPEG XL Reconstruction ICC Profile".into(),
            DataMap::ICCProfile(preview.icc_profile),
        );
        metadata.insert("orientation_applied".into(), DataMap::Bool(true));
        metadata.insert(
            "JPEG XL Source Orientation".into(),
            DataMap::UInt(preview.orientation.into()),
        );
        let mut image = crate::draw::ImageBuffer::from_buffer(ow, oh, output);
        image.metadata = Some(metadata);
        check()?;
        Ok(Some(image))
    })
}

#[cfg(feature = "color-management")]
fn preview_oriented_position(
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    orientation: u8,
) -> (usize, usize) {
    match orientation {
        2 => (w - 1 - x, y),
        3 => (w - 1 - x, h - 1 - y),
        4 => (x, h - 1 - y),
        5 => (y, x),
        6 => (h - 1 - y, x),
        7 => (h - 1 - y, w - 1 - x),
        8 => (y, w - 1 - x),
        _ => (x, y),
    }
}

/// Inspect embedded preview geometry after the same bounded admission as decoding.
///
/// Returns coded and orientation-applied preview dimensions, or `None` when the
/// admitted still has no preview. This does not decode preview pixels, validate
/// pixel entropy, or publish an image. The complete container, ICC size, TOC and
/// internal-layer structure must pass admission, including for preview absence.
/// Input assembly and preflight use finite caller-tightened resource limits.
pub fn preview_dimensions_with_resource_limits(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
) -> Result<Option<crate::image_info::ImageDimensions>, Error> {
    resources.validate()?;
    let limits = resources.tighten(limits);
    limits::check(
        input.len(),
        limits.input_bytes.min(MAX_INPUT),
        "JPEG XL input",
    )?;
    limits::check(
        mul(input.len(), 3)?,
        limits.expanded_bytes,
        "JPEG XL input assembly workspace",
    )?;
    let input = unpack_with_resources(input, limits, Some(resources))?;
    preflight_with_resources(&input, limits, resources)?;
    // Re-read only the bounded header; do not initialize a codec or ICC decoder.
    let mut br = BitReader::new(&input.codestream[..input.codestream.len().min(MAX_HEADER)]);
    let header = FileHeader::read(&mut br).map_err(preflight_error)?;
    Ok(header.image_metadata.preview.as_ref().map(|size| {
        let coded = crate::image_info::ImageSize {
            width: size.xsize(),
            height: size.ysize(),
        };
        let output = if header.image_metadata.orientation.is_transposing() {
            crate::image_info::ImageSize {
                width: coded.height,
                height: coded.width,
            }
        } else {
            coded
        };
        crate::image_info::ImageDimensions {
            coded,
            output: Some(output),
            intrinsic_display: None,
        }
    }))
}

/// Decode a source-coordinate RGBA8 still with caller-tightened JPEG XL limits.
///
/// Resource fields cannot raise fixed shipping caps. The finite workspace
/// budget includes conservative codec admission and cumulative adapter buffers.
/// Errors return no image. This does not establish a global allocator/RSS bound.
pub fn image_from_with_resource_limits(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
) -> Result<crate::draw::ImageBuffer, Error> {
    resources.validate()?;
    let limits = resources.tighten(limits);
    let mut image = crate::draw::ImageBuffer::new();
    crate::decode_guard::run(
        &mut bin_rs::reader::StreamReader::new(std::io::Cursor::new(input)),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
        |reader, options| decode::<_, false, false>(reader, options, false, None, Some(resources)),
    )?;
    Ok(image)
}

/// Explicit sRGB output with additional finite JPEG XL resource limits.
#[cfg(feature = "color-management")]
pub fn image_from_srgb_with_resource_limits(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
) -> Result<crate::draw::ImageBuffer, Error> {
    cancelled_image(input, limits, None, Some(resources))
}

/// Resource-limited sRGB output with the existing cooperative cancellation
/// contract. Pre-cancellation wins over invalid policies and malformed input.
#[cfg(feature = "color-management")]
pub fn image_from_srgb_with_resource_limits_and_cancel(
    input: &[u8],
    limits: DecodeLimits,
    resources: JxlResourceLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<crate::draw::ImageBuffer, Error> {
    cancelled_image(input, limits, Some(cancelled), Some(resources))
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
    cancelled_image(input, limits, None, None)
}

/// Bounded sRGB display decoding with cooperative cancellation.
///
/// `cancelled` returns true to stop; it runs synchronously on the calling thread
/// and should be inexpensive. Cancellation returns [`DecodeCancelled`] in the
/// boxed error. Pre-cancellation wins over input/limit errors; otherwise errors
/// already encountered retain precedence. No image is returned on cancellation.
///
/// Checks surround codec stages, run between jxl 0.7.4 runner tasks, between
/// 256-pixel ICC batches, and before/after private output population. Individual
/// codec tasks, serial codec work, allocation/copies, preflight, and ICC profile
/// parsing/transform compilation are non-interruptible. There is no latency bound.
#[cfg(feature = "color-management")]
pub fn image_from_srgb_with_limits_and_cancel(
    input: &[u8],
    limits: DecodeLimits,
    cancelled: &dyn Fn() -> bool,
) -> Result<crate::draw::ImageBuffer, Error> {
    cancelled_image(input, limits, Some(cancelled), None)
}

#[cfg(feature = "color-management")]
fn cancelled_image(
    input: &[u8],
    limits: DecodeLimits,
    probe: Option<&dyn Fn() -> bool>,
    resources: Option<JxlResourceLimits>,
) -> Result<crate::draw::ImageBuffer, Error> {
    let cancellation = Cancellation {
        probe,
        cancelled: std::cell::Cell::new(false),
    };
    cancellation.check()?;
    let limits = if let Some(resources) = resources {
        resources.validate()?;
        resources.tighten(limits)
    } else {
        limits
    };
    let mut image = crate::draw::ImageBuffer::new();
    crate::decode_guard::run(
        &mut bin_rs::reader::StreamReader::new(std::io::Cursor::new(input)),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
        |reader, options| {
            if probe.is_some() {
                decode::<_, true, true>(reader, options, false, Some(&cancellation), resources)
            } else {
                decode::<_, true, false>(reader, options, false, None, resources)
            }
        },
    )?;
    cancellation.check()?;
    Ok(image)
}

fn decode<B: BinaryReader, const MANAGED: bool, const CANCELLABLE: bool>(
    reader: &mut B,
    option: &mut DecodeOptions,
    native: bool,
    cancellation: Option<&Cancellation<'_>>,
    resources: Option<JxlResourceLimits>,
) -> Result<Option<ImgWarnings>, Error> {
    let check = || {
        if CANCELLABLE {
            cancellation.map_or(Ok(()), Cancellation::check)
        } else {
            Ok(())
        }
    };
    check()?;
    let inactive = Cancellation {
        probe: None,
        cancelled: std::cell::Cell::new(false),
    };
    let cancellation = cancellation.unwrap_or(&inactive);
    let mut runner = CancellableRunner(cancellation);

    // Compile the explicit display path separately so its CMS work cannot
    // obscure optimization of existing source-coordinate entry points. The
    // cancellation specialization also removes probes from existing routes.
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
    let mut input = if resources.is_some() {
        unpack_with_resources(&raw, limits, resources)?
    } else {
        unpack(&raw, limits)?
    };
    drop(raw);
    let h = if let Some(policy) = resources {
        preflight_with_resources(&input, limits, policy)?
    } else {
        preflight(&input, limits)?
    };
    check()?;
    let mut options = JxlDecoderOptions::default();
    options.sample_limit = Some(h.sample_limit);
    options.adjust_orientation = true;
    options.alpha_mode = AlphaMode::AsIs;
    // Preserve 0.7.4's three force_level5_* defaults, including when the
    // independently validated original container carries a jxll=10 label.
    options.max_profile_level = ProfileLevel::Main5;
    options.skip_preview = true; // Explicit: return the main image, never preview pixels.
    options.high_precision = true;
    options.render_spot_colors = false;
    let mut stream = input.codestream.as_slice();
    let mut decoder = JxlDecoder::new(options);
    decoder.close_input();
    expect_event(
        decoder
            .process(
                &mut stream,
                None,
                if CANCELLABLE && cancellation.probe.is_some() {
                    Some(&mut runner)
                } else {
                    None
                },
            )
            .map_err(|e| {
                if CANCELLABLE {
                    cancellation.codec_error(e)
                } else {
                    codec_error(e)
                }
            })?,
        Event::BasicInfo,
    )?;
    check()?;
    validate_basic_info(
        decoder
            .basic_info()
            .ok_or_else(|| invalid("missing JPEG XL basic info"))?,
        &h,
    )?;
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
        extra_channel_format: vec![
            None;
            decoder
                .basic_info()
                .ok_or_else(|| invalid("missing JPEG XL basic info"))?
                .extra_channels
                .len()
        ],
    };
    decoder.set_pixel_format(format).map_err(codec_error)?;
    let mut copies = resources
        .map(|_| resources::MetadataCopies::new(h.workspace, input.metadata_bytes, limits));
    input.metadata.push(profile_metadata(
        "JPEG XL Source ICC Profile",
        decoder
            .embedded_color_profile()
            .ok_or_else(|| invalid("missing JPEG XL source profile"))?,
        h.color_channels,
        &mut copies,
    )?);
    let output_profile = metadata_output_profile(
        decoder
            .output_color_profile()
            .ok_or_else(|| invalid("missing JPEG XL output profile"))?,
        !managed
            && !gray
            && decoder
                .output_color_profile()
                .ok_or_else(|| invalid("missing JPEG XL output profile"))?
                .channels()
                == 1,
    )?;
    let output_key = if managed {
        "JPEG XL Reconstruction ICC Profile"
    } else {
        "ICC Profile"
    };
    let output_channels = if managed {
        output_profile.channels()
    } else if gray {
        1
    } else {
        3
    };
    // Equal simple encodings have deterministic ICC serialization. Retain the
    // validated source envelope, then admit and fallibly copy the output.
    // Raw ICC profiles keep their existing path (no large equality scan).
    let same_simple = matches!((decoder.embedded_color_profile(), output_profile.as_ref()),
        (Some(JxlColorProfile::Simple(source)), JxlColorProfile::Simple(output)) if source == output);
    let output_metadata = if same_simple {
        let Some((_, DataMap::ICCProfile(bytes))) = input.metadata.last() else {
            return Err(invalid("missing JPEG XL source ICC envelope"));
        };
        validate_icc(bytes, output_channels)?;
        (
            output_key.into(),
            DataMap::ICCProfile(metadata_copy(output_key, bytes, &mut copies)?),
        )
    } else {
        profile_metadata(output_key, &output_profile, output_channels, &mut copies)?
    };
    input.metadata.push(output_metadata);
    // End metadata borrows before the next mutable process() call. Only owned,
    // fallibly copied payloads escape into metadata.
    drop(output_profile);
    #[cfg(feature = "color-management")]
    if managed {
        let profile = metadata_copy("ICC Profile", crate::color_convert::SRGB_ICC, &mut copies)?;
        input
            .metadata
            .push(("ICC Profile".into(), DataMap::ICCProfile(profile)));
    }
    for (key, value) in [
        ("coded_width", h.dimensions.coded.width),
        ("coded_height", h.dimensions.coded.height),
        ("output_width", h.width as u32),
        ("output_height", h.height as u32),
    ] {
        input
            .metadata
            .push((key.into(), DataMap::UInt(value.into())));
    }
    if let (Some(source), Some(display)) = (h.source_intrinsic, h.dimensions.intrinsic_display) {
        for (key, value) in [
            ("JPEG XL Source Intrinsic Width", source.width),
            ("JPEG XL Source Intrinsic Height", source.height),
            ("intrinsic_display_width", display.width),
            ("intrinsic_display_height", display.height),
        ] {
            input
                .metadata
                .push((key.into(), DataMap::UInt(value.into())));
        }
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
    let source_header = metadata_copy(
        "JPEG XL Source Header",
        &input.codestream[..h.source_header_bytes],
        &mut copies,
    )?;
    input
        .metadata
        .push(("JPEG XL Source Header".into(), DataMap::Raw(source_header)));
    let metadata_bytes = input.metadata.iter().try_fold(0usize, |n, (k, v)| {
        let bytes = match v {
            DataMap::Raw(b) | DataMap::ICCProfile(b) => b.len(),
            DataMap::Ascii(s) => s.len(),
            _ => 8,
        };
        add(n, add(k.len(), bytes)?)
    })?;
    limits::check(metadata_bytes, limits.metadata_bytes, "JPEG XL metadata")?;
    // Preflight already holds the 2x container allowance. Only generated
    // profiles/header/output metadata are additional at this boundary.
    let added_metadata = metadata_bytes
        .checked_sub(input.metadata_bytes)
        .ok_or_else(|| invalid("JPEG XL metadata accounting mismatch"))?;
    if let Some(copies) = copies {
        copies.finish(added_metadata)?;
    }
    let retained_workspace = add(h.workspace, mul(added_metadata, 2)?)?;
    limits::check(
        retained_workspace,
        limits.expanded_bytes,
        "JPEG XL metadata workspace",
    )?;
    let retained_workspace = structured_metadata::augment(
        &mut input.metadata,
        limits,
        metadata_bytes,
        retained_workspace,
    )?;
    let mut workspace = resources
        .map(|_| resources::Workspace::new(retained_workspace, limits.expanded_bytes))
        .transpose()?;
    expect_event(
        decoder
            .process(
                &mut stream,
                None,
                if CANCELLABLE && cancellation.probe.is_some() {
                    Some(&mut runner)
                } else {
                    None
                },
            )
            .map_err(|e| {
                if CANCELLABLE {
                    cancellation.codec_error(e)
                } else {
                    codec_error(e)
                }
            })?,
        Event::FrameHeader,
    )?;
    check()?;
    let row = mul(h.width, mul(channels, if bits == 8 { 1 } else { 2 })?)?;
    let bytes = mul(row, h.height)?;
    // Vec<u16> guarantees the upstream row alignment even on WASM. All rows
    // have an even byte stride for U16; the U8 path has no alignment requirement.
    if let Some(w) = workspace.as_mut() {
        w.claim(
            mul(bytes.div_ceil(2), 2)?,
            "JPEG XL aligned output workspace",
        )?;
    }
    let mut aligned = aligned_output(bytes)?;
    // SAFETY: u16 has no invalid bit patterns; the exclusively borrowed,
    // initialized allocation covers bytes.len(), and out does not outlive it.
    let out = unsafe { std::slice::from_raw_parts_mut(aligned.as_mut_ptr().cast::<u8>(), bytes) };
    let mut buffers = [JxlOutputBuffer::new(out, h.height, row)];
    expect_event(
        decoder
            .process(
                &mut stream,
                Some(&mut buffers),
                if CANCELLABLE && cancellation.probe.is_some() {
                    Some(&mut runner)
                } else {
                    None
                },
            )
            .map_err(|e| {
                if CANCELLABLE {
                    cancellation.codec_error(e)
                } else {
                    codec_error(e)
                }
            })?,
        Event::FrameComplete {
            has_more_frames: false,
        },
    )?;
    check()?;
    expect_event(
        decoder
            .process(
                &mut stream,
                None,
                if CANCELLABLE && cancellation.probe.is_some() {
                    Some(&mut runner)
                } else {
                    None
                },
            )
            .map_err(|e| {
                if CANCELLABLE {
                    cancellation.codec_error(e)
                } else {
                    codec_error(e)
                }
            })?,
        Event::Complete,
    )?;
    check()?;
    // The codestream parser may buffer ahead; file_length is its authoritative
    // boundary and catches trailing junk/multiple raw streams without guessing.
    if decoder.file_length() != Some(input.codestream.len() as u64) {
        return Err(invalid("trailing or incomplete JPEG XL codestream"));
    }
    let legacy = if !managed && !native && bits != 8 {
        let max = (1u32 << bits) - 1;
        let mut bytes = Vec::new();
        if let Some(w) = workspace.as_mut() {
            w.claim(out.len() / 2, "JPEG XL rounded output workspace")?;
        }
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
        if let Some(w) = workspace.as_mut() {
            w.claim(out.len(), "JPEG XL CMS sample workspace")?;
        }
        rgba.try_reserve_exact(out.len() / 2)?;
        for chunk in out.chunks(256 * 8) {
            check()?;
            rgba.extend(
                chunk
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]])),
            );
        }
        let parse_bytes = if workspace.is_some() {
            resources::cms_profiles(profile, crate::color_convert::SRGB_ICC)?
        } else {
            0
        };
        if let Some(w) = workspace.as_mut() {
            w.claim(parse_bytes, "JPEG XL CMS profile workspace")?;
        }
        let compiled_budget = workspace.as_ref().map_or_else(
            || limits.expanded_bytes.saturating_sub(retained_workspace),
            resources::Workspace::remaining,
        );
        crate::color_convert::rgba16_to_rgb_profile_with_cancel(
            &mut rgba,
            profile,
            crate::color_convert::SRGB_ICC,
            h.color_channels == 1,
            compiled_budget,
            &check,
        )?;
        if let Some(w) = workspace.as_mut() {
            w.release(parse_bytes)?;
        }
        if let Some(w) = workspace.as_mut() {
            w.claim(rgba.len(), "JPEG XL CMS rounded output workspace")?;
        }
        let mut bytes = crate::draw::zeroed_bytes(rgba.len())?;
        for (dst, src) in bytes.chunks_mut(256 * 4).zip(rgba.chunks(256 * 4)) {
            check()?;
            for (dst, &sample) in dst.iter_mut().zip(src) {
                *dst = ((u32::from(sample) * 255 + 32767) / 65535) as u8;
            }
        }
        drop(rgba);
        if let Some(w) = workspace.as_mut() {
            w.release(out.len())?;
        }
        Some(bytes)
    } else {
        None
    };
    #[cfg(not(feature = "color-management"))]
    let managed_output: Option<Vec<u8>> = None;
    check()?;
    if let Some(w) = workspace.as_mut() {
        w.claim(
            mul(mul(h.width, h.height)?, 4)?,
            "JPEG XL collector workspace",
        )?;
    }
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
    check()?;
    Ok(None)
}

// Keep output reservation fallible, including under a real allocator refusal.
// Checked byte conversion is shared by the runtime and allocation-failure test.
fn aligned_output(bytes: usize) -> Result<Vec<u16>, Error> {
    let words = bytes.div_ceil(2);
    mul(words, 2)?;
    let mut output = Vec::new();
    output.try_reserve_exact(words)?;
    output.resize(words, 0);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_protocol_and_preflight_projection_fail_closed() {
        assert!(expect_event(Event::BasicInfo, Event::BasicInfo).is_ok());
        assert!(expect_event(Event::NeedMoreInput { size_hint: 1 }, Event::Complete).is_err());
        assert!(
            expect_event(
                Event::FrameComplete {
                    has_more_frames: true
                },
                Event::FrameComplete {
                    has_more_frames: false
                }
            )
            .is_err()
        );
        let input = include_bytes!("../tests/fixtures/review/jxl-rgb8.jxl");
        let envelope = unpack(input, DecodeLimits::default()).unwrap();
        let h = preflight(&envelope, DecodeLimits::default()).unwrap();
        let mut stream = envelope.codestream.as_slice();
        let mut decoder = JxlDecoder::new(JxlDecoderOptions::default());
        decoder.close_input();
        assert_eq!(
            decoder.process(&mut stream, None, None).unwrap(),
            Event::BasicInfo
        );
        let info = decoder.basic_info().unwrap();
        validate_basic_info(info, &h).unwrap();
        let mut wrong = info.clone();
        wrong.size.0 += 1;
        assert!(validate_basic_info(&wrong, &h).is_err());
        let mut wrong = info.clone();
        wrong.bit_depth = JxlBitDepth::Float {
            bits_per_sample: 16,
            exponent_bits_per_sample: 5,
        };
        assert!(validate_basic_info(&wrong, &h).is_err());
        let mut wrong = info.clone();
        wrong.orientation = Orientation::Rotate180;
        assert!(validate_basic_info(&wrong, &h).is_err());
        let mut wrong = info.clone();
        wrong.extra_channels.push(JxlExtraChannel {
            ec_type: jxl_codec::api::ExtraChannel::Alpha,
            alpha_associated: true,
        });
        assert!(validate_basic_info(&wrong, &h).is_err());
    }

    #[test]
    fn pinned_codec_observes_runner_cancellation_inside_frame_processing() {
        let mut stream = include_bytes!("../tests/fixtures/review/jxl-rgb8.jxl").as_slice();
        let mut decoder = JxlDecoder::new(JxlDecoderOptions::default());
        decoder.close_input();
        assert_eq!(
            decoder.process(&mut stream, None, None).unwrap(),
            Event::BasicInfo
        );
        let (width, height) = decoder.basic_info().unwrap().size;
        decoder.set_pixel_format(JxlPixelFormat::rgba8(0)).unwrap();
        assert_eq!(
            decoder.process(&mut stream, None, None).unwrap(),
            Event::FrameHeader
        );
        let mut output = vec![0; width * height * 4];
        let mut buffers = [JxlOutputBuffer::new(&mut output, height, width * 4)];
        let probe = || true;
        let cancellation = Cancellation {
            probe: Some(&probe),
            cancelled: std::cell::Cell::new(false),
        };
        // This is inside the codec frame call, not an adapter before/after check.
        let result = decoder.process(
            &mut stream,
            Some(&mut buffers),
            Some(&mut CancellableRunner(&cancellation)),
        );
        let err = result.err().expect("codec must propagate runner failure");
        assert!(cancellation.cancelled.get());
        assert!(cancellation.codec_error(err).is::<DecodeCancelled>());
    }

    #[test]
    fn codec_runner_cancels_between_tasks_and_preserves_task_errors() {
        use std::cell::Cell;
        let completed = std::sync::atomic::AtomicUsize::new(0);
        use std::sync::atomic::Ordering::Relaxed;
        let probe = || completed.load(Relaxed) == 1;
        let cancellation = Cancellation {
            probe: Some(&probe),
            cancelled: Cell::new(false),
        };
        let mut runner = CancellableRunner(&cancellation);
        let err = runner
            .run_ordered(10, None, &|_| {
                completed.fetch_add(1, Relaxed);
                Ok(())
            })
            .unwrap_err();
        assert_eq!(completed.load(Relaxed), 1);
        assert!(cancellation.codec_error(err).is::<DecodeCancelled>());
        let probe = || false;
        let cancellation = Cancellation {
            probe: Some(&probe),
            cancelled: Cell::new(false),
        };
        let err = CancellableRunner(&cancellation)
            .run_ordered(10, None, &|_| Err(jxl_codec::error::Error::SectionTooShort))
            .unwrap_err();
        assert!(!cancellation.codec_error(err).is::<DecodeCancelled>());
    }

    #[test]
    fn retained_metadata_is_admitted_before_header_and_icc_work_without_double_counting() {
        let data = include_bytes!("../tests/fixtures/review/jxl-icc-lossless.jxl");
        let limits = DecodeLimits::default();
        let bare = unpack(data, limits).unwrap();
        let baseline = preflight(&bare, limits).unwrap();
        let mut boxed = SIGNATURE.to_vec();
        let mut put = |kind: &[u8; 4], payload: &[u8]| {
            boxed.extend(((payload.len() + 8) as u32).to_be_bytes());
            boxed.extend(kind);
            boxed.extend(payload);
        };
        put(b"ftyp", b"jxl \0\0\0\0jxl ");
        put(b"test", &vec![0; 1 << 20]);
        put(b"jxlc", &bare.codestream);
        let retained = unpack(&boxed, limits).unwrap();
        let admitted = preflight(&retained, limits).unwrap();
        assert_eq!(
            admitted.workspace,
            baseline.workspace + retained.metadata_bytes * 2
        );
        let frame_pixels = (baseline.width.max(16) * baseline.height.max(16) * 4).div_ceil(3);
        let before_icc = (1 << 20) + bare.codestream.len() * 2 + (8 << 20) + frame_pixels * 128;
        let tight = DecodeLimits {
            expanded_bytes: before_icc + retained.metadata_bytes * 2 - 1,
            ..limits
        };
        assert!(preflight(&bare, tight).is_ok());
        assert!(
            preflight(&retained, tight)
                .err()
                .unwrap()
                .to_string()
                .contains("ICC workspace")
        );
        // Public full-decode admission still charges the old total exactly once.
        let image = crate::draw::image_from_with_limits(&boxed, limits).unwrap();
        let final_bytes: usize = image
            .metadata
            .as_ref()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                key.len()
                    + match value {
                        DataMap::Raw(v) | DataMap::ICCProfile(v) => v.len(),
                        DataMap::Ascii(v) => v.len(),
                        _ => 8,
                    }
            })
            .sum();
        let exact = baseline.workspace + final_bytes * 2;
        assert!(
            crate::draw::image_from_with_limits(
                &boxed,
                DecodeLimits {
                    expanded_bytes: exact,
                    ..limits
                }
            )
            .is_ok()
        );
        assert!(
            crate::draw::image_from_with_limits(
                &boxed,
                DecodeLimits {
                    expanded_bytes: exact - 1,
                    ..limits
                }
            )
            .is_err()
        );
    }

    #[test]
    fn brob_metadata_quota_counts_raw_decoded_and_keys_at_the_exact_boundary() {
        let raw = include_bytes!("../tests/fixtures/review/jxl-rgba8.jxl");
        let compressed = include_bytes!("../tests/fixtures/review/jxl-brob-xml.br");
        let xml = b"<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><metadata>JPEG XL bounded metadata</metadata></x:xmpmeta>";
        let mut input = SIGNATURE.to_vec();
        let mut put = |kind: &[u8; 4], payload: &[u8]| {
            input.extend(((payload.len() + 8) as u32).to_be_bytes());
            input.extend(kind);
            input.extend(payload);
        };
        put(b"ftyp", b"jxl \0\0\0\0jxl ");
        put(b"jxlc", raw);
        let mut payload = b"xml ".to_vec();
        payload.extend(compressed);
        put(b"brob", &payload);
        let needed = payload.len()
            + xml.len()
            + box_key("Source", 2, b"brob").len()
            + box_key("Decoded", 2, b"xml ").len();
        let limits = DecodeLimits {
            metadata_bytes: needed,
            ..Default::default()
        };
        assert_eq!(unpack(&input, limits).unwrap().metadata_bytes, needed);
        assert!(
            unpack(
                &input,
                DecodeLimits {
                    metadata_bytes: needed - 1,
                    ..limits
                }
            )
            .err()
            .unwrap()
            .to_string()
            .contains("brob expanded metadata")
        );
    }

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
            icc_budget(&mut actual, 32 << 20, JxlResourceLimits::legacy()).unwrap();
            let mut reader = jxl_preflight::icc::IncrementalIccReader::new(&mut reference).unwrap();
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
