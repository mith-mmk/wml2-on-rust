//! Bounded Classic TIFF and BigTIFF directory reader.
use super::header::{DataPack, Rational, SRational, TiffHeader, TiffHeaders};
use crate::error::{ImgError, ImgErrorKind};
use bin_rs::{Endian, reader::BinaryReader};
use std::{collections::HashSet, io::SeekFrom};

type Error = Box<dyn std::error::Error>;
pub(crate) fn invalid(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::IllegalData,
        message.into(),
    ))
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TiffVariant {
    Classic,
    BigTiff,
}
#[derive(Debug)]
pub(crate) struct TiffEntry {
    pub tag: u16,
    pub type_id: u16,
    pub count: u64,
    pub payload: Vec<u8>,
}
impl TiffEntry {
    pub(crate) fn unsigned(&self, endian: Endian) -> Result<Vec<u64>, Error> {
        let size = match self.type_id {
            1 => 1,
            3 => 2,
            4 | 13 => 4,
            16 | 18 => 8,
            _ => return Err(invalid("TIFF tag requires unsigned integer values")),
        };
        Ok(self
            .payload
            .chunks_exact(size)
            .map(|s| number(s, endian))
            .collect())
    }
    fn header(&self, endian: Endian) -> Result<TiffHeader, Error> {
        let p = &self.payload;
        let data = match self.type_id {
            1 => DataPack::Bytes(p.clone()),
            2 => DataPack::Ascii(
                String::from_utf8_lossy(p)
                    .trim_end_matches('\0')
                    .to_string(),
            ),
            3 => DataPack::Short(
                p.chunks_exact(2)
                    .map(|s| number(s, endian) as u16)
                    .collect(),
            ),
            4 | 13 => DataPack::Long(
                p.chunks_exact(4)
                    .map(|s| number(s, endian) as u32)
                    .collect(),
            ),
            5 => DataPack::Rational(
                p.chunks_exact(8)
                    .map(|s| Rational {
                        n: number(&s[..4], endian) as u32,
                        d: number(&s[4..], endian) as u32,
                    })
                    .collect(),
            ),
            6 => DataPack::SByte(p.iter().map(|v| *v as i8).collect()),
            7 => DataPack::Undef(p.clone()),
            8 => DataPack::SShort(
                p.chunks_exact(2)
                    .map(|s| number(s, endian) as i16)
                    .collect(),
            ),
            9 => DataPack::SLong(
                p.chunks_exact(4)
                    .map(|s| number(s, endian) as i32)
                    .collect(),
            ),
            10 => DataPack::SRational(
                p.chunks_exact(8)
                    .map(|s| SRational {
                        n: number(&s[..4], endian) as i32,
                        d: number(&s[4..], endian) as i32,
                    })
                    .collect(),
            ),
            11 => DataPack::Float(
                p.chunks_exact(4)
                    .map(|s| f32::from_bits(number(s, endian) as u32))
                    .collect(),
            ),
            12 => DataPack::Double(
                p.chunks_exact(8)
                    .map(|s| f64::from_bits(number(s, endian)))
                    .collect(),
            ),
            // The public Classic EXIF enum is retained. Wide values remain lossless
            // bytes here, while page interpretation uses the typed internal entry.
            16..=18 => DataPack::Undef(p.clone()),
            _ => return Err(invalid("Unknown TIFF field type")),
        };
        let length = if matches!(self.type_id, 16..=18) {
            p.len()
        } else {
            usize::try_from(self.count)?
        };
        Ok(TiffHeader {
            tagid: usize::from(self.tag),
            data,
            length,
        })
    }
}
#[derive(Debug)]
pub(crate) struct TiffIfd {
    pub entries: Vec<TiffEntry>,
    pub next_ifd: Option<u64>,
    pub exif: Option<Vec<TiffHeader>>,
    pub gps: Option<Vec<TiffHeader>>,
}
impl TiffIfd {
    pub(crate) fn headers(
        &self,
        variant: TiffVariant,
        endian: Endian,
    ) -> Result<TiffHeaders, Error> {
        Ok(TiffHeaders {
            version: if variant == TiffVariant::Classic {
                42
            } else {
                43
            },
            endian,
            headers: self
                .entries
                .iter()
                .map(|e| e.header(endian))
                .collect::<Result<_, _>>()?,
            exif: self.exif.clone(),
            gps: self.gps.clone(),
        })
    }
}
#[derive(Debug)]
pub(crate) struct TiffDocument {
    pub variant: TiffVariant,
    pub endian: Endian,
    pub ifds: Vec<TiffIfd>,
}
pub(crate) fn number(bytes: &[u8], endian: Endian) -> u64 {
    if endian == Endian::LittleEndian {
        bytes.iter().rev().fold(0, |v, b| (v << 8) | u64::from(*b))
    } else {
        bytes.iter().fold(0, |v, b| (v << 8) | u64::from(*b))
    }
}
fn range(offset: u64, length: u64, end: u64) -> Result<(), Error> {
    if offset.checked_add(length).is_none_or(|v| v > end) {
        return Err(invalid("TIFF offset or length lies outside input"));
    }
    Ok(())
}
struct Parser<'a> {
    reader: &'a mut dyn BinaryReader,
    variant: TiffVariant,
    end: u64,
    metadata: usize,
}
impl Parser<'_> {
    fn charge(&mut self, size: usize) -> Result<(), Error> {
        self.metadata = self
            .metadata
            .checked_add(size)
            .ok_or_else(|| invalid("TIFF metadata size overflow"))?;
        crate::limits::check(
            self.metadata,
            crate::limits::current().metadata_bytes,
            "TIFF metadata",
        )
    }
    fn read_classic_ifd(&mut self, offset: u64) -> Result<TiffIfd, Error> {
        self.directory(offset, false)
    }
    fn read_bigtiff_ifd(&mut self, offset: u64) -> Result<TiffIfd, Error> {
        self.directory(offset, true)
    }
    fn directory(&mut self, offset: u64, big: bool) -> Result<TiffIfd, Error> {
        let (prefix, entry_size, slot) = if big {
            (8u64, 20u64, 8usize)
        } else {
            (2, 12, 4)
        };
        range(offset, prefix, self.end)?;
        self.reader.seek(SeekFrom::Start(offset))?;
        let count = if big {
            self.reader.read_u64()?
        } else {
            u64::from(self.reader.read_u16()?)
        };
        let bytes = count
            .checked_mul(entry_size)
            .and_then(|v| v.checked_add(slot as u64))
            .ok_or_else(|| invalid("TIFF directory size overflow"))?;
        range(
            offset
                .checked_add(prefix)
                .ok_or_else(|| invalid("TIFF offset overflow"))?,
            bytes,
            self.end,
        )?;
        let count = usize::try_from(count).map_err(|_| invalid("TIFF directory count overflow"))?;
        self.charge(
            count
                .checked_mul(std::mem::size_of::<TiffEntry>())
                .ok_or_else(|| invalid("TIFF directory allocation overflow"))?,
        )?;
        let mut entries = Vec::new();
        entries.try_reserve_exact(count)?;
        let mut tags = HashSet::new();
        for _ in 0..count {
            let tag = self.reader.read_u16()?;
            if !tags.insert(tag) {
                return Err(invalid("Duplicate TIFF tag within IFD"));
            }
            let type_id = self.reader.read_u16()?;
            let count = if big {
                self.reader.read_u64()?
            } else {
                u64::from(self.reader.read_u32()?)
            };
            let width: u64 = match type_id {
                1 | 2 | 6 | 7 => 1,
                3 | 8 => 2,
                4 | 9 | 11 | 13 => 4,
                5 | 10 | 12 | 16 | 17 | 18 => 8,
                _ => {
                    return Err(Box::new(ImgError::new_const(
                        ImgErrorKind::NoSupportFormat,
                        "Unsupported TIFF field type".into(),
                    )));
                }
            };
            let length = count
                .checked_mul(width)
                .ok_or_else(|| invalid("TIFF field count overflow"))?;
            let length =
                usize::try_from(length).map_err(|_| invalid("TIFF field size overflow"))?;
            self.charge(length)?;
            let mut inline = [0u8; 8];
            self.reader.read_exact(&mut inline[..slot])?;
            let payload = if length <= slot {
                inline[..length].to_vec()
            } else {
                let address = number(&inline[..slot], self.reader.endian());
                range(address, u64::try_from(length)?, self.end)?;
                let saved = self.reader.offset()?;
                self.reader.seek(SeekFrom::Start(address))?;
                let mut data = Vec::new();
                data.try_reserve_exact(length)?;
                data.resize(length, 0);
                self.reader.read_exact(&mut data)?;
                self.reader.seek(SeekFrom::Start(saved))?;
                data
            };
            entries.push(TiffEntry {
                tag,
                type_id,
                count,
                payload,
            });
        }
        let next = if big {
            self.reader.read_u64()?
        } else {
            u64::from(self.reader.read_u32()?)
        };
        Ok(TiffIfd {
            entries,
            next_ifd: if next == 0 { None } else { Some(next) },
            exif: None,
            gps: None,
        })
    }
    fn directory_at(&mut self, offset: u64) -> Result<TiffIfd, Error> {
        match self.variant {
            TiffVariant::Classic => self.read_classic_ifd(offset),
            TiffVariant::BigTiff => self.read_bigtiff_ifd(offset),
        }
    }
}
impl TiffDocument {
    pub(crate) fn read(reader: &mut dyn BinaryReader) -> Result<Self, Error> {
        let saved = reader.offset()?;
        let end = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(saved))?;
        crate::limits::check_input_length(end, crate::limits::current().input_bytes)?;
        let mut marker = [0; 2];
        reader.read_exact(&mut marker)?;
        let endian = match marker {
            [b'I', b'I'] => Endian::LittleEndian,
            [b'M', b'M'] => Endian::BigEndian,
            _ => return Err(invalid("Invalid TIFF byte order")),
        };
        reader.set_endian(endian);
        let variant = match reader.read_u16()? {
            42 => TiffVariant::Classic,
            43 => TiffVariant::BigTiff,
            _ => return Err(invalid("Invalid TIFF version")),
        };
        let mut next = if variant == TiffVariant::BigTiff {
            if reader.read_u16()? != 8 || reader.read_u16()? != 0 {
                return Err(invalid("Invalid BigTIFF offset size or reserved field"));
            }
            reader.read_u64()?
        } else {
            u64::from(reader.read_u32()?)
        };
        let mut parser = Parser {
            reader,
            variant,
            end,
            metadata: 0,
        };
        let mut ifds = Vec::new();
        let mut seen = HashSet::new();
        // Metadata directories may be shared by pages, but cannot also be
        // traversed as image IFDs. Keep these roles separate from chain cycles.
        let mut metadata_ifds = HashSet::new();
        while next != 0 {
            if metadata_ifds.contains(&next) {
                return Err(invalid("TIFF image chain references a metadata IFD"));
            }
            if !seen.insert(next) {
                return Err(invalid("Cyclic TIFF IFD chain"));
            }
            crate::limits::check(
                ifds.len()
                    .checked_add(1)
                    .ok_or_else(|| invalid("TIFF page count overflow"))?,
                crate::limits::current().frames,
                "TIFF pages",
            )?;
            let mut ifd = parser.directory_at(next)?;
            for tag in [0x8769, 0x8825] {
                if let Some(entry) = ifd.entries.iter().find(|e| e.tag == tag) {
                    if !matches!(entry.type_id, 4 | 13)
                        && !(variant == TiffVariant::BigTiff && matches!(entry.type_id, 16 | 18))
                    {
                        return Err(invalid("Invalid TIFF metadata IFD pointer type"));
                    }
                    let values = entry.unsigned(endian)?;
                    if values.len() != 1 {
                        return Err(invalid("Invalid TIFF metadata IFD pointer"));
                    }
                    let address = values[0];
                    if address != 0 {
                        if seen.contains(&address) {
                            return Err(invalid("Cyclic TIFF metadata IFD reference"));
                        }
                        metadata_ifds.insert(address);
                        let sub = parser.directory_at(address)?;
                        if sub.next_ifd.is_some()
                            || sub.entries.iter().any(|e| matches!(e.tag, 0x8769 | 0x8825))
                        {
                            return Err(invalid("Nested TIFF metadata IFD chain is unsupported"));
                        }
                        let headers = sub.headers(variant, endian)?.headers;
                        if tag == 0x8769 {
                            ifd.exif = Some(headers)
                        } else {
                            ifd.gps = Some(headers)
                        }
                    }
                }
            }
            next = ifd.next_ifd.unwrap_or(0);
            ifds.push(ifd);
        }
        Ok(Self {
            variant,
            endian,
            ifds,
        })
    }
    pub(crate) fn headers(&self) -> Result<TiffHeaders, Error> {
        let mut result = TiffHeaders::empty(self.endian);
        result.version = if self.variant == TiffVariant::Classic {
            42
        } else {
            43
        };
        for ifd in &self.ifds {
            let h = ifd.headers(self.variant, self.endian)?;
            result.headers.extend(h.headers);
            if result.exif.is_none() {
                result.exif = h.exif;
            }
            if result.gps.is_none() {
                result.gps = h.gps;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bin_rs::reader::BytesReader;

    fn put(out: &mut Vec<u8>, value: u64, bytes: usize, endian: Endian) {
        for i in 0..bytes {
            let shift = if endian == Endian::LittleEndian {
                i * 8
            } else {
                (bytes - 1 - i) * 8
            };
            out.push((value >> shift) as u8);
        }
    }
    #[test]
    fn wide_field_types_keep_full_payload_in_both_byte_orders() {
        for endian in [Endian::LittleEndian, Endian::BigEndian] {
            let mut data = if endian == Endian::LittleEndian {
                b"II".to_vec()
            } else {
                b"MM".to_vec()
            };
            put(&mut data, 43, 2, endian);
            put(&mut data, 8, 2, endian);
            put(&mut data, 0, 2, endian);
            put(&mut data, 16, 8, endian);
            put(&mut data, 3, 8, endian);
            for (tag, ty, count, value) in [
                (65000, 16, 1, u64::MAX),
                (65001, 17, 1, (-7i64) as u64),
                (65002, 18, 2, 96),
            ] {
                put(&mut data, tag, 2, endian);
                put(&mut data, ty, 2, endian);
                put(&mut data, count, 8, endian);
                put(&mut data, value, 8, endian);
            }
            put(&mut data, 0, 8, endian);
            data.resize(96, 0);
            put(&mut data, 0x1_0000_0010, 8, endian);
            put(&mut data, 42, 8, endian);
            let doc = TiffDocument::read(&mut BytesReader::new(&data)).unwrap();
            assert_eq!(doc.ifds[0].entries[0].unsigned(endian).unwrap(), [u64::MAX]);
            assert_eq!(number(&doc.ifds[0].entries[1].payload, endian) as i64, -7);
            assert_eq!(
                doc.ifds[0].entries[2].unsigned(endian).unwrap(),
                [0x1_0000_0010, 42]
            );
            let metadata = doc.headers().unwrap();
            assert_eq!(metadata.headers[2].length, 16);
            assert!(metadata.to_string().contains("TIFF Ver43"));
            let exif = super::super::header::exif_to_bytes(&metadata).unwrap();
            assert_eq!(number(&exif[2..4], endian), 42);
        }
    }
    #[test]
    fn huge_bigtiff_directory_count_is_rejected_before_allocation() {
        let mut data = b"II\x2b\0\x08\0\0\0\x10\0\0\0\0\0\0\0".to_vec();
        data.extend_from_slice(&u64::MAX.to_le_bytes());
        assert!(TiffDocument::read(&mut BytesReader::new(&data)).is_err());
    }

    #[test]
    fn shared_metadata_is_allowed_but_cannot_be_reused_as_an_image_ifd() {
        let endian = Endian::LittleEndian;
        let mut data = b"II\x2a\0\x08\0\0\0".to_vec();
        for next in [26, 0] {
            put(&mut data, 1, 2, endian);
            put(&mut data, 0x8769, 2, endian);
            put(&mut data, 4, 2, endian);
            put(&mut data, 1, 4, endian);
            put(&mut data, 44, 4, endian);
            put(&mut data, next, 4, endian);
        }
        put(&mut data, 1, 2, endian);
        put(&mut data, 0x10f, 2, endian);
        put(&mut data, 2, 2, endian);
        put(&mut data, 2, 4, endian);
        put(&mut data, u64::from(b'A'), 4, endian);
        put(&mut data, 0, 4, endian);
        let document = TiffDocument::read(&mut BytesReader::new(&data)).unwrap();
        assert_eq!(document.ifds.len(), 2);
        assert_eq!(document.ifds[0].exif, document.ifds[1].exif);
        let mut wrong_type = data.clone();
        wrong_type[12..14].copy_from_slice(&3u16.to_le_bytes());
        assert!(TiffDocument::read(&mut BytesReader::new(&wrong_type)).is_err());
        data[22..26].copy_from_slice(&44u32.to_le_bytes());
        assert!(TiffDocument::read(&mut BytesReader::new(&data)).is_err());
    }
}
