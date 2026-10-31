//! Bounded codestream-prefix reader. Auxiliary payloads and everything following
//! the queried dimension/organization prefix remain opaque and unvalidated.
use super::*;

const MAX_PREFIX: usize = 64;
const MAX_BOXES: usize = 1024;
const SIGNATURE: &[u8; 12] = b"\0\0\0\x0cJXL \r\n\x87\n";

struct Source<'a, B> {
    reader: &'a mut B,
    end: u64,
    boxed: bool,
    remaining: u64,
    final_chunk: bool,
    started: bool,
    partial: bool,
    next: u32,
    boxes: usize,
    level: bool,
}
impl<B: BinaryReader> Source<'_, B> {
    fn next_chunk(&mut self) -> Result<()> {
        if self.final_chunk {
            return invalid("truncated JPEG XL dimension prefix");
        }
        loop {
            if self.boxes >= MAX_BOXES {
                return Err(ImageInfoError::LimitExceeded("JPEG XL prefix boxes"));
            }
            available(self.reader.offset()?, 8, self.end)?;
            let length = self.reader.read_u32_be()?;
            let mut kind = [0; 4];
            self.reader.read_exact(&mut kind)?;
            let (length, header) = match length {
                0 => (
                    self.end
                        .checked_sub(self.reader.offset()? - 8)
                        .ok_or(ImageInfoError::InvalidHeader("JPEG XL box offset"))?,
                    8,
                ),
                1 => {
                    available(self.reader.offset()?, 8, self.end)?;
                    (self.reader.read_u64_be()?, 16)
                }
                _ => (u64::from(length), 8),
            };
            let payload = length
                .checked_sub(header)
                .ok_or(ImageInfoError::InvalidHeader(
                    "JPEG XL box below header size",
                ))?;
            available(self.reader.offset()?, payload, self.end)?;
            let first = self.boxes == 0;
            self.boxes += 1;
            if first && &kind != b"ftyp" {
                return invalid("JPEG XL ftyp must be first");
            }
            match &kind {
                b"ftyp" => {
                    if !first || payload < 12 || payload % 4 != 0 {
                        return invalid("JPEG XL ftyp layout");
                    }
                    let mut brand = [0; 4];
                    self.reader.read_exact(&mut brand)?;
                    if &brand != b"jxl " || self.reader.read_u32_be()? != 0 {
                        return invalid("JPEG XL major brand/version");
                    }
                    let mut remaining = payload - 8;
                    let mut found = false;
                    while remaining != 0 {
                        self.reader.read_exact(&mut brand)?;
                        remaining -= 4;
                        if &brand == b"jxl " {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return invalid("JPEG XL compatible brand missing");
                    }
                    skip(self.reader, remaining, self.end)?;
                }
                b"jxlc" => {
                    if self.started {
                        return invalid("JPEG XL duplicate/mixed codestream");
                    }
                    self.started = true;
                    self.final_chunk = true;
                    self.remaining = payload;
                    return Ok(());
                }
                b"jxlp" => {
                    if payload < 4 || (self.started && !self.partial) {
                        return invalid("JPEG XL partial codestream layout");
                    }
                    let index = self.reader.read_u32_be()?;
                    if index & 0x7fff_ffff != self.next {
                        return invalid("JPEG XL partial index");
                    }
                    self.next = self
                        .next
                        .checked_add(1)
                        .ok_or(ImageInfoError::InvalidHeader(
                            "JPEG XL partial count overflow",
                        ))?;
                    self.started = true;
                    self.partial = true;
                    self.final_chunk = index & 0x8000_0000 != 0;
                    self.remaining = payload - 4;
                    if self.remaining != 0 {
                        return Ok(());
                    }
                    if self.final_chunk {
                        return invalid("JPEG XL final chunk has no queried header");
                    }
                }
                b"jxll" => {
                    if self.started
                        || self.level
                        || payload != 1
                        || !matches!(self.reader.read_byte()?, 5 | 10)
                    {
                        return invalid("JPEG XL level box");
                    }
                    self.level = true;
                }
                b"JXL " => return invalid("JPEG XL duplicate signature"),
                _ => skip(self.reader, payload, self.end)?,
            }
        }
    }
    fn byte(&mut self) -> Result<u8> {
        if self.boxed {
            while self.remaining == 0 {
                self.next_chunk()?;
            }
            self.remaining -= 1;
        }
        available(self.reader.offset()?, 1, self.end)?;
        Ok(self.reader.read_byte()?)
    }
}

pub(super) fn read<B: BinaryReader>(reader: &mut B, end: u64) -> Result<ImageInfo> {
    let mut signature = [0; 2];
    reader.read_exact(&mut signature)?;
    let boxed = signature != [0xff, 0x0a];
    reader.seek(SeekFrom::Start(0))?;
    if boxed {
        let mut actual = [0; 12];
        reader.read_exact(&mut actual)?;
        if &actual != SIGNATURE {
            return invalid("JPEG XL signature");
        }
    }
    let mut source = Source {
        reader,
        end,
        boxed,
        remaining: 0,
        final_chunk: false,
        started: false,
        partial: false,
        next: 0,
        boxes: 0,
        level: false,
    };
    let mut buffer = [0u8; MAX_PREFIX];
    for count in 1..=MAX_PREFIX {
        buffer[count - 1] = source.byte()?;
        let parsed = match crate::jpegxl::dimensions::prefix(&buffer[..count]) {
            Ok(v) => v,
            Err(jxl_codec::error::Error::OutOfBounds(_)) => continue,
            Err(e) => return Err(ImageInfoError::Header(Box::new(e))),
        };
        let mut output = info(
            ImageFormat::JpegXl,
            parsed.dimensions.coded,
            EncodedColor::Unknown,
            (None, None),
            AlphaInfo::Unknown,
            if parsed.animated {
                ImageStructure::Animation {
                    frames: ImageCount::Unknown,
                }
            } else {
                ImageStructure::Still
            },
            InfoScan::HeaderPrefix,
        );
        output.dimensions = parsed.dimensions;
        output.orientation = Some(parsed.orientation);
        output.sample_kind = SampleKind::Unknown;
        return Ok(output);
    }
    Err(ImageInfoError::LimitExceeded("JPEG XL dimension prefix"))
}
