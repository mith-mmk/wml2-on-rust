//! Metadata value types shared by image decoders and encoders.

use crate::error::{ImgError, ImgErrorKind};
#[cfg(feature = "exif")]
use crate::tiff::header::TiffHeaders;
#[cfg(feature = "exif")]
use crate::tiff::header::exif_to_bytes;
use std::collections::HashMap;

#[cfg(feature = "c2pa")]
pub mod c2pa;
#[cfg(feature = "exif")]
pub mod exif;

/// A map of metadata keys to typed values.
pub type Metadata = HashMap<String, DataMap>;

/// A typed metadata value.
#[derive(Debug, Clone, PartialEq)]
pub enum DataMap {
    /// Boolean metadata value.
    Bool(bool),
    UInt(u64),
    SInt(i64),
    Float(f64),
    UIntAllay(Vec<u64>),
    SIntAllay(Vec<i64>),
    FloatAllay(Vec<f64>),
    Raw(Vec<u8>),
    Ascii(String),
    JSON(String),
    I18NString(String),
    SJISString(Vec<u8>),
    #[cfg(feature = "exif")]
    Exif(TiffHeaders),
    ICCProfile(Vec<u8>),
    None,
}

impl DataMap {
    /// Converts the metadata value to a display-oriented string.
    pub fn to_string(&self) -> String {
        match self {
            DataMap::Bool(d) => d.to_string(),
            DataMap::UInt(d) => d.to_string(),
            DataMap::SInt(d) => d.to_string(),
            DataMap::Float(d) => d.to_string(),
            DataMap::UIntAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::SIntAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::FloatAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::Raw(d) => {
                format!("{:?}", d)
            }
            DataMap::Ascii(d) => d.to_string(),
            DataMap::JSON(d) => json_pretty(d),
            DataMap::I18NString(d) => d.to_string(),
            DataMap::SJISString(d) => {
                format!("{:?}", d)
            }
            #[cfg(feature = "exif")]
            DataMap::Exif(header) => header.to_string(),
            DataMap::ICCProfile(iccprofile) => {
                format!("{:?}", iccprofile)
            }
            DataMap::None => "none".to_owned(),
        }
    }
}

/// Formats a JSON string using two-space indentation.
///
/// The formatter is intentionally small and only changes whitespace outside
/// string literals. It is suitable for metadata display paths where the source
/// JSON is already produced by the decoder.
pub fn json_pretty(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len() + value.len() / 4);
    let mut indent = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                out.push(ch);
                let close = if ch == '{' { '}' } else { ']' };
                if next_non_ws(&chars, index + 1) != Some(close) {
                    indent += 1;
                    push_json_newline(&mut out, indent);
                }
            }
            '}' | ']' => {
                let open = if ch == '}' { '{' } else { '[' };
                if previous_non_ws(&chars, index) != Some(open) {
                    indent = indent.saturating_sub(1);
                    push_json_newline(&mut out, indent);
                }
                out.push(ch);
            }
            ',' => {
                out.push(',');
                push_json_newline(&mut out, indent);
            }
            ':' => {
                out.push_str(": ");
            }
            ch if ch.is_whitespace() => {}
            _ => out.push(ch),
        }
        index += 1;
    }

    out
}

fn push_json_newline(out: &mut String, indent: usize) {
    out.push('\n');
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn next_non_ws(chars: &[char], mut index: usize) -> Option<char> {
    while index < chars.len() {
        let ch = chars[index];
        if !ch.is_whitespace() {
            return Some(ch);
        }
        index += 1;
    }
    None
}

fn previous_non_ws(chars: &[char], index: usize) -> Option<char> {
    let mut index = index.checked_sub(1)?;
    loop {
        let ch = chars[index];
        if !ch.is_whitespace() {
            return Some(ch);
        }
        if index == 0 {
            break;
        }
        index -= 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_pretty_formats_like_json_stringify_indent_two() {
        let json = r#"{"a":1,"b":[true,{"c":"d,e"}],"empty":[]}"#;

        assert_eq!(
            json_pretty(json),
            "{\n  \"a\": 1,\n  \"b\": [\n    true,\n    {\n      \"c\": \"d,e\"\n    }\n  ],\n  \"empty\": []\n}"
        );
    }
}

/// Extracts a serialized EXIF payload from metadata when present.
///
/// This accepts decoded TIFF-style metadata (`"Tiff headers"`), generic EXIF
/// metadata (`"EXIF"`), or an already serialized fallback (`"EXIF Raw"`).
pub fn get_exif(
    metadata: Option<&Metadata>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let Some(metadata) = metadata else {
        return Ok(None);
    };

    #[cfg(feature = "exif")]
    match metadata.get("EXIF") {
        Some(DataMap::Exif(headers)) => return exif_to_bytes(headers).map(Some),
        Some(DataMap::Raw(bytes)) => return Ok(Some(bytes.clone())),
        Some(_) => {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "EXIF metadata is not serializable".to_string(),
            )));
        }
        None => {}
    }

    #[cfg(feature = "exif")]
    match metadata.get("Tiff headers") {
        Some(DataMap::Exif(headers)) => return exif_to_bytes(headers).map(Some),
        Some(_) => {
            return Err(Box::new(ImgError::new_const(
                ImgErrorKind::InvalidParameter,
                "Tiff headers metadata is not serializable".to_string(),
            )));
        }
        None => {}
    }

    match metadata.get("EXIF Raw") {
        Some(DataMap::Raw(bytes)) => Ok(Some(bytes.clone())),
        Some(_) => Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "EXIF Raw metadata is not raw bytes".to_string(),
        ))),
        None => Ok(None),
    }
}

fn find_exif_option<'a>(options: Option<&'a Metadata>) -> Option<&'a DataMap> {
    let options = options?;
    options
        .get("exif")
        .or_else(|| options.get("exif "))
        .or_else(|| {
            options
                .iter()
                .find(|(key, _)| key.trim().eq_ignore_ascii_case("exif"))
                .map(|(_, value)| value)
        })
}

/// Resolves an EXIF payload from encoder options.
///
/// Supported option forms:
/// - `{"exif": Raw(bytes)}`: use serialized EXIF/TIFF bytes directly
/// - `{"exif": Exif(headers)}`: serialize [`TiffHeaders`] on demand
/// - `{"exif": Ascii("copy")}`: copy EXIF from source metadata
pub fn get_exif_option(
    options: Option<&Metadata>,
    metadata: Option<&Metadata>,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let Some(value) = find_exif_option(options) else {
        return Ok(None);
    };

    match value {
        DataMap::Raw(bytes) => Ok(Some(bytes.clone())),
        #[cfg(feature = "exif")]
        DataMap::Exif(headers) => exif_to_bytes(headers).map(Some),
        DataMap::Ascii(value) if value.trim().eq_ignore_ascii_case("copy") => get_exif(metadata),
        DataMap::Ascii(_) => Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "exif option must be Raw(bytes), Exif(headers), or Ascii(\"copy\")".to_string(),
        ))),
        _ => Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "exif option must be Raw(bytes), Exif(headers), or Ascii(\"copy\")".to_string(),
        ))),
    }
}

/// Selected EXIF data for EX encoders. Selection and sizing never clone payloads
/// or invoke the serializer; callers must check their budgets before materialize.
#[cfg(all(feature = "image-buffer-ex", any(feature = "png", feature = "tiff")))]
pub(crate) enum ExifOutputEx<'a> {
    Bytes(&'a [u8]),
    #[cfg(feature = "exif")]
    Tags(&'a TiffHeaders),
}

#[cfg(all(feature = "image-buffer-ex", any(feature = "png", feature = "tiff")))]
impl<'a> ExifOutputEx<'a> {
    pub(crate) fn size_bound(&self) -> Result<usize, Box<dyn std::error::Error>> {
        match self {
            Self::Bytes(bytes) => Ok(bytes.len()),
            #[cfg(feature = "exif")]
            Self::Tags(tags) => {
                use crate::tiff::header::{DataPack, TiffHeader};
                type Error = Box<dyn std::error::Error>;
                fn ifd_bound(tags: &[TiffHeader], pointers: usize) -> Result<usize, Error> {
                    let count = tags
                        .len()
                        .checked_add(pointers)
                        .ok_or("EXIF tag count overflow")?;
                    if count > u16::MAX as usize {
                        return Err("EXIF IFD has too many tags".into());
                    }
                    let mut size = count
                        .checked_mul(12)
                        .and_then(|n| n.checked_add(6))
                        .ok_or("EXIF IFD size overflow")?;
                    for tag in tags {
                        let (count, item) = match &tag.data {
                            DataPack::Bytes(v) | DataPack::Undef(v) | DataPack::Unkown(v) => {
                                (v.len(), 1)
                            }
                            DataPack::SByte(v) => (v.len(), 1),
                            DataPack::Ascii(v) => (
                                v.len()
                                    .checked_add(usize::from(!v.ends_with('\0')))
                                    .ok_or("EXIF ASCII size overflow")?,
                                1,
                            ),
                            DataPack::Short(v) => (v.len(), 2),
                            DataPack::SShort(v) => (v.len(), 2),
                            DataPack::Long(v) => (v.len(), 4),
                            DataPack::SLong(v) => (v.len(), 4),
                            DataPack::Float(v) => (v.len(), 4),
                            DataPack::Rational(v) => (v.len(), 8),
                            DataPack::SRational(v) => (v.len(), 8),
                            DataPack::Double(v) => (v.len(), 8),
                        };
                        u32::try_from(count)?;
                        let payload = count
                            .checked_mul(item)
                            .ok_or("EXIF payload size overflow")?;
                        if payload > 4 {
                            size = size
                                .checked_add(payload)
                                .and_then(|n| n.checked_add(payload % 2))
                                .ok_or("EXIF payload sum overflow")?;
                        }
                    }
                    Ok(size)
                }
                // Count every source tag, even structural pointers that the
                // serializer may replace. This is an allocation-free upper bound.
                let exif = tags.exif.as_deref().filter(|v| !v.is_empty());
                let gps = tags.gps.as_deref().filter(|v| !v.is_empty());
                let mut size = 8usize
                    .checked_add(ifd_bound(
                        &tags.headers,
                        usize::from(exif.is_some()) + usize::from(gps.is_some()),
                    )?)
                    .ok_or("EXIF size overflow")?;
                for child in [exif, gps].into_iter().flatten() {
                    size = size
                        .checked_add(ifd_bound(child, 0)?)
                        .ok_or("EXIF child IFD size overflow")?;
                }
                u32::try_from(size)?;
                Ok(size)
            }
        }
    }
    pub(crate) fn workspace_bound(&self, size: usize) -> Result<usize, Box<dyn std::error::Error>> {
        match self {
            Self::Bytes(_) => Ok(0), // borrowed; no adapter-owned clone
            // Include cloned tag structs, encoded tags, payloads and directory
            // assembly buffers in the existing serializer. Caller storage and
            // allocator overhead are separate from this conservative estimate.
            #[cfg(feature = "exif")]
            Self::Tags(_) => size
                .checked_mul(16)
                .ok_or_else(|| "EXIF serialization workspace overflow".into()),
        }
    }
    pub(crate) fn materialize(
        &self,
    ) -> Result<std::borrow::Cow<'a, [u8]>, Box<dyn std::error::Error>> {
        #[cfg(test)]
        EXIF_EX_MATERIALIZATIONS.with(|n| n.set(n.get() + 1));
        match self {
            Self::Bytes(bytes) => Ok(std::borrow::Cow::Borrowed(bytes)),
            #[cfg(feature = "exif")]
            Self::Tags(tags) => exif_to_bytes(tags).map(std::borrow::Cow::Owned),
        }
    }
}

#[cfg(all(
    test,
    feature = "image-buffer-ex",
    any(feature = "png", feature = "tiff")
))]
thread_local! { static EXIF_EX_MATERIALIZATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(all(feature = "image-buffer-ex", any(feature = "png", feature = "tiff")))]
pub(crate) fn exif_output_ex<'a>(
    options: Option<&'a Metadata>,
    metadata: Option<&'a Metadata>,
) -> Result<Option<ExifOutputEx<'a>>, Box<dyn std::error::Error>> {
    fn payload(value: &DataMap) -> Result<ExifOutputEx<'_>, Box<dyn std::error::Error>> {
        match value {
            DataMap::Raw(bytes) => Ok(ExifOutputEx::Bytes(bytes)),
            #[cfg(feature = "exif")]
            DataMap::Exif(tags) => Ok(ExifOutputEx::Tags(tags)),
            _ => Err("EXIF metadata is not serializable".into()),
        }
    }
    let Some(value) = find_exif_option(options) else {
        return Ok(None);
    };
    if matches!(value, DataMap::Ascii(v) if v.trim().eq_ignore_ascii_case("copy")) {
        let Some(metadata) = metadata else {
            return Ok(None);
        };
        #[cfg(feature = "exif")]
        for key in ["EXIF", "Tiff headers"] {
            if let Some(value) = metadata.get(key) {
                if key == "Tiff headers" && !matches!(value, DataMap::Exif(_)) {
                    return Err("Tiff headers metadata is not serializable".into());
                }
                return payload(value).map(Some);
            }
        }
        return match metadata.get("EXIF Raw") {
            Some(DataMap::Raw(bytes)) => Ok(Some(ExifOutputEx::Bytes(bytes))),
            Some(_) => Err("EXIF Raw metadata is not raw bytes".into()),
            None => Ok(None),
        };
    }
    payload(value).map(Some)
}

#[cfg(all(test, feature = "image-buffer-ex", feature = "png"))]
mod ex_encode_tests {
    use super::*;
    use crate::draw::*;
    use crate::limits::DecodeLimits;
    use crate::tiff::header::{DataPack, TiffHeader};
    use crate::util::ImageFormat;
    use bin_rs::Endian;

    #[test]
    fn exif_raw_is_borrowed_and_structured_bound_covers_payloads_and_child_ifds() {
        let bytes = vec![123; 4096];
        let raw = ExifOutputEx::Bytes(&bytes);
        assert_eq!(raw.size_bound().unwrap(), bytes.len());
        assert_eq!(raw.workspace_bound(bytes.len()).unwrap(), 0);
        let materialized = raw.materialize().unwrap();
        assert!(matches!(materialized, std::borrow::Cow::Borrowed(_)));
        assert_eq!(materialized.as_ptr(), bytes.as_ptr());
        let data = [
            DataPack::Bytes(vec![1; 7]),
            DataPack::Undef(vec![2; 8]),
            DataPack::Unkown(vec![3; 9]),
            DataPack::SByte(vec![-1; 5]),
            DataPack::Ascii("odd".into()),
            DataPack::Ascii("nul\0".into()),
            DataPack::Short(vec![2; 3]),
            DataPack::SShort(vec![-2; 3]),
            DataPack::Long(vec![3; 3]),
            DataPack::SLong(vec![-3; 3]),
            DataPack::Float(vec![1.5; 3]),
            DataPack::Double(vec![2.5; 3]),
            DataPack::Rational(vec![crate::tiff::header::Rational { n: 1, d: 2 }; 3]),
            DataPack::SRational(vec![crate::tiff::header::SRational { n: -1, d: 2 }; 3]),
        ];
        let mut headers = TiffHeaders::empty(Endian::BigEndian);
        headers.headers = data
            .into_iter()
            .enumerate()
            .map(|(i, data)| TiffHeader {
                tagid: 0x100 + i,
                length: usize::MAX,
                data,
            })
            .collect();
        headers.exif = Some(headers.headers.clone());
        headers.gps = Some(headers.headers.clone());
        let plan = ExifOutputEx::Tags(&headers);
        let bound = plan.size_bound().unwrap();
        assert!(plan.materialize().unwrap().len() <= bound);
    }

    #[test]
    fn png_ex_rejects_metadata_and_workspace_budgets_before_materialization() {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba16, None, vec![0; 8]).unwrap();
        let mut headers = TiffHeaders::empty(Endian::LittleEndian);
        headers.headers.push(TiffHeader {
            tagid: 0x010e,
            length: 1 << 20,
            data: DataPack::Undef(vec![7; 1 << 20]),
        });
        for option in [
            DataMap::Raw(vec![7; 1 << 20]),
            DataMap::Ascii("copy".into()),
            DataMap::Exif(headers),
        ] {
            image.metadata = Some(HashMap::from([(
                "EXIF Raw".into(),
                DataMap::Raw(vec![7; 1 << 20]),
            )]));
            let options = Some(HashMap::from([("exif".into(), option)]));
            let mut encode = EncodeOptionsEx {
                debug_flag: 0,
                drawer: &mut image,
                output: PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
                options,
            };
            for limits in [
                DecodeLimits {
                    metadata_bytes: 1,
                    ..DecodeLimits::unlimited()
                },
                DecodeLimits {
                    expanded_bytes: 2048,
                    ..DecodeLimits::unlimited()
                },
            ] {
                EXIF_EX_MATERIALIZATIONS.with(|n| n.set(0));
                let error = image_encoder_ex_with_limits(&mut encode, ImageFormat::Png, limits)
                    .unwrap_err();
                assert!(error.to_string().contains("limit"), "{error}");
                assert_eq!(EXIF_EX_MATERIALIZATIONS.with(|n| n.get()), 0);
            }
        }
    }
}
