//! Byte storage with an explicit source declaration and independent storage layout.
use super::*;

fn ex_error(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::InvalidParameter,
        message.to_string(),
    ))
}

impl InitOptions {
    /// Adds a source byte declaration without changing legacy option fields.
    pub fn with_format(self, source_format: PixelFormatEx) -> InitOptionsEx {
        InitOptionsEx {
            source_format,
            source_stride: None,
            legacy: Some(self),
        }
    }
}

/// Supported interleaved byte layouts. No alignment of the byte buffer is assumed.
/// 12/16-bit samples are little-endian u16; 12-bit samples use the low 12 bits
/// and must have their high four bits clear. RGBA uses straight alpha.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormatEx {
    Gray8,
    Gray12,
    Gray16,
    #[default]
    Rgba8,
    Rgba12,
    Rgba16,
}
impl PixelFormatEx {
    pub const fn channels(self) -> usize {
        match self {
            Self::Gray8 | Self::Gray12 | Self::Gray16 => 1,
            _ => 4,
        }
    }
    pub const fn bits(self) -> u8 {
        match self {
            Self::Gray8 | Self::Rgba8 => 8,
            Self::Gray12 | Self::Rgba12 => 12,
            _ => 16,
        }
    }
    pub const fn bytes_per_sample(self) -> usize {
        if self.bits() == 8 { 1 } else { 2 }
    }
    pub const fn bytes_per_pixel(self) -> usize {
        self.channels() * self.bytes_per_sample()
    }
    pub(crate) fn row_bytes(self, width: usize) -> Result<usize, Error> {
        width
            .checked_mul(self.bytes_per_pixel())
            .ok_or_else(|| ex_error("EX row size overflow"))
    }
    pub(crate) fn validate_stride(self, stride: usize) -> Result<(), Error> {
        if stride == 0 {
            return Err(ex_error("EX stride must be positive"));
        }
        Ok(())
    }
    pub(crate) fn extent(
        self,
        width: usize,
        height: usize,
        stride: Option<usize>,
    ) -> Result<(usize, usize), Error> {
        if width == 0 || height == 0 {
            return Err(ex_error("EX dimensions must be nonzero"));
        }
        let row = self.row_bytes(width)?;
        let stride = stride.unwrap_or(row);
        self.validate_stride(stride)?;
        if stride < row {
            return Err(ex_error("EX stride is shorter than a pixel row"));
        }
        let length = stride
            .checked_mul(height)
            .ok_or_else(|| ex_error("EX buffer size overflow"))?;
        Ok((stride, length))
    }
    fn max(self) -> u32 {
        (1u32 << self.bits()) - 1
    }
}

/// Precision changes scale the complete normalized range, including alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrecisionConversion {
    /// Reject any sample whose scaled value is not an integer.
    #[default]
    Exact,
    /// Round scaled samples to the nearest integer (ties upward).
    Round,
}

/// Describes incoming bytes, never the drawer's internal storage or a request
/// to a codec. `source_stride` is in bytes for each draw rectangle, including
/// padding after its final row; `None` means packed for that rectangle's width.
/// A tile stride may be smaller than a canvas row and is checked per draw.
#[derive(Debug, Default)]
pub struct InitOptionsEx {
    pub source_format: PixelFormatEx,
    pub source_stride: Option<usize>,
    /// Legacy background is always RGBA8. Its normalized range is scaled using
    /// the buffer's conversion policy; Gray storage accepts only opaque neutral
    /// backgrounds. Animation is currently unsupported.
    pub legacy: Option<InitOptions>,
}

/// Checked byte-based static image storage. Initialization declares incoming
/// bytes separately from the storage selected by `with_storage`. Both callback
/// initializers enter the same private routine without calling each other.
/// Encoders require an explicit `to_rgba8` precision conversion.
pub struct ImageBufferEx {
    width: usize,
    height: usize,
    format: PixelFormatEx,
    stride: usize,
    buffer: Option<Vec<u8>>,
    source_format: PixelFormatEx,
    source_stride: Option<usize>,
    conversion: PrecisionConversion,
    pub metadata: Option<HashMap<String, DataMap>>,
}
impl Default for ImageBufferEx {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageBufferEx {
    pub fn new() -> Self {
        Self::with_storage(PixelFormatEx::Rgba8, PrecisionConversion::Exact)
    }
    pub fn with_storage(format: PixelFormatEx, conversion: PrecisionConversion) -> Self {
        Self {
            width: 0,
            height: 0,
            format,
            stride: 0,
            buffer: None,
            source_format: PixelFormatEx::Rgba8,
            source_stride: None,
            conversion,
            metadata: None,
        }
    }
    /// Validates every addressed sample and requires exactly stride * height
    /// bytes, including final row padding. Padding bytes are uninterpreted.
    pub fn from_bytes(
        width: usize,
        height: usize,
        format: PixelFormatEx,
        stride: Option<usize>,
        bytes: Vec<u8>,
    ) -> Result<Self, Error> {
        let (stride, length) = format.extent(width, height, stride)?;
        let limits = crate::limits::current();
        crate::limits::check(
            width
                .checked_mul(height)
                .ok_or_else(|| ex_error("EX pixel count overflow"))?,
            limits.pixels,
            "EX pixels",
        )?;
        crate::limits::check(length, limits.expanded_bytes, "EX storage")?;
        validate_bytes(&bytes, width, height, format, stride, length)?;
        let mut image = Self::with_storage(format, PrecisionConversion::Exact);
        image.width = width;
        image.height = height;
        image.stride = stride;
        image.source_format = format;
        image.buffer = Some(bytes);
        Ok(image)
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn format(&self) -> PixelFormatEx {
        self.format
    }
    pub fn stride(&self) -> usize {
        self.stride
    }
    pub fn bytes(&self) -> Option<&[u8]> {
        self.buffer.as_deref()
    }

    fn initialize(&mut self, width: usize, height: usize, option: InitOptionsEx) -> Response {
        // Failed initialization invalidates the old callback target.
        self.buffer = None;
        let (stride, length) = self.format.extent(width, height, None)?;
        option.source_format.row_bytes(width)?;
        if let Some(source_stride) = option.source_stride {
            option.source_format.validate_stride(source_stride)?;
            source_stride
                .checked_mul(height)
                .ok_or_else(|| ex_error("EX source size overflow"))?;
        }
        if option.legacy.as_ref().is_some_and(|o| o.animation) {
            return Err(ex_error("EX animation initialization is unsupported"));
        }
        check_conversion(option.source_format, self.format)?;
        let limits = crate::limits::current();
        let pixels = width
            .checked_mul(height)
            .ok_or_else(|| ex_error("EX pixel count overflow"))?;
        crate::limits::check(pixels, limits.pixels, "EX pixels")?;
        crate::limits::check(length, limits.expanded_bytes, "EX storage")?;
        crate::limits::check(length, limits.animation_bytes, "EX canvas storage")?;
        let background = option.legacy.as_ref().and_then(|o| o.background.as_ref());
        let pixel = if let Some(background) = background {
            let rgba = [
                background.red,
                background.green,
                background.blue,
                background.alpha,
            ];
            if self.format.channels() == 1 {
                if rgba[0] != rgba[1] || rgba[1] != rgba[2] || rgba[3] != 255 {
                    return Err(ex_error("Gray background must be opaque neutral RGBA8"));
                }
                convert_pixel(
                    &rgba[..1],
                    PixelFormatEx::Gray8,
                    self.format,
                    self.conversion,
                )?
            } else {
                convert_pixel(&rgba, PixelFormatEx::Rgba8, self.format, self.conversion)?
            }
        } else {
            [0u8; 8]
        };
        let buffer = initialized_bytes(
            length,
            background.map(|_| &pixel[..self.format.bytes_per_pixel()]),
        )?;
        self.width = width;
        self.height = height;
        self.stride = stride;
        self.source_format = option.source_format;
        self.source_stride = option.source_stride;
        self.buffer = Some(buffer);
        Ok(None)
    }

    /// Converts pixels for legacy encoders, retaining EX callback metadata.
    /// No animation, timing, or highres color information is synthesized.
    pub fn to_rgba8(&self, conversion: PrecisionConversion) -> Result<ImageBuffer, Error> {
        let bytes = self.converted_bytes(PixelFormatEx::Rgba8, conversion)?;
        let mut image = ImageBuffer::from_buffer(self.width, self.height, bytes);
        image.metadata = self.metadata.clone();
        Ok(image)
    }
    fn converted_bytes(
        &self,
        format: PixelFormatEx,
        conversion: PrecisionConversion,
    ) -> Result<Vec<u8>, Error> {
        let source = self
            .buffer
            .as_ref()
            .ok_or_else(|| ex_error("EX buffer is not initialized"))?;
        let (_, length) = format.extent(self.width, self.height, None)?;
        crate::limits::check(
            length,
            crate::limits::current().expanded_bytes,
            "EX conversion",
        )?;
        let mut output = zeroed_bytes(length)?;
        convert_rows(
            &mut output,
            source,
            self.width,
            self.height,
            self.stride,
            self.format,
            format,
            conversion,
        )?;
        Ok(output)
    }
}
impl DrawCallback for ImageBufferEx {
    fn init(&mut self, width: usize, height: usize, option: Option<InitOptions>) -> Response {
        self.initialize(
            width,
            height,
            InitOptionsEx {
                legacy: option,
                ..Default::default()
            },
        )
    }
    fn init_ex(&mut self, width: usize, height: usize, option: Option<InitOptionsEx>) -> Response {
        self.initialize(width, height, option.unwrap_or_default())
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        data: &[u8],
        _: Option<DrawOptions>,
    ) -> Response {
        if self.buffer.is_none() {
            return Err(ex_error("EX buffer is not initialized"));
        }
        let (source_stride, length) =
            self.source_format
                .extent(width, height, self.source_stride)?;
        validate_bytes(
            data,
            width,
            height,
            self.source_format,
            source_stride,
            length,
        )?;
        x.checked_add(width)
            .ok_or_else(|| ex_error("EX rectangle x overflow"))?;
        y.checked_add(height)
            .ok_or_else(|| ex_error("EX rectangle y overflow"))?;
        let clip = clipped_rect(x, y, width, height, self.width, self.height)?;
        // Validate precision even in the clipped-away portion, before mutation.
        if self.source_format != self.format {
            let (_, length) = self.format.extent(width, height, None)?;
            crate::limits::check(
                length,
                crate::limits::current().expanded_bytes,
                "EX conversion",
            )?;
            let mut converted = zeroed_bytes(length)?;
            convert_rows(
                &mut converted,
                data,
                width,
                height,
                source_stride,
                self.source_format,
                self.format,
                self.conversion,
            )?;
            if let Some((w, h)) = clip {
                copy_rows(
                    self.buffer.as_mut().unwrap(),
                    &converted,
                    self.format.row_bytes(width)?,
                    self.stride,
                    y * self.stride + x * self.format.bytes_per_pixel(),
                    self.format.row_bytes(w)?,
                    h,
                );
            }
        } else if let Some((w, h)) = clip {
            copy_rows(
                self.buffer.as_mut().unwrap(),
                data,
                source_stride,
                self.stride,
                y * self.stride + x * self.format.bytes_per_pixel(),
                self.format.row_bytes(w)?,
                h,
            );
        }
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        Err(ex_error("EX animation frames are unsupported"))
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        Ok(None)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
        Ok(None)
    }
}
fn check_conversion(source: PixelFormatEx, destination: PixelFormatEx) -> Result<(), Error> {
    if source.channels() == 4 && destination.channels() == 1 {
        return Err(ex_error(
            "RGBA to Gray requires an explicit color conversion",
        ));
    }
    Ok(())
}
pub(crate) fn validate_bytes(
    data: &[u8],
    width: usize,
    height: usize,
    format: PixelFormatEx,
    stride: usize,
    length: usize,
) -> Result<(), Error> {
    if data.len() != length {
        return Err(ex_error("EX buffer length must equal stride * height"));
    }
    if format.bits() == 12 {
        let row = format.row_bytes(width)?;
        for y in 0..height {
            for sample in data[y * stride..y * stride + row].chunks_exact(2) {
                if sample[1] & 0xf0 != 0 {
                    return Err(ex_error("EX 12-bit sample has nonzero high bits"));
                }
            }
        }
    }
    Ok(())
}
fn convert_pixel(
    source: &[u8],
    src: PixelFormatEx,
    dst: PixelFormatEx,
    policy: PrecisionConversion,
) -> Result<[u8; 8], Error> {
    check_conversion(src, dst)?;
    let mut output = [0u8; 8];
    for channel in 0..dst.channels() {
        let value = if src.channels() == 1 && channel == 3 {
            src.max()
        } else {
            let offset = if src.channels() == 1 {
                0
            } else {
                channel * src.bytes_per_sample()
            };
            if src.bits() == 8 {
                u32::from(source[offset])
            } else {
                u32::from(u16::from_le_bytes([source[offset], source[offset + 1]]))
            }
        };
        let numerator = u64::from(value) * u64::from(dst.max());
        let denominator = u64::from(src.max());
        if policy == PrecisionConversion::Exact && numerator % denominator != 0 {
            return Err(ex_error("EX precision conversion would lose information"));
        }
        let value = ((numerator
            + if policy == PrecisionConversion::Round {
                denominator / 2
            } else {
                0
            })
            / denominator) as u16;
        let offset = channel * dst.bytes_per_sample();
        if dst.bits() == 8 {
            output[offset] = value as u8;
        } else {
            output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
        }
    }
    Ok(output)
}
fn convert_rows(
    destination: &mut [u8],
    source: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    src: PixelFormatEx,
    dst: PixelFormatEx,
    conversion: PrecisionConversion,
) -> Result<(), Error> {
    check_conversion(src, dst)?;
    if src == dst {
        let row = dst.row_bytes(width)?;
        copy_rows(destination, source, stride, row, 0, row, height);
        return Ok(());
    }
    let mut offset = 0;
    for y in 0..height {
        for x in 0..width {
            let base = y * stride + x * src.bytes_per_pixel();
            let pixel = convert_pixel(
                &source[base..base + src.bytes_per_pixel()],
                src,
                dst,
                conversion,
            )?;
            let length = dst.bytes_per_pixel();
            destination[offset..offset + length].copy_from_slice(&pixel[..length]);
            offset += length;
        }
    }
    Ok(())
}

#[cfg(feature = "high-bit-depth")]
impl ImageBufferEx {
    /// Copies only pixels from a typed highres frame. Metadata, timing, and
    /// color information remain on the original frame; no such data is mapped
    /// to the unrelated legacy callback metadata. Full-resolution Gray without
    /// alpha or straight RGBA at uniformly 8/12/16 bits is supported. Planar,
    /// interleaved, and padded highres layouts are read by their channel roles.
    pub fn from_highres_pixels(frame: &crate::highres::ImageFrame) -> Result<Self, Error> {
        use crate::highres::{
            AlphaAssociation, ChannelModel, ChannelRole, PixelBuffer, Subsampling,
        };
        frame.validate()?;
        let descriptor = frame.descriptor();
        let gray = descriptor.model() == ChannelModel::Gray
            && descriptor.alpha() == AlphaAssociation::None;
        let rgba = descriptor.model() == ChannelModel::RGB
            && descriptor.alpha() == AlphaAssociation::Straight;
        if !gray && !rgba {
            return Err(ex_error(
                "EX highres adapter requires Gray or straight RGBA",
            ));
        }
        let bits = descriptor.planes()[0].meaningful_bits();
        if descriptor
            .planes()
            .iter()
            .any(|p| p.meaningful_bits() != bits || p.layout().subsampling() != Subsampling::FULL)
        {
            return Err(ex_error(
                "EX highres adapter requires uniform full-resolution channels",
            ));
        }
        let format = match (gray, bits, frame.pixels()) {
            (true, 8, PixelBuffer::U8(_) | PixelBuffer::U16(_)) => PixelFormatEx::Gray8,
            (true, 12, PixelBuffer::U16(_)) => PixelFormatEx::Gray12,
            (true, 16, PixelBuffer::U16(_)) => PixelFormatEx::Gray16,
            (false, 8, PixelBuffer::U8(_) | PixelBuffer::U16(_)) => PixelFormatEx::Rgba8,
            (false, 12, PixelBuffer::U16(_)) => PixelFormatEx::Rgba12,
            (false, 16, PixelBuffer::U16(_)) => PixelFormatEx::Rgba16,
            _ => {
                return Err(ex_error(
                    "EX highres adapter supports only U8/8 and U16/12/16",
                ));
            }
        };
        let roles: &[ChannelRole] = if gray {
            &[ChannelRole::Gray]
        } else {
            &[
                ChannelRole::Red,
                ChannelRole::Green,
                ChannelRole::Blue,
                ChannelRole::Alpha,
            ]
        };
        let width = usize::try_from(descriptor.width())?;
        let height = usize::try_from(descriptor.height())?;
        let (_, length) = format.extent(width, height, None)?;
        crate::limits::check(
            length,
            crate::limits::current().expanded_bytes,
            "EX highres copy",
        )?;
        let mut bytes = zeroed_bytes(length)?;
        for (channel, role) in roles.iter().enumerate() {
            let (plane_index, channel_index) = descriptor
                .planes()
                .iter()
                .enumerate()
                .find_map(|(i, plane)| plane.roles().iter().position(|r| r == role).map(|c| (i, c)))
                .ok_or_else(|| ex_error("EX highres channel missing"))?;
            let layout = descriptor.planes()[plane_index].layout();
            for y in 0..height {
                for x in 0..width {
                    let index = y * layout.row_stride()
                        + x * layout.pixel_stride()
                        + layout.channel_offsets()[channel_index];
                    let offset = (y * width + x) * format.bytes_per_pixel()
                        + channel * format.bytes_per_sample();
                    match frame.pixels() {
                        PixelBuffer::U8(planes) => {
                            bytes[offset] = planes.as_slice()[plane_index].samples()[index]
                        }
                        PixelBuffer::U16(planes) => {
                            let value = planes.as_slice()[plane_index].samples()[index];
                            if bits == 8 {
                                bytes[offset] = u8::try_from(value)?;
                            } else {
                                bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
                            }
                        }
                        PixelBuffer::F32(_) => unreachable!(),
                    }
                }
            }
        }
        Self::from_bytes(width, height, format, None, bytes)
    }

    /// Copies only pixels to a typed highres frame with default unspecified
    /// color information and empty metadata/timing. This is not a frame metadata
    /// or color conversion. Callers must explicitly supply those separately.
    pub fn to_highres_pixels(&self) -> Result<crate::highres::ImageFrame, Error> {
        use crate::highres::{
            AlphaAssociation, ChannelModel, ChannelRole, ImageDescriptor, ImageFrame, PixelBuffer,
            Plane, PlaneDescriptor, PlaneLayout, Subsampling,
        };
        if self.buffer.is_none() {
            return Err(ex_error("EX buffer is not initialized"));
        }
        let width = u32::try_from(self.width)?;
        let height = u32::try_from(self.height)?;
        let roles = if self.format.channels() == 1 {
            vec![ChannelRole::Gray]
        } else {
            vec![
                ChannelRole::Red,
                ChannelRole::Green,
                ChannelRole::Blue,
                ChannelRole::Alpha,
            ]
        };
        let layout = PlaneLayout::interleaved(width, height, roles.len(), Subsampling::FULL)?;
        let descriptor = ImageDescriptor::new(
            width,
            height,
            if roles.len() == 1 {
                ChannelModel::Gray
            } else {
                ChannelModel::RGB
            },
            vec![PlaneDescriptor::new(
                layout.clone(),
                roles,
                self.format.bits(),
            )?],
        )?
        .with_alpha(if self.format.channels() == 1 {
            AlphaAssociation::None
        } else {
            AlphaAssociation::Straight
        })?;
        let packed = self.converted_bytes(self.format, PrecisionConversion::Exact)?;
        let pixels = if self.format.bits() == 8 {
            PixelBuffer::u8(vec![Plane::new(layout, packed)?])?
        } else {
            let mut samples = Vec::new();
            samples.try_reserve_exact(packed.len() / 2)?;
            samples.extend(
                packed
                    .chunks_exact(2)
                    .map(|s| u16::from_le_bytes([s[0], s[1]])),
            );
            PixelBuffer::u16(vec![Plane::new(layout, samples)?])?
        };
        Ok(ImageFrame::new(descriptor, pixels)?)
    }
}
