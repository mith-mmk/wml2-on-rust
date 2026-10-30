//! Optional interpretation of retained JPEG XL metadata; pixel identity stays
//! with the codestream. Malformed/unsupported optional metadata stays raw.
use super::{Error, add, error, mul};
use crate::{
    error::ImgErrorKind,
    limits::{self, DecodeLimits},
    metadata::DataMap,
};

const XML_BYTES: usize = 4 << 20;
const XML_NODES: u32 = 65536;
const XML_FACTOR: usize = 128;
const TEMP_FIXED: usize = 64 << 10;
#[cfg(feature = "exif")]
const EXIF_PARSE_BYTES: usize = 8 << 20;
#[cfg(feature = "exif")]
const EXIF_FIXED: usize = 4096;

#[cfg(test)]
std::thread_local! { static XML_PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
#[cfg(all(test, feature = "exif"))]
std::thread_local! { static EXIF_PARSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

struct Budget {
    metadata: usize,
    workspace: usize,
    limits: DecodeLimits,
}
impl Budget {
    fn remaining(&self) -> (usize, usize) {
        (
            self.limits.metadata_bytes.saturating_sub(self.metadata),
            self.limits.expanded_bytes.saturating_sub(self.workspace),
        )
    }
    fn retain(&mut self, key: &str, bytes: usize) -> Result<(), Error> {
        let bytes = add(key.len(), bytes)?;
        self.metadata = add(self.metadata, bytes)?;
        self.workspace = add(self.workspace, mul(bytes, 2)?)?;
        limits::check(
            self.metadata,
            self.limits.metadata_bytes,
            "JPEG XL structured metadata",
        )?;
        limits::check(
            self.workspace,
            self.limits.expanded_bytes,
            "JPEG XL structured workspace",
        )
    }
}

pub(super) fn augment(
    entries: &mut Vec<(String, DataMap)>,
    limits: DecodeLimits,
    metadata: usize,
    workspace: usize,
) -> Result<usize, Error> {
    let mut budget = Budget {
        metadata,
        workspace,
        limits,
    };
    let mut xmp = false;
    #[cfg(feature = "exif")]
    let mut exif = false;
    // Only the original indexed records are candidates, in file order. A brob
    // contributes its Decoded record, never its compressed Source payload.
    for index in 0..entries.len() {
        let candidate = {
            let (key, value) = &entries[index];
            if !key.starts_with("JPEG XL Source Box ") && !key.starts_with("JPEG XL Decoded Box ") {
                continue;
            }
            let DataMap::Raw(data) = value else {
                continue;
            };
            let (remaining, space) = budget.remaining();
            if !xmp && key.ends_with(" 786d6c20") {
                xml(data, remaining, space)?.map(|text| ("XMP", text.len(), DataMap::Ascii(text)))
            } else {
                #[cfg(feature = "exif")]
                {
                    if !exif && key.ends_with(" 45786966") {
                        exif_data(data, remaining, space, limits)?
                            .map(|(headers, size)| ("EXIF", size, DataMap::Exif(headers)))
                    } else {
                        None
                    }
                }
                #[cfg(not(feature = "exif"))]
                {
                    None
                }
            }
        };
        if let Some((key, size, value)) = candidate {
            // Parsing/materialization reserved these costs before allocating.
            budget.retain(key, size)?;
            if key == "XMP" {
                xmp = true;
            }
            #[cfg(feature = "exif")]
            if key == "EXIF" {
                exif = true;
            }
            entries.push((key.to_owned(), value));
        }
    }
    Ok(budget.workspace)
}

fn xml(data: &[u8], remaining: usize, space: usize) -> Result<Option<String>, Error> {
    let Ok(text) = std::str::from_utf8(data) else {
        return Ok(None);
    };
    if !text
        .trim_start_matches(|c: char| c == '\u{feff}' || c.is_ascii_whitespace())
        .starts_with('<')
    {
        return Ok(None);
    }
    if !utf8_declaration(text) {
        return Ok(None);
    }
    limits::check(data.len(), XML_BYTES, "JPEG XL XMP input")?;
    limits::check(add(data.len(), 3)?, remaining, "JPEG XL XMP metadata")?;
    // The pinned parser guesses initial Vec capacities from '<' and '=' bytes
    // BEFORE applying nodes_limit. Admit input-proportional temporary storage
    // before constructing a Document; nodes_limit alone would not suffice.
    // Namespace inheritance can otherwise multiply declaration and element
    // counts. Each parser tree_order index is u16; reserve doubled capacity
    // for every possible inherited reference, including built-in xmlns:xml.
    let namespace_count = add(data.windows(5).filter(|part| *part == b"xmlns").count(), 1)?;
    let markup_count = add(data.iter().filter(|&&b| b == b'<').count(), 1)?;
    let namespaces = mul(mul(namespace_count, markup_count)?, 4)?;
    limits::check(
        add(add(mul(data.len(), XML_FACTOR)?, TEMP_FIXED)?, namespaces)?,
        space,
        "JPEG XL XMP workspace",
    )?;
    #[cfg(test)]
    XML_PARSES.with(|n| n.set(n.get() + 1));
    let document = match roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: XML_NODES,
            entity_resolver: None,
        },
    ) {
        Ok(doc) => doc,
        Err(
            roxmltree::Error::NodesLimitReached
            | roxmltree::Error::AttributesLimitReached
            | roxmltree::Error::NamespacesLimitReached,
        ) => {
            return Err(error(
                ImgErrorKind::OutOfMemory,
                "JPEG XL XMP complexity exceeds decode limit",
            ));
        }
        Err(_) => return Ok(None),
    };
    let root = document.root_element().tag_name();
    let is_xmp = matches!(
        (root.namespace(), root.name()),
        (Some("adobe:ns:meta/"), "xmpmeta" | "xapmeta")
            | (Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#"), "RDF")
    );
    let unsafe_reference = document.descendants().any(|node| {
        node.tag_name().namespace() == Some("http://www.w3.org/2001/XInclude")
            || node.pi().is_some_and(|pi| pi.target != "xpacket")
    });
    if !is_xmp || unsafe_reference {
        return Ok(None);
    }
    drop(document);
    let mut value = String::new();
    value.try_reserve_exact(text.len())?;
    value.push_str(text);
    Ok(Some(value))
}

// This is an encoding/version support check, not an XML parser. The pinned
// parser validates the declaration's grammar, but does not expose its values.
fn utf8_declaration(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(after) = text.strip_prefix("<?xml") else {
        return true;
    };
    if !after.starts_with(char::is_whitespace) {
        return true;
    }
    let Some((decl, _)) = after.split_once("?>") else {
        return false;
    };
    let attr = |name: &str| -> Option<&str> {
        let (_, value) = decl.split_once(name)?;
        let value = value.trim_start().strip_prefix('=')?.trim_start();
        let quote = value.chars().next()?;
        if !matches!(quote, '\'' | '"') {
            return None;
        }
        value[1..].split_once(quote).map(|(value, _)| value)
    };
    if attr("version") != Some("1.0") {
        return false;
    }
    if decl.contains("standalone") && !matches!(attr("standalone"), Some("yes" | "no")) {
        return false;
    }
    if let Some(encoding) = attr("encoding")
        && !(encoding.eq_ignore_ascii_case("UTF-8")
            || encoding.eq_ignore_ascii_case("US-ASCII") && text.is_ascii())
    {
        return false;
    }
    true
}

#[cfg(feature = "exif")]
fn exif_data(
    data: &[u8],
    remaining: usize,
    space: usize,
    outer: DecodeLimits,
) -> Result<Option<(crate::tiff::header::TiffHeaders, usize)>, Error> {
    use bin_rs::reader::BytesReader;
    let Some(offset) = data.get(..4) else {
        return Ok(None);
    };
    let offset = u32::from_be_bytes(offset.try_into()?) as usize;
    let Some(start) = offset.checked_add(4) else {
        return Ok(None);
    };
    let Some(tiff) = data.get(start..) else {
        return Ok(None);
    };
    if tiff.len() < 8 || !matches!(&tiff[..4], b"II*\0" | b"MM\0*" | b"II+\0" | b"MM\0+") {
        return Ok(None);
    }
    limits::check(EXIF_FIXED + 4, remaining, "JPEG XL EXIF metadata")?;
    limits::check(TEMP_FIXED, space, "JPEG XL EXIF workspace")?;
    let maximum = EXIF_PARSE_BYTES
        .min((remaining - EXIF_FIXED - 4) / 4)
        .min((space - TEMP_FIXED) / 16);
    let inner = DecodeLimits {
        // These bytes may have come from brob. Outer input_bytes already bound
        // the compressed file; this bounded slice is decoded metadata instead.
        input_bytes: tiff.len(),
        metadata_bytes: maximum,
        expanded_bytes: space,
        frames: 64,
        ..outer
    };
    let result = limits::scope(inner, || {
        #[cfg(test)]
        EXIF_PARSES.with(|n| n.set(n.get() + 1));
        let document = crate::tiff::ifd::TiffDocument::read_exif(&mut BytesReader::new(tiff))?;
        let reserved = add(mul(document.metadata_bytes, 4)?, EXIF_FIXED)?;
        limits::check(
            add(reserved, 4)?,
            remaining,
            "JPEG XL EXIF retained metadata",
        )?;
        limits::check(
            add(mul(document.metadata_bytes, 16)?, TEMP_FIXED)?,
            space,
            "JPEG XL EXIF workspace",
        )?;
        let mut headers = document.headers()?;
        // Common EXIF describes delivered pixels: preserve every original byte
        // in indexed raw metadata, but never advertise a second rotation.
        let mut orientation = false;
        for tag in headers.headers.iter_mut().filter(|tag| tag.tagid == 0x112) {
            tag.data = crate::tiff::header::DataPack::Short(vec![1]);
            tag.length = 1;
            orientation = true;
        }
        if !orientation {
            crate::metadata::exif::set_short(
                &mut headers,
                crate::metadata::exif::ExifIfd::Primary,
                0x112,
                vec![1],
            );
        }
        let cost = header_bytes(&headers)?;
        limits::check(cost, reserved, "JPEG XL EXIF retained estimate")?;
        Ok::<_, Error>((headers, cost))
    });
    match result {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error)
            if error
                .downcast_ref::<crate::error::ImgError>()
                .is_some_and(|e| e.is_optional_metadata_error()) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

#[cfg(feature = "exif")]
fn header_bytes(headers: &crate::tiff::header::TiffHeaders) -> Result<usize, Error> {
    use crate::tiff::header::DataPack;
    fn owned<T>(values: &Vec<T>) -> Result<usize, Error> {
        mul(values.capacity(), std::mem::size_of::<T>())
    }
    let mut total = std::mem::size_of_val(headers);
    for group in [
        Some(&headers.headers),
        headers.exif.as_ref(),
        headers.gps.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        total = add(total, owned(group)?)?;
        for tag in group {
            let bytes = match &tag.data {
                DataPack::Bytes(v) | DataPack::Unkown(v) | DataPack::Undef(v) => owned(v)?,
                DataPack::Ascii(v) => v.capacity(),
                DataPack::SByte(v) => owned(v)?,
                DataPack::Short(v) => owned(v)?,
                DataPack::SShort(v) => owned(v)?,
                DataPack::Long(v) => owned(v)?,
                DataPack::SLong(v) => owned(v)?,
                DataPack::Rational(v) => owned(v)?,
                DataPack::SRational(v) => owned(v)?,
                DataPack::Float(v) => owned(v)?,
                DataPack::Double(v) => owned(v)?,
            };
            total = add(total, bytes)?;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_admission_precedes_parser_construction() {
        let data = b"<x:xmpmeta xmlns:x='adobe:ns:meta/'/>";
        XML_PARSES.with(|n| n.set(0));
        assert!(xml(data, 0, usize::MAX).is_err());
        assert!(xml(data, usize::MAX, 0).is_err());
        // Input-proportional admission alone omits inherited namespace indexes.
        assert!(xml(data, usize::MAX, data.len() * XML_FACTOR + TEMP_FIXED).is_err());
        XML_PARSES.with(|n| assert_eq!(n.get(), 0));
        assert!(xml(data, usize::MAX, usize::MAX).unwrap().is_some());
        XML_PARSES.with(|n| assert_eq!(n.get(), 1));
    }
    #[test]
    #[cfg(feature = "exif")]
    fn exif_admission_precedes_parser_and_retained_cost_counts_owned_buffers() {
        let mut data = vec![0; 4];
        data.extend(include_bytes!(
            "../tests/fixtures/review/jxl-meta-little.tiff"
        ));
        EXIF_PARSES.with(|n| n.set(0));
        assert!(exif_data(&data, 0, usize::MAX, DecodeLimits::unlimited()).is_err());
        assert!(exif_data(&data, usize::MAX, 0, DecodeLimits::unlimited()).is_err());
        EXIF_PARSES.with(|n| assert_eq!(n.get(), 0));
        let (headers, size) = exif_data(&data, usize::MAX, usize::MAX, DecodeLimits::unlimited())
            .unwrap()
            .unwrap();
        assert_eq!(size, header_bytes(&headers).unwrap());
        assert!(size > std::mem::size_of_val(&headers) + 8);
        EXIF_PARSES.with(|n| assert_eq!(n.get(), 1));
    }
}
