//! Header information without pixel decompression. This is not file validation.
use crate::limits::DecodeLimits;
use crate::util::{ImageFormat, format_check};
use bin_rs::reader::{BinaryReader, StreamReader};
use std::cell::Cell;
use std::fmt;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

/// Positive canvas dimensions in pixels, independent of byte stride/storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}
/// Keep encoded, actual ordinary full-resolution output, and intrinsic display
/// sizes separate. `None` means not obtained, not an implicit equality to coded.
/// Output is the existing ordinary decoder's canvas, before any caller scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageDimensions {
    pub coded: ImageSize,
    pub output: Option<ImageSize>,
    pub intrinsic_display: Option<ImageSize>,
}
/// Encoded color model, not the decoder's expanded RGBA callback format.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodedColor {
    Gray,
    Rgb,
    Indexed,
    Cmyk,
    YCbCr,
    Components,
    Unknown,
}
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaInfo {
    Absent,
    Straight,
    Associated,
    Transparency,
    Unknown,
}
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Unsigned,
    Signed,
    Float,
    Unknown,
}
/// Known counts were traversed; Declared counts came from a header and were
/// not checked against reconstructed frames; Unknown does not imply one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageCount {
    Known(u64),
    Declared(u64),
    Unknown,
}
/// Container organization. Pages/Layers can contain one or an unknown number;
/// Layers does not assert that the document actually has any nonempty layers.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageStructure {
    Still,
    Animation { frames: ImageCount },
    Pages { pages: ImageCount },
    Layers { layers: ImageCount },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoScan {
    HeaderPrefix,
    ContainerScan,
    FirstDirectory,
}
/// Basic info never includes/decompresses ICC, EXIF, XMP or text payloads.
/// Absence of details here does not imply the file has no detailed metadata.
pub struct ImageInfo {
    pub format: ImageFormat,
    pub dimensions: ImageDimensions,
    pub color: EncodedColor,
    pub channels: Option<u16>,
    /// PNG indexed/GIF depths describe indices; other depths are per channel.
    pub bits_per_sample: Option<Vec<u16>>,
    pub sample_kind: SampleKind,
    pub alpha: AlphaInfo,
    pub structure: ImageStructure,
    /// TIFF orientation if obtained. JPEG/PNG EXIF orientation is not parsed.
    pub orientation: Option<u16>,
    pub scan: InfoScan,
    /// Actual reads through the unbuffered counting source; seeks do not count.
    pub bytes_read: u64,
}
impl fmt::Debug for ImageInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let format = match self.format {
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Png => "PNG",
            ImageFormat::Gif => "GIF",
            ImageFormat::Tiff => "TIFF",
            #[cfg(feature = "psd")]
            ImageFormat::Psd => "PSD",
            _ => "unsupported",
        };
        f.debug_struct("ImageInfo")
            .field("format", &format)
            .field("dimensions", &self.dimensions)
            .field("color", &self.color)
            .field("channels", &self.channels)
            .field("bits_per_sample", &self.bits_per_sample)
            .field("sample_kind", &self.sample_kind)
            .field("alpha", &self.alpha)
            .field("structure", &self.structure)
            .field("orientation", &self.orientation)
            .field("scan", &self.scan)
            .field("bytes_read", &self.bytes_read)
            .finish()
    }
}
#[derive(Debug)]
pub enum ImageInfoError {
    UnknownFormat,
    UnsupportedFormat,
    InvalidHeader(&'static str),
    LimitExceeded(&'static str),
    Io(io::Error),
    /// Existing header-parser error, including its own resource/truncation errors.
    Header(Box<dyn std::error::Error>),
}
impl fmt::Display for ImageInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat => f.write_str("unrecognized image format"),
            Self::UnsupportedFormat => {
                f.write_str("information parser unavailable for this format/feature")
            }
            Self::InvalidHeader(s) => write!(f, "invalid image header: {s}"),
            Self::LimitExceeded(s) => write!(f, "image information limit exceeded: {s}"),
            Self::Io(e) => write!(f, "image information I/O: {e}"),
            Self::Header(e) => write!(f, "image header parser: {e}"),
        }
    }
}
impl std::error::Error for ImageInfoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Header(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}
impl From<io::Error> for ImageInfoError {
    fn from(e: io::Error) -> Self {
        if e.get_ref()
            .is_some_and(|error| error.is::<HeaderReadLimit>())
        {
            Self::LimitExceeded("metadata read bytes")
        } else {
            Self::Io(e)
        }
    }
}
impl From<Box<dyn std::error::Error>> for ImageInfoError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        match e.downcast::<io::Error>() {
            Ok(error) => Self::from(*error),
            Err(error) => Self::Header(error),
        }
    }
}
#[derive(Debug)]
struct HeaderReadLimit;
impl fmt::Display for HeaderReadLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("image info metadata read budget exceeded")
    }
}
impl std::error::Error for HeaderReadLimit {}
type Result<T> = std::result::Result<T, ImageInfoError>;
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
fn invalid<T>(s: &'static str) -> Result<T> {
    Err(ImageInfoError::InvalidHeader(s))
}
fn check(n: usize, limit: usize, s: &'static str) -> Result<()> {
    if n > limit {
        Err(ImageInfoError::LimitExceeded(s))
    } else {
        Ok(())
    }
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
fn canvas(width: u32, height: u32) -> Result<ImageSize> {
    if width == 0 || height == 0 {
        invalid("zero canvas size")
    } else {
        Ok(ImageSize { width, height })
    }
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
fn info(
    format: ImageFormat,
    size: ImageSize,
    color: EncodedColor,
    samples: (Option<u16>, Option<Vec<u16>>),
    alpha: AlphaInfo,
    structure: ImageStructure,
    scan: InfoScan,
) -> ImageInfo {
    ImageInfo {
        format,
        dimensions: ImageDimensions {
            coded: size,
            output: Some(size),
            intrinsic_display: None,
        },
        color,
        channels: samples.0,
        bits_per_sample: samples.1,
        sample_kind: SampleKind::Unsigned,
        alpha,
        structure,
        orientation: None,
        scan,
        bytes_read: 0,
    }
}
/// Query a byte slice. Only the supported header/container structure is checked;
/// success does not prove that pixel data is present or decodable.
pub fn image_info(bytes: &[u8]) -> Result<ImageInfo> {
    image_info_with_limits(bytes, DecodeLimits::default())
}
/// Input length and actual header reads use input_bytes/metadata_bytes. The
/// expanded_bytes budget bounds known header buffers, not pixel dimensions.
/// pixels and animation_bytes are deliberately unused; no pixels are expanded.
/// frames bounds traversed/declared image counts. Existing parser limit failures
/// remain available as Header error sources.
pub fn image_info_with_limits(bytes: &[u8], limits: DecodeLimits) -> Result<ImageInfo> {
    image_info_from_reader_with_limits(&mut io::Cursor::new(bytes), limits)
}
/// Query a seekable source from offset zero and attempt to restore its original
/// position on success/error. A prior parse error takes precedence over a restore
/// error. This never reads the entire file into an intermediate Vec.
pub fn image_info_from_reader<R: Read + Seek>(reader: &mut R) -> Result<ImageInfo> {
    image_info_from_reader_with_limits(reader, DecodeLimits::default())
}
/// Reader variant of image_info_with_limits with the same budget contract.
pub fn image_info_from_reader_with_limits<R: Read + Seek>(
    reader: &mut R,
    limits: DecodeLimits,
) -> Result<ImageInfo> {
    let saved = reader.stream_position()?;
    let result = (|| {
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        if limits.input_bytes != usize::MAX && end > limits.input_bytes as u64 {
            return Err(ImageInfoError::LimitExceeded("input bytes"));
        }
        check(4096, limits.expanded_bytes, "basic header workspace")?;
        let count = Cell::new(0u64);
        let source = CountingReader {
            reader,
            count: &count,
            limit: limits.metadata_bytes,
        };
        // One-byte buffering prevents header probes from prefetching raster data.
        let mut r = StreamReader::new(BufReader::with_capacity(1, source));
        let mut prefix = [0; 16];
        let first = usize::try_from(end.min(2)).unwrap();
        r.read_exact(&mut prefix[..first])?;
        let n = if first == 2
            && (prefix[..2] == [0xff, 0xd8]
                || prefix[..2] == *b"BM"
                || prefix[..2] == [0xff, 0x0a]
                || prefix[..2] == [0xff, 0x4f])
        {
            first
        } else {
            usize::try_from(end.min(8)).unwrap()
        };
        if n > first {
            r.read_exact(&mut prefix[first..n])?;
        }
        r.seek(SeekFrom::Start(0))?;
        let extra = if n == 8 && (&prefix[..4] == b"RIFF" || &prefix[..8] == b"\0\0\0\x0cJXL ") {
            12
        } else if n == 8 && &prefix[4..8] == b"ftyp" {
            16
        } else {
            n
        };
        if extra > n && end >= extra as u64 {
            r.seek(SeekFrom::Start(n as u64))?;
            r.read_exact(&mut prefix[n..extra])?;
            r.seek(SeekFrom::Start(0))?;
        }
        #[cfg(not(feature = "psd"))]
        if prefix[..n].starts_with(b"8BPS") {
            return Err(ImageInfoError::UnsupportedFormat);
        }
        let format = format_check(&prefix[..if end >= extra as u64 { extra } else { n }]);
        check(1, limits.frames, "image count")?;
        let mut output: ImageInfo = crate::limits::scope(limits, || match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => jpeg(&mut r, end),
            #[cfg(feature = "png")]
            ImageFormat::Png => png(&mut r, end),
            #[cfg(feature = "gif")]
            ImageFormat::Gif => gif(&mut r, end, limits.frames),
            #[cfg(feature = "tiff")]
            ImageFormat::Tiff => tiff(&mut r),
            #[cfg(feature = "psd")]
            ImageFormat::Psd => psd(&mut r),
            ImageFormat::Unknown => Err(ImageInfoError::UnknownFormat),
            _ => Err(ImageInfoError::UnsupportedFormat),
        })?;
        output.bytes_read = count.get();
        Ok(output)
    })();
    let restored = reader.seek(SeekFrom::Start(saved));
    match (result, restored) {
        (Err(e), _) => Err(e),
        (Ok(_), Err(e)) => Err(e.into()),
        (Ok(v), Ok(_)) => Ok(v),
    }
}
struct CountingReader<'a, R> {
    reader: &'a mut R,
    count: &'a Cell<u64>,
    limit: usize,
}
impl<R: Read> Read for CountingReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let next = self
            .count
            .get()
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::other("image info read count overflow"))?;
        if self.limit != usize::MAX && next > self.limit as u64 {
            return Err(io::Error::other(HeaderReadLimit));
        }
        let n = self.reader.read(buf)?;
        self.count.set(self.count.get() + n as u64);
        Ok(n)
    }
}
impl<R: Seek> Seek for CountingReader<'_, R> {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.reader.seek(p)
    }
}
#[cfg(any(feature = "jpeg", feature = "png", feature = "gif"))]
fn available(offset: u64, length: u64, end: u64) -> Result<()> {
    if offset.checked_add(length).is_none_or(|n| n > end) {
        invalid("declared header payload exceeds input")
    } else {
        Ok(())
    }
}
#[cfg(any(feature = "jpeg", feature = "png", feature = "gif"))]
fn skip<B: BinaryReader>(r: &mut B, n: u64, end: u64) -> Result<()> {
    let current = r.offset()?;
    available(current, n, end)?;
    r.seek(SeekFrom::Start(current + n))?;
    Ok(())
}

#[cfg(feature = "jpeg")]
fn jpeg<B: BinaryReader>(r: &mut B, end: u64) -> Result<ImageInfo> {
    if r.read_u16_be()? != 0xffd8 {
        return invalid("JPEG SOI missing");
    }
    loop {
        if r.read_byte()? != 0xff {
            return invalid("JPEG marker prefix missing");
        }
        let mut marker = r.read_byte()?;
        while marker == 0xff {
            marker = r.read_byte()?;
        }
        if matches!(marker, 0 | 0xd8 | 0xd9 | 0xda | 0xd0..=0xd7) {
            return invalid("JPEG has no supported SOF before scan/end");
        }
        if marker == 1 {
            continue;
        }
        let length = usize::from(r.read_u16_be()?);
        if length < 2 {
            return invalid("JPEG marker length below2");
        }
        let size = length - 2;
        available(r.offset()?, size as u64, end)?;
        if matches!(marker, 0xc0..=0xc2) {
            if size < 6 {
                return invalid("JPEG SOF truncated");
            }
            let mut fixed = [0u8; 6];
            r.read_exact(&mut fixed)?;
            let channels = usize::from(fixed[5]);
            if channels == 0 || channels > 4 || size != 6 + channels * 3 {
                return invalid("JPEG SOF component layout unsupported or malformed");
            }
            if (marker == 0xc0 && fixed[0] != 8) || !matches!(fixed[0], 8 | 12) {
                return invalid("JPEG SOF precision invalid");
            }
            let mut data = Vec::with_capacity(size);
            data.extend(fixed);
            data.resize(size, 0);
            r.read_exact(&mut data[6..])?;
            let mut ids = [false; 256];
            for c in data[6..].chunks_exact(3) {
                if ids[usize::from(c[0])]
                    || !(1..=4).contains(&(c[1] >> 4))
                    || !(1..=4).contains(&(c[1] & 15))
                    || c[2] > 3
                {
                    return invalid(
                        "JPEG SOF component identifiers/sampling/quantization selector invalid",
                    );
                }
                ids[usize::from(c[0])] = true;
            }
            let header = crate::jpeg::header::FrameHeader::new(usize::from(marker & 15), &data);
            if header.height == 0 {
                return Err(ImageInfoError::UnsupportedFormat);
            }
            let size = canvas(header.width as u32, header.height as u32)?;
            return Ok(info(
                ImageFormat::Jpeg,
                size,
                if channels == 1 {
                    EncodedColor::Gray
                } else {
                    EncodedColor::Components
                },
                (
                    Some(channels as u16),
                    Some(vec![header.bitperpixel as u16; channels]),
                ),
                AlphaInfo::Absent,
                ImageStructure::Still,
                InfoScan::HeaderPrefix,
            ));
        }
        if matches!(marker,0xc3|0xc5..=0xc7|0xc9..=0xcb|0xcd..=0xcf|0xde|0xdc) {
            return Err(ImageInfoError::UnsupportedFormat);
        }
        // APP/COM/table payloads are not parsed, cloned or inflated.
        skip(r, size as u64, end)?;
    }
}

#[cfg(feature = "png")]
fn png<B: BinaryReader>(r: &mut B, end: u64) -> Result<ImageInfo> {
    use crate::png::utils::CRC32;
    let mut signature = [0; 8];
    r.read_exact(&mut signature)?;
    if signature != crate::png::header::SIGNATURE {
        return invalid("PNG signature");
    }
    let mut fixed = [0; 25];
    r.read_exact(&mut fixed)?;
    if u32::from_be_bytes(fixed[..4].try_into().unwrap()) != 13 || &fixed[4..8] != b"IHDR" {
        return invalid("PNG must start with a13-byte IHDR");
    }
    let crc = CRC32::new();
    if crc.crc32(&fixed[4..21]) != u32::from_be_bytes(fixed[21..25].try_into().unwrap()) {
        return invalid("PNG IHDR CRC");
    }
    let width = u32::from_be_bytes(fixed[8..12].try_into().unwrap());
    let height = u32::from_be_bytes(fixed[12..16].try_into().unwrap());
    let size = canvas(width, height)?;
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return invalid("PNG dimensions exceed spec");
    }
    let depth = fixed[16];
    let color = fixed[17];
    crate::png::header::validate_color_depth(color, depth)?;
    if fixed[18] != 0 || fixed[19] != 0 || fixed[20] > 1 {
        return invalid("PNG compression/filter/interlace method");
    }
    let (model, channels, mut alpha) = match color {
        0 => (EncodedColor::Gray, 1, AlphaInfo::Absent),
        2 => (EncodedColor::Rgb, 3, AlphaInfo::Absent),
        3 => (EncodedColor::Indexed, 1, AlphaInfo::Absent),
        4 => (EncodedColor::Gray, 2, AlphaInfo::Straight),
        6 => (EncodedColor::Rgb, 4, AlphaInfo::Straight),
        _ => unreachable!(),
    };
    let mut palette = 0u32;
    let mut transparency = false;
    let mut animation = None;
    loop {
        let length = r.read_u32_be()?;
        let mut tag = [0; 4];
        r.read_exact(&mut tag)?;
        if length > i32::MAX as u32
            || tag.iter().any(|b| !b.is_ascii_alphabetic())
            || !tag[2].is_ascii_uppercase()
        {
            return invalid("PNG chunk header");
        }
        available(r.offset()?, u64::from(length) + 4, end)?;
        match &tag {
            b"IDAT" => {
                if color == 3 && palette == 0 {
                    return invalid("PNG indexed data has no palette");
                }
                break;
            }
            b"IHDR" | b"IEND" => return invalid("PNG duplicate IHDR or IEND before data"),
            b"PLTE" => {
                if palette != 0
                    || length == 0
                    || length > 768
                    || length % 3 != 0
                    || matches!(color, 0 | 4)
                    || (color == 3 && length / 3 > (1u32 << depth))
                {
                    return invalid("PNG palette layout");
                }
                palette = length / 3;
                skip(r, u64::from(length) + 4, end)?;
            }
            b"tRNS" | b"acTL" => {
                if (&tag == b"acTL" && (animation.is_some() || length != 8))
                    || (&tag == b"tRNS"
                        && (transparency
                            || match color {
                                0 => length != 2,
                                2 => length != 6,
                                3 => palette == 0 || length == 0 || length > palette,
                                _ => true,
                            }))
                {
                    return invalid("PNG animation/transparency header");
                }
                let mut data = [0; 264];
                data[..4].copy_from_slice(&tag);
                r.read_exact(&mut data[4..4 + length as usize])?;
                let stored = r.read_u32_be()?;
                if crc.crc32(&data[..4 + length as usize]) != stored {
                    return invalid("PNG selected chunk CRC");
                }
                if &tag == b"tRNS" {
                    transparency = true;
                    alpha = AlphaInfo::Transparency;
                } else {
                    let frames = u32::from_be_bytes(data[4..8].try_into().unwrap());
                    if frames == 0 {
                        return invalid("PNG animation declares zero frames");
                    }
                    check(
                        frames as usize,
                        crate::limits::current().frames,
                        "declared PNG frames",
                    )?;
                    animation = Some(u64::from(frames));
                }
            }
            _ => {
                if tag[0].is_ascii_uppercase() {
                    return Err(ImageInfoError::UnsupportedFormat);
                }
                skip(r, u64::from(length) + 4, end)?;
            }
        }
    }
    Ok(info(
        ImageFormat::Png,
        size,
        model,
        (
            Some(channels),
            Some(vec![u16::from(depth); channels as usize]),
        ),
        alpha,
        animation.map_or(ImageStructure::Still, |n| ImageStructure::Animation {
            frames: ImageCount::Declared(n),
        }),
        InfoScan::HeaderPrefix,
    ))
}

#[cfg(feature = "gif")]
fn gif<B: BinaryReader>(r: &mut B, end: u64, max_frames: usize) -> Result<ImageInfo> {
    use crate::gif::header::{GifHeader, GifLscd};
    let header = GifHeader::new(r, 0)?;
    let size = canvas(header.width as u32, header.height as u32)?;
    let mut frames = 0u64;
    let mut pending_transparency = None;
    let mut has_transparency = false;
    let mut first_depth = None;
    let mut plain_text = false;
    fn subblocks<B: BinaryReader>(r: &mut B, end: u64) -> Result<()> {
        loop {
            let n = r.read_byte()?;
            if n == 0 {
                break;
            }
            skip(r, u64::from(n), end)?;
        }
        Ok(())
    }
    loop {
        match r.read_byte()? {
            0x3b => break,
            0x21 => {
                let kind = r.read_byte()?;
                if kind == 0xf9 {
                    if r.read_byte()? != 4 {
                        return invalid("GIF graphic control length");
                    }
                    let mut gce = [0; 4];
                    r.read_exact(&mut gce)?;
                    if r.read_byte()? != 0 {
                        return invalid("GIF graphic control terminator");
                    }
                    if gce[0] & 0xe0 != 0 || (gce[0] >> 2) & 7 > 3 {
                        return invalid("GIF graphic control flags");
                    }
                    pending_transparency = if gce[0] & 1 != 0 { Some(gce[3]) } else { None };
                } else {
                    if matches!(kind, 1 | 0xff) {
                        let fixed = r.read_byte()?;
                        if fixed != if kind == 1 { 12 } else { 11 } {
                            return invalid("GIF text/application fixed header size");
                        }
                        skip(r, u64::from(fixed), end)?;
                    }
                    subblocks(r, end)?;
                    if kind == 1 {
                        plain_text = true;
                        pending_transparency = None;
                    }
                }
            }
            0x2c => {
                let frame = GifLscd::new(r)?;
                if frame.xsize == 0 || frame.ysize == 0 {
                    return invalid("GIF empty frame");
                }
                if frame.field & 0x18 != 0
                    || u32::from(frame.xstart) + u32::from(frame.xsize) > size.width
                    || u32::from(frame.ystart) + u32::from(frame.ysize) > size.height
                {
                    return invalid("GIF frame rectangle/flags");
                }
                frames = frames
                    .checked_add(1)
                    .ok_or(ImageInfoError::LimitExceeded("GIF frame count overflow"))?;
                check(
                    usize::try_from(frames)
                        .map_err(|_| ImageInfoError::LimitExceeded("GIF frame count"))?,
                    max_frames,
                    "GIF frames",
                )?;
                let depth = if frame.field & 0x80 != 0 {
                    (frame.field & 7) + 1
                } else {
                    (header.scd.field & 7) + 1
                };
                if frame.field & 0x80 != 0 {
                    skip(r, 3u64 << depth, end)?;
                } else if header.scd.field & 0x80 == 0 {
                    return invalid("GIF frame has no palette");
                }
                if pending_transparency.is_some_and(|index| u16::from(index) >= (1u16 << depth)) {
                    return invalid("GIF transparent index exceeds palette");
                }
                first_depth = Some(first_depth.map_or(depth, |old: u8| old.max(depth)));
                let code = r.read_byte()?;
                if !(2..=8).contains(&code) {
                    return invalid("GIF LZW minimum code size");
                }
                subblocks(r, end)?;
                has_transparency |= pending_transparency.is_some();
                pending_transparency = None;
            }
            _ => return invalid("GIF block introducer"),
        }
    }
    if frames == 0 {
        return if plain_text {
            Err(ImageInfoError::UnsupportedFormat)
        } else {
            invalid("GIF has no image blocks")
        };
    }
    // Palette precision may vary by frame. Do not label the maximum as uniform.
    Ok(info(
        ImageFormat::Gif,
        size,
        EncodedColor::Indexed,
        (
            Some(1),
            if frames == 1 {
                Some(vec![u16::from(first_depth.unwrap())])
            } else {
                None
            },
        ),
        if plain_text {
            AlphaInfo::Unknown
        } else if has_transparency {
            AlphaInfo::Transparency
        } else {
            AlphaInfo::Absent
        },
        if plain_text {
            ImageStructure::Animation {
                frames: ImageCount::Unknown,
            }
        } else if frames == 1 {
            ImageStructure::Still
        } else {
            ImageStructure::Animation {
                frames: ImageCount::Known(frames),
            }
        },
        InfoScan::ContainerScan,
    ))
}

#[cfg(feature = "psd")]
fn psd<B: BinaryReader>(r: &mut B) -> Result<ImageInfo> {
    let mut bytes = [0; 26];
    r.read_exact(&mut bytes)?;
    let h = crate::psd::decoder::basic_header(&bytes)?;
    let model = match h.mode {
        1 => EncodedColor::Gray,
        2 => EncodedColor::Indexed,
        3 => EncodedColor::Rgb,
        4 => EncodedColor::Cmyk,
        _ => EncodedColor::Unknown,
    };
    Ok(info(
        ImageFormat::Psd,
        canvas(h.width as u32, h.height as u32)?,
        model,
        (Some(h.channels as u16), Some(vec![h.depth; h.channels])),
        AlphaInfo::Unknown,
        ImageStructure::Layers {
            layers: ImageCount::Unknown,
        },
        InfoScan::HeaderPrefix,
    ))
}

#[cfg(feature = "tiff")]
fn tiff<B: BinaryReader>(r: &mut B) -> Result<ImageInfo> {
    let document = crate::tiff::ifd::TiffDocument::read_basic(r)?;
    let first = document
        .ifds
        .first()
        .ok_or(ImageInfoError::InvalidHeader("TIFF has no IFD"))?;
    for entry in &first.entries {
        let valid = if matches!(entry.tag, 256 | 257) {
            matches!(entry.type_id, 3 | 4)
                || (document.variant == crate::tiff::ifd::TiffVariant::BigTiff
                    && entry.type_id == 16)
        } else {
            entry.type_id == 3
        };
        if !valid {
            return invalid("TIFF basic tag has an invalid field type");
        }
    }

    let values = |tag: u16| -> Result<Option<Vec<u64>>> {
        first
            .entries
            .iter()
            .find(|e| e.tag == tag)
            .map(|e| e.unsigned(document.endian).map_err(ImageInfoError::Header))
            .transpose()
    };
    let scalar = |tag, default: Option<u64>| -> Result<Option<u64>> {
        match values(tag)? {
            None => Ok(default),
            Some(v) if v.len() == 1 => Ok(Some(v[0])),
            _ => invalid("TIFF scalar tag count"),
        }
    };
    let width = scalar(256, None)?.ok_or(ImageInfoError::InvalidHeader("TIFF width missing"))?;
    let height = scalar(257, None)?.ok_or(ImageInfoError::InvalidHeader("TIFF height missing"))?;
    let size = canvas(
        u32::try_from(width)
            .map_err(|_| ImageInfoError::InvalidHeader("TIFF width exceeds u32"))?,
        u32::try_from(height)
            .map_err(|_| ImageInfoError::InvalidHeader("TIFF height exceeds u32"))?,
    )?;
    let channels = scalar(277, Some(1))?.unwrap();
    if channels == 0 || channels > 64 {
        return invalid("TIFF sample count outside information scope1..64");
    }
    let mut bits = values(258)?.unwrap_or_else(|| vec![1; channels as usize]);
    if bits.len() == 1 && channels > 1 {
        bits = vec![bits[0]; channels as usize];
    }
    if bits.len() != channels as usize || bits.iter().any(|v| *v == 0 || *v > u64::from(u16::MAX)) {
        return invalid("TIFF bits-per-sample layout");
    }
    let mut sample = values(339)?.unwrap_or_else(|| vec![1; channels as usize]);
    if sample.len() == 1 && channels > 1 {
        sample = vec![sample[0]; channels as usize];
    }
    if sample.len() != channels as usize {
        return invalid("TIFF sample-format layout");
    }
    let sample_kind = if sample.iter().all(|v| *v == 1) {
        SampleKind::Unsigned
    } else if sample.iter().all(|v| *v == 2) {
        SampleKind::Signed
    } else if sample.iter().all(|v| *v == 3) {
        SampleKind::Float
    } else {
        SampleKind::Unknown
    };
    let photo = scalar(262, None)?;
    let ink_set = if photo == Some(5) {
        let set = scalar(332, Some(1))?.unwrap();
        if !matches!(set, 1 | 2) {
            return invalid("TIFF InkSet invalid");
        }
        if set == 1 && scalar(334, Some(4))? != Some(4) {
            return invalid("TIFF CMYK NumberOfInks invalid");
        }
        Some(set)
    } else {
        None
    };
    let (color, base) = match photo {
        Some(0 | 1) => (EncodedColor::Gray, Some(1)),
        Some(2) => (EncodedColor::Rgb, Some(3)),
        Some(3) => (EncodedColor::Indexed, Some(1)),
        Some(5) if ink_set == Some(1) => (EncodedColor::Cmyk, Some(4)),
        Some(5) => (EncodedColor::Components, None),
        Some(6) => (EncodedColor::YCbCr, Some(3)),
        _ => (EncodedColor::Unknown, None),
    };
    let extras = values(338)?;
    let alpha = if let Some(extra) = extras {
        if base.is_some_and(|b| channels < b || extra.len() != (channels - b) as usize)
            || extra.len() > channels as usize
            || extra.iter().any(|v| *v > 2)
        {
            return invalid("TIFF extra-sample layout");
        }
        if extra.iter().filter(|v| **v != 0).count() > 1 {
            AlphaInfo::Unknown
        } else if extra.contains(&1) {
            AlphaInfo::Associated
        } else if extra.contains(&2) {
            AlphaInfo::Straight
        } else if extra.is_empty() {
            AlphaInfo::Absent
        } else {
            AlphaInfo::Unknown
        }
    } else if base == Some(channels) {
        AlphaInfo::Absent
    } else {
        AlphaInfo::Unknown
    };
    if base.is_some_and(|b| channels < b) {
        return invalid("TIFF fewer samples than color model");
    }
    let orientation = scalar(274, Some(1))?.unwrap();
    if !(1..=8).contains(&orientation) {
        return invalid("TIFF orientation invalid");
    }
    let mut output = info(
        ImageFormat::Tiff,
        size,
        color,
        (
            Some(channels as u16),
            Some(bits.into_iter().map(|v| v as u16).collect()),
        ),
        alpha,
        ImageStructure::Pages {
            pages: if first.next_ifd.is_none() {
                ImageCount::Known(1)
            } else {
                ImageCount::Unknown
            },
        },
        InfoScan::FirstDirectory,
    );
    // The ordinary TIFF route can skip thumbnail/mask IFDs and select a later
    // primary page. A first-directory probe cannot promise that output canvas.
    output.dimensions.output = None;
    output.orientation = Some(orientation as u16);
    output.sample_kind = sample_kind;
    Ok(output)
}
