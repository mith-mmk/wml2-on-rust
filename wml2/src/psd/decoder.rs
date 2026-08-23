//! PSD v1 parser and callback adapter.

use std::collections::HashMap;
use std::io::SeekFrom;

use bin_rs::reader::BinaryReader;

use crate::draw::{
    CallbackResponse, DecodeOptions, ImageRect, InitOptions, NextBlend, NextDispose, NextOption,
    NextOptions, ResponseCommand,
};
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;
use crate::warning::ImgWarnings;

use super::color::{RasterFormat, base_channels, planes_to_rgba};
use super::compression::decompress_planar;

type Error = Box<dyn std::error::Error>;

const PSD_SIGNATURE: &[u8; 4] = b"8BPS";
const MAX_PSD_DIMENSION: usize = 30_000;

fn err(kind: ImgErrorKind, message: impl Into<String>) -> Error {
    Box::new(ImgError::new_const(kind, message.into()))
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }

    fn read(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD section offset overflow"))?;
        let value = self.data.get(self.offset..end).ok_or_else(|| {
            err(
                ImgErrorKind::UnexpectedEof,
                format!("PSD needs {length} bytes at offset {}", self.offset),
            )
        })?;
        self.offset = end;
        Ok(value)
    }

    fn skip(&mut self, length: usize) -> Result<(), Error> {
        self.read(length).map(|_| ())
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.read(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, Error> {
        let value = self.read(2)?;
        Ok(u16::from_be_bytes([value[0], value[1]]))
    }

    fn i16(&mut self) -> Result<i16, Error> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, Error> {
        let value = self.read(4)?;
        Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        Ok(self.u32()? as i32)
    }

    fn sized_u32(&mut self, context: &str) -> Result<&'a [u8], Error> {
        let length = usize::try_from(self.u32()?).map_err(|_| {
            err(
                ImgErrorKind::IllegalData,
                format!("PSD {context} length does not fit usize"),
            )
        })?;
        self.read(length)
    }
}

#[derive(Clone, Copy)]
struct Header {
    channels: usize,
    width: usize,
    height: usize,
    depth: u16,
    mode: u16,
}

struct ChannelInfo {
    id: i16,
    length: usize,
}

struct LayerRecord {
    top: i32,
    left: i32,
    width: usize,
    height: usize,
    channels: Vec<ChannelInfo>,
    blend_mode: String,
    opacity: u8,
    visible: bool,
    name: String,
}

struct LayerFrame {
    source_index: usize,
    top: i32,
    left: i32,
    width: usize,
    height: usize,
    blend_mode: String,
    opacity: u8,
    visible: bool,
    name: String,
    rgba: Vec<u8>,
}

struct ParsedPsd {
    header: Header,
    metadata: HashMap<String, DataMap>,
    composite: Vec<u8>,
    layers: Vec<LayerFrame>,
}

fn parse_header(cursor: &mut Cursor<'_>) -> Result<Header, Error> {
    if cursor.read(4)? != PSD_SIGNATURE {
        return Err(err(ImgErrorKind::IllegalData, "PSD signature is not 8BPS"));
    }
    let version = cursor.u16()?;
    if version == 2 {
        return Err(err(
            ImgErrorKind::UnsupportedFeature,
            "PSB large-document files are not supported",
        ));
    }
    if version != 1 {
        return Err(err(
            ImgErrorKind::IllegalData,
            format!("PSD version {version} is invalid"),
        ));
    }
    if cursor.read(6)?.iter().any(|byte| *byte != 0) {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD reserved header bytes must be zero",
        ));
    }
    let channels = cursor.u16()? as usize;
    if !(1..=56).contains(&channels) {
        return Err(err(
            ImgErrorKind::IllegalData,
            format!("PSD channel count {channels} is outside 1..=56"),
        ));
    }
    let height = usize::try_from(cursor.u32()?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "PSD height does not fit usize"))?;
    let width = usize::try_from(cursor.u32()?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "PSD width does not fit usize"))?;
    if width == 0 || height == 0 || width > MAX_PSD_DIMENSION || height > MAX_PSD_DIMENSION {
        return Err(err(
            ImgErrorKind::IllegalData,
            format!("PSD dimensions {width}x{height} are outside the v1 limit"),
        ));
    }
    let depth = cursor.u16()?;
    if depth != 8 && depth != 16 {
        return Err(err(
            ImgErrorKind::UnsupportedFeature,
            format!("PSD depth {depth} is not supported"),
        ));
    }
    let mode = cursor.u16()?;
    let base = base_channels(mode)?;
    if channels < base {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD has fewer channels than its color mode requires",
        ));
    }
    if mode == 2 && depth != 8 {
        return Err(err(
            ImgErrorKind::UnsupportedFeature,
            "indexed PSD is supported only at 8 bits per channel",
        ));
    }
    Ok(Header {
        channels,
        width,
        height,
        depth,
        mode,
    })
}

fn color_mode_name(mode: u16) -> &'static str {
    match mode {
        1 => "Grayscale",
        2 => "Indexed",
        3 => "RGB",
        4 => "CMYK",
        _ => "Unsupported",
    }
}

fn compression_name(compression: u16) -> &'static str {
    match compression {
        0 => "Raw",
        1 => "RLE",
        2 => "ZIP",
        3 => "ZIP prediction",
        _ => "Unsupported",
    }
}

fn parse_resources(data: &[u8], metadata: &mut HashMap<String, DataMap>) -> Result<(), Error> {
    let mut cursor = Cursor::new(data);
    while cursor.remaining() != 0 {
        if cursor.remaining() < 12 {
            return Err(err(
                ImgErrorKind::UnexpectedEof,
                "PSD image resource header is truncated",
            ));
        }
        if cursor.read(4)? != b"8BIM" {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PSD image resource signature is not 8BIM",
            ));
        }
        let id = cursor.u16()?;
        let name_length = cursor.u8()? as usize;
        cursor.skip(name_length)?;
        if !(name_length + 1).is_multiple_of(2) {
            cursor.skip(1)?;
        }
        let resource = cursor.sized_u32("image resource")?;
        if resource.len() % 2 != 0 {
            cursor.skip(1)?;
        }
        match id {
            1039 => {
                metadata.insert(
                    "ICC Profile".to_string(),
                    DataMap::ICCProfile(resource.to_vec()),
                );
            }
            1058 | 1059 => {
                metadata.insert("EXIF Raw".to_string(), DataMap::Raw(resource.to_vec()));
            }
            1060 => match std::str::from_utf8(resource) {
                Ok(xmp) => {
                    metadata.insert("XMP".to_string(), DataMap::Ascii(xmp.to_string()));
                }
                Err(_) => {
                    metadata.insert("XMP Raw".to_string(), DataMap::Raw(resource.to_vec()));
                }
            },
            _ => {}
        }
    }
    Ok(())
}

fn layer_extent(start: i32, end: i32, axis: &str) -> Result<usize, Error> {
    let size = i64::from(end) - i64::from(start);
    if size < 0 {
        return Err(err(
            ImgErrorKind::IllegalData,
            format!("PSD layer {axis} extent is negative"),
        ));
    }
    usize::try_from(size).map_err(|_| {
        err(
            ImgErrorKind::IllegalData,
            "PSD layer extent does not fit usize",
        )
    })
}

fn parse_unicode_name(data: &[u8]) -> Option<String> {
    let mut cursor = Cursor::new(data);
    let count = usize::try_from(cursor.u32().ok()?).ok()?;
    let bytes = cursor.read(count.checked_mul(2)?).ok()?;
    let utf16: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();
    Some(String::from_utf16_lossy(&utf16))
}

fn parse_layer_extra(data: &[u8]) -> Result<String, Error> {
    let mut cursor = Cursor::new(data);
    let mask_length = usize::try_from(cursor.u32()?)
        .map_err(|_| err(ImgErrorKind::IllegalData, "PSD layer mask length overflow"))?;
    cursor.skip(mask_length)?;
    let ranges_length = usize::try_from(cursor.u32()?).map_err(|_| {
        err(
            ImgErrorKind::IllegalData,
            "PSD blend ranges length overflow",
        )
    })?;
    cursor.skip(ranges_length)?;

    let name_length = cursor.u8()? as usize;
    let name_bytes = cursor.read(name_length)?;
    let mut name = String::from_utf8_lossy(name_bytes).to_string();
    let pascal_size = name_length + 1;
    let padding = (4 - pascal_size % 4) % 4;
    cursor.skip(padding)?;

    while cursor.remaining() >= 12 {
        let signature = cursor.read(4)?;
        if signature != b"8BIM" && signature != b"8B64" {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PSD additional layer signature is invalid",
            ));
        }
        let key = cursor.read(4)?;
        let payload = cursor.sized_u32("additional layer information")?;
        if payload.len() % 2 != 0 {
            cursor.skip(1)?;
        }
        if key == b"luni"
            && let Some(unicode) = parse_unicode_name(payload)
        {
            name = unicode;
        }
    }
    if cursor.remaining() != 0
        && cursor
            .read(cursor.remaining())?
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD layer extra data has non-padding trailing bytes",
        ));
    }
    Ok(name)
}

fn parse_layer_record(cursor: &mut Cursor<'_>) -> Result<LayerRecord, Error> {
    let top = cursor.i32()?;
    let left = cursor.i32()?;
    let bottom = cursor.i32()?;
    let right = cursor.i32()?;
    let width = layer_extent(left, right, "horizontal")?;
    let height = layer_extent(top, bottom, "vertical")?;
    let channel_count = cursor.u16()? as usize;
    if channel_count > 64 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD layer channel count is unreasonable",
        ));
    }
    let mut channels = Vec::new();
    channels.try_reserve_exact(channel_count).map_err(|_| {
        err(
            ImgErrorKind::OutOfMemory,
            "cannot allocate PSD layer channel table",
        )
    })?;
    for _ in 0..channel_count {
        let id = cursor.i16()?;
        let length = usize::try_from(cursor.u32()?).map_err(|_| {
            err(
                ImgErrorKind::IllegalData,
                "PSD layer channel length does not fit usize",
            )
        })?;
        if length == 1 {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PSD layer channel is shorter than its compression field",
            ));
        }
        channels.push(ChannelInfo { id, length });
    }
    if cursor.read(4)? != b"8BIM" {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD layer blend signature is not 8BIM",
        ));
    }
    let blend_mode = String::from_utf8_lossy(cursor.read(4)?).to_string();
    let opacity = cursor.u8()?;
    let _clipping = cursor.u8()?;
    let flags = cursor.u8()?;
    let _filler = cursor.u8()?;
    let extra = cursor.sized_u32("layer extra data")?;
    let name = parse_layer_extra(extra)?;
    Ok(LayerRecord {
        top,
        left,
        width,
        height,
        channels,
        blend_mode,
        opacity,
        visible: flags & 0x02 == 0,
        name,
    })
}

fn parse_layers(
    data: &[u8],
    header: Header,
    palette: Option<&[u8]>,
) -> Result<(usize, Vec<LayerFrame>), Error> {
    if data.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut section = Cursor::new(data);
    let layer_info = section.sized_u32("layer information")?;
    if layer_info.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut cursor = Cursor::new(layer_info);
    let signed_count = cursor.i16()?;
    let record_count = signed_count.unsigned_abs() as usize;
    if record_count > cursor.remaining() / 34 {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD layer count exceeds the layer information section",
        ));
    }
    let mut records = Vec::new();
    records.try_reserve_exact(record_count).map_err(|_| {
        err(
            ImgErrorKind::OutOfMemory,
            "cannot allocate PSD layer records",
        )
    })?;
    for _ in 0..record_count {
        records.push(parse_layer_record(&mut cursor)?);
    }

    let base = base_channels(header.mode)?;
    let mut layers = Vec::new();
    layers.try_reserve_exact(record_count).map_err(|_| {
        err(
            ImgErrorKind::OutOfMemory,
            "cannot allocate PSD decoded layers",
        )
    })?;
    for (source_index, record) in records.into_iter().enumerate() {
        let mut planes = vec![None; base];
        let mut alpha = None;
        for channel in &record.channels {
            let encoded = cursor.read(channel.length)?;
            if encoded.is_empty() || record.width == 0 || record.height == 0 {
                continue;
            }
            let compression = u16::from_be_bytes([encoded[0], encoded[1]]);
            if channel.id == -1 || (0..base as i16).contains(&channel.id) {
                let decoded = decompress_planar(
                    compression,
                    &encoded[2..],
                    record.width,
                    record.height,
                    1,
                    header.depth,
                )?;
                if channel.id == -1 {
                    alpha = Some(decoded);
                } else {
                    planes[channel.id as usize] = Some(decoded);
                }
            }
        }
        if record.width == 0 || record.height == 0 || planes.iter().any(Option::is_none) {
            continue;
        }
        let rgba = planes_to_rgba(
            RasterFormat {
                width: record.width,
                height: record.height,
                depth: header.depth,
                mode: header.mode,
                palette,
            },
            &planes,
            alpha.as_deref(),
            record.opacity,
        )?;
        layers.push(LayerFrame {
            source_index,
            top: record.top,
            left: record.left,
            width: record.width,
            height: record.height,
            blend_mode: record.blend_mode,
            opacity: record.opacity,
            visible: record.visible,
            name: record.name,
            rgba,
        });
    }
    if cursor.remaining() != 0
        && cursor
            .read(cursor.remaining())?
            .iter()
            .any(|byte| *byte != 0)
    {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD layer information contains unexpected trailing bytes",
        ));
    }
    Ok((record_count, layers))
}

fn split_planes(data: Vec<u8>, header: Header) -> Result<Vec<Option<Vec<u8>>>, Error> {
    let bytes_per_sample = usize::from(header.depth / 8);
    let plane_len = header
        .width
        .checked_mul(header.height)
        .and_then(|value| value.checked_mul(bytes_per_sample))
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD plane size overflow"))?;
    let expected = plane_len
        .checked_mul(header.channels)
        .ok_or_else(|| err(ImgErrorKind::IllegalData, "PSD planar size overflow"))?;
    if data.len() != expected {
        return Err(err(
            ImgErrorKind::IllegalData,
            "PSD planar data length does not match its header",
        ));
    }
    let mut planes = Vec::new();
    planes.try_reserve_exact(header.channels).map_err(|_| {
        err(
            ImgErrorKind::OutOfMemory,
            "cannot allocate PSD color planes",
        )
    })?;
    for chunk in data.chunks_exact(plane_len) {
        planes.push(Some(chunk.to_vec()));
    }
    Ok(planes)
}

fn parse_psd(data: &[u8]) -> Result<ParsedPsd, Error> {
    let mut cursor = Cursor::new(data);
    let header = parse_header(&mut cursor)?;
    let color_mode_data = cursor.sized_u32("color mode data")?;
    let palette = if header.mode == 2 {
        if color_mode_data.len() != 768 {
            return Err(err(
                ImgErrorKind::IllegalData,
                "PSD indexed palette must contain 768 bytes",
            ));
        }
        Some(color_mode_data.to_vec())
    } else {
        None
    };

    let resources = cursor.sized_u32("image resources")?;
    let mut metadata = HashMap::new();
    parse_resources(resources, &mut metadata)?;
    let layer_data = cursor.sized_u32("layer and mask information")?;
    let (layer_record_count, layers) = parse_layers(layer_data, header, palette.as_deref())?;

    let compression = cursor.u16()?;
    let planar = decompress_planar(
        compression,
        cursor.read(cursor.remaining())?,
        header.width,
        header.height,
        header.channels,
        header.depth,
    )?;
    let planes = split_planes(planar, header)?;
    let base = base_channels(header.mode)?;
    let alpha = planes.get(base).and_then(Option::as_deref);
    let composite = planes_to_rgba(
        RasterFormat {
            width: header.width,
            height: header.height,
            depth: header.depth,
            mode: header.mode,
            palette: palette.as_deref(),
        },
        &planes[..base],
        alpha,
        255,
    )?;

    metadata.insert("Format".to_string(), DataMap::Ascii("PSD".to_string()));
    metadata.insert("width".to_string(), DataMap::UInt(header.width as u64));
    metadata.insert("height".to_string(), DataMap::UInt(header.height as u64));
    metadata.insert(
        "channels".to_string(),
        DataMap::UInt(header.channels as u64),
    );
    metadata.insert(
        "bits per channel".to_string(),
        DataMap::UInt(header.depth as u64),
    );
    metadata.insert(
        "color mode".to_string(),
        DataMap::Ascii(color_mode_name(header.mode).to_string()),
    );
    metadata.insert(
        "compression".to_string(),
        DataMap::Ascii(compression_name(compression).to_string()),
    );
    metadata.insert(
        "wml2.psd.layer_record_count".to_string(),
        DataMap::UInt(layer_record_count as u64),
    );
    metadata.insert(
        "wml2.psd.layer_count".to_string(),
        DataMap::UInt(layers.len() as u64),
    );
    if !layers.is_empty() {
        metadata.insert(
            "wml2.psd.layer_model".to_string(),
            DataMap::Ascii("animation".to_string()),
        );
        for (index, layer) in layers.iter().enumerate() {
            let prefix = format!("wml2.psd.layer.{index}");
            metadata.insert(
                format!("{prefix}.source_index"),
                DataMap::UInt(layer.source_index as u64),
            );
            metadata.insert(
                format!("{prefix}.name"),
                DataMap::I18NString(layer.name.clone()),
            );
            metadata.insert(
                format!("{prefix}.visible"),
                DataMap::UInt(u64::from(layer.visible)),
            );
            metadata.insert(
                format!("{prefix}.opacity"),
                DataMap::UInt(layer.opacity as u64),
            );
            metadata.insert(
                format!("{prefix}.blend_mode"),
                DataMap::Ascii(layer.blend_mode.clone()),
            );
        }
    }
    Ok(ParsedPsd {
        header,
        metadata,
        composite,
        layers,
    })
}

fn aborted(response: Option<CallbackResponse>) -> bool {
    response.is_some_and(|response| response.response == ResponseCommand::Abort)
}

fn emit(parsed: ParsedPsd, option: &mut DecodeOptions<'_>) -> Result<Option<ImgWarnings>, Error> {
    for (key, value) in parsed.metadata {
        if aborted(option.drawer.set_metadata(&key, value)?) {
            return Ok(None);
        }
    }
    let init = InitOptions {
        loop_count: 1,
        background: None,
        animation: !parsed.layers.is_empty(),
    };
    if aborted(
        option
            .drawer
            .init(parsed.header.width, parsed.header.height, Some(init))?,
    ) {
        return Ok(None);
    }
    if aborted(option.drawer.draw(
        0,
        0,
        parsed.header.width,
        parsed.header.height,
        &parsed.composite,
        None,
    )?) {
        return Ok(None);
    }
    for layer in parsed.layers {
        let next = NextOptions {
            flag: NextOption::Continue,
            await_time: 0,
            image_rect: Some(ImageRect {
                start_x: layer.left,
                start_y: layer.top,
                width: layer.width,
                height: layer.height,
            }),
            dispose_option: Some(NextDispose::None),
            blend: Some(NextBlend::Source),
        };
        if aborted(option.drawer.next(Some(next))?) {
            return Ok(None);
        }
        if aborted(
            option
                .drawer
                .draw(0, 0, layer.width, layer.height, &layer.rgba, None)?,
        ) {
            return Ok(None);
        }
    }
    let _ = option.drawer.terminate(None)?;
    Ok(None)
}

/// Decodes a PSD v1 image into WML2 callbacks.
pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions<'_>,
) -> Result<Option<ImgWarnings>, Error> {
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let length = usize::try_from(end.checked_sub(start).ok_or_else(|| {
        err(
            ImgErrorKind::InvalidParameter,
            "PSD reader end precedes its current offset",
        )
    })?)
    .map_err(|_| err(ImgErrorKind::InvalidParameter, "PSD file is too large"))?;
    let data = reader.read_bytes_as_vec(length)?;
    emit(parse_psd(&data)?, option)
}
