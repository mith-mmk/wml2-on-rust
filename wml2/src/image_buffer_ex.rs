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
/// and must have their high four bits clear. Valid integer ranges are
/// 0..=255, 0..=4095, and 0..=65535 respectively. RGBA uses straight alpha.
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

/// Explicit output request for conversion or EX pixel retrieval. The format
/// and precision policy must be supplied; there is no implicit narrowing.
/// `stride` is in bytes for the requested rectangle, with padding after every
/// row including the last. `None` selects packed output; padding is zeroed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelOutputEx {
    pub format: PixelFormatEx,
    pub stride: Option<usize>,
    pub conversion: PrecisionConversion,
}
impl PixelOutputEx {
    pub fn new(format: PixelFormatEx, conversion: PrecisionConversion) -> Self {
        Self {
            format,
            stride: None,
            conversion,
        }
    }
}

/// Owned bytes returned by an EX conversion or picker, with their actual
/// layout. Dimensions and byte layout are checked before construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelBytesEx {
    width: usize,
    height: usize,
    format: PixelFormatEx,
    stride: usize,
    bytes: Vec<u8>,
}
impl PixelBytesEx {
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
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// EX storage declaration alongside the unchanged legacy image profile.
/// `format` and `stride` describe the stored canvas; each pick independently
/// requests its output layout. Background, if present, remains legacy RGBA8.
#[derive(Debug)]
pub struct ImageProfilesEx {
    pub legacy: ImageProfiles,
    pub format: PixelFormatEx,
    pub stride: usize,
}

/// Supplies explicitly requested Gray/RGBA bytes to EX-aware consumers.
/// This separate, feature-gated trait does not change legacy `PickCallback`
/// implementers or the byte format consumed by existing codecs.
pub trait PickCallbackEx: Sync + Send {
    /// Reports the stored canvas. The encoder options retain their legacy
    /// meaning; this does not negotiate or select a codec's native format.
    fn encode_start_ex(
        &mut self,
        option: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfilesEx>, Error>;
    /// Reads a nonempty rectangle fully inside the stored canvas, converting
    /// storage to the explicit output request. Bounds/stride/precision errors
    /// return no output and leave storage unchanged. Legacy pick options are
    /// supplied separately so their meaning remains unchanged.
    fn encode_pick_ex(
        &mut self,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
        output: PixelOutputEx,
        option: Option<PickOptions>,
    ) -> Result<Option<PixelBytesEx>, Error>;
    fn encode_end_ex(&mut self, option: Option<EndOptions>) -> Result<(), Error>;
    /// Returns legacy callback metadata, independently of highres frame/color
    /// information or timing.
    fn metadata_ex(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error>;
}

/// Converts declared source bytes into an explicitly requested byte layout.
/// Source length must equal its stride times height, including final padding.
/// Every addressed source sample is validated; padding bytes are ignored.
/// Odd byte strides are supported without alignment casts. Gray expands to
/// RGB with opaque alpha; RGBA to Gray is unsupported. Exact/Round scale the
/// normalized integer range, including alpha, without color/ICC/HDR transforms.
/// Pixel and source/output byte budgets are checked before output allocation.
///
/// ```
/// use wml2::draw::{convert_pixels_ex, PixelFormatEx, PixelOutputEx,
///     PrecisionConversion};
/// let rgba = convert_pixels_ex(&[0xff, 0x0f], 1, 1,
///     PixelFormatEx::Gray12, None,
///     PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact))?;
/// assert_eq!(rgba.bytes(), &[0xff; 8]);
/// assert_eq!(rgba.stride(), 8);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn convert_pixels_ex(
    source: &[u8],
    width: usize,
    height: usize,
    source_format: PixelFormatEx,
    source_stride: Option<usize>,
    output: PixelOutputEx,
) -> Result<PixelBytesEx, Error> {
    let (source_stride, length) = source_format.extent(width, height, source_stride)?;
    check_pixel_budget(width, height)?;
    crate::limits::check(
        length,
        crate::limits::current().expanded_bytes,
        "EX conversion source",
    )?;
    validate_bytes(source, width, height, source_format, source_stride, length)?;
    convert_validated_region(source, width, height, source_format, source_stride, output)
}

fn check_pixel_budget(width: usize, height: usize) -> Result<(), Error> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| ex_error("EX pixel count overflow"))?;
    crate::limits::check(pixels, crate::limits::current().pixels, "EX pixels")
}

// The source region has already been validated against its owning canvas or
// public source declaration. Its final row need not include canvas padding.
fn convert_validated_region(
    source: &[u8],
    width: usize,
    height: usize,
    source_format: PixelFormatEx,
    source_stride: usize,
    output: PixelOutputEx,
) -> Result<PixelBytesEx, Error> {
    check_conversion(source_format, output.format)?;
    let (stride, length) = output.format.extent(width, height, output.stride)?;
    check_pixel_budget(width, height)?;
    crate::limits::check(
        length,
        crate::limits::current().expanded_bytes,
        "EX conversion output",
    )?;
    let mut bytes = zeroed_bytes(length)?;
    convert_rows(
        &mut bytes,
        source,
        (width, height),
        (source_format, source_stride),
        (output.format, stride),
        output.conversion,
    )?;
    Ok(PixelBytesEx {
        width,
        height,
        format: output.format,
        stride,
        bytes,
    })
}

fn pick_region(
    source: &[u8],
    canvas: (usize, usize),
    storage: (PixelFormatEx, usize),
    start: (usize, usize),
    size: (usize, usize),
    output: PixelOutputEx,
) -> Result<PixelBytesEx, Error> {
    let (x, y) = start;
    let (width, height) = size;
    let (format, stride) = storage;
    let (canvas_width, canvas_height) = canvas;
    if width == 0 || height == 0 {
        return Err(ex_error("EX pick dimensions must be nonzero"));
    }
    let end_x = x
        .checked_add(width)
        .ok_or_else(|| ex_error("EX pick x overflow"))?;
    let end_y = y
        .checked_add(height)
        .ok_or_else(|| ex_error("EX pick y overflow"))?;
    if end_x > canvas_width || end_y > canvas_height {
        return Err(ex_error("EX pick rectangle exceeds canvas"));
    }
    let offset = y
        .checked_mul(stride)
        .and_then(|row| {
            x.checked_mul(format.bytes_per_pixel())
                .and_then(|column| row.checked_add(column))
        })
        .ok_or_else(|| ex_error("EX pick offset overflow"))?;
    convert_validated_region(&source[offset..], width, height, format, stride, output)
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
/// Legacy encoders require an explicit `to_rgba8` precision conversion;
/// EX-aware consumers use `PickCallbackEx` with an explicit output request.
///
/// ```
/// use wml2::draw::{DrawCallback, ImageBufferEx, InitOptionsEx,
///     PixelFormatEx, PrecisionConversion};
/// let mut image = ImageBufferEx::with_storage(
///     PixelFormatEx::Rgba16, PrecisionConversion::Exact);
/// image.init_ex(1, 1, Some(InitOptionsEx {
///     source_format: PixelFormatEx::Gray12, ..Default::default()
/// })).unwrap();
/// image.draw(0, 0, 1, 1, &[0xff, 0x0f], None).unwrap();
/// assert_eq!(image.bytes().unwrap(), &[0xff; 8]);
/// let legacy = image.to_rgba8(PrecisionConversion::Exact).unwrap();
/// assert_eq!(legacy.buffer.unwrap(), [255; 4]);
/// ```
pub struct ImageBufferEx {
    width: usize,
    height: usize,
    format: PixelFormatEx,
    stride: usize,
    buffer: Option<Vec<u8>>,
    background_color: Option<RGBA>,
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
            background_color: None,
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

    /// Incoming draw layout from the last successful initialization, or the
    /// supplied layout of `from_bytes`. This is not the encoded file's color
    /// type: codecs may expand RGB, palettes or packed Gray before drawing.
    /// Returns None before initialization or after a failed initialization.
    pub fn source_format(&self) -> Option<PixelFormatEx> {
        self.buffer.as_ref().map(|_| self.source_format)
    }

    /// Explicit incoming stride per draw rectangle; None means packed for
    /// that rectangle's width. Meaningful while `source_format()` is Some;
    /// `from_bytes` retains its storage stride but subsequent draws are packed.
    /// This is independent of the stored canvas stride.
    pub fn source_stride(&self) -> Option<usize> {
        self.source_stride
    }

    /// The selected source-to-storage precision policy. Picker and legacy
    /// encoder requests independently choose their own conversion policy.
    pub fn precision_conversion(&self) -> PrecisionConversion {
        self.conversion
    }

    /// Borrows one stored pixel without allocation, conversion or narrowing.
    /// Interpret its bytes using `format()`: 12/16-bit samples are little-endian
    /// and RGBA alpha is straight. Row padding is never returned. Coordinates
    /// must lie inside the canvas; source canvas resource limits still apply.
    pub fn pixel_bytes(&self, x: usize, y: usize) -> Result<&[u8], Error> {
        let source = self.ex_pixels()?;
        if x >= self.width || y >= self.height {
            return Err(ex_error("EX pixel exceeds canvas"));
        }
        let length = self.format.bytes_per_pixel();
        let offset = y
            .checked_mul(self.stride)
            .and_then(|row| {
                x.checked_mul(length)
                    .and_then(|column| row.checked_add(column))
            })
            .ok_or_else(|| ex_error("EX pixel offset overflow"))?;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| ex_error("EX pixel extent overflow"))?;
        source
            .get(offset..end)
            .ok_or_else(|| ex_error("EX pixel storage is incomplete"))
    }

    /// Borrows this static canvas for existing RGBA8 encoders with an explicit
    /// precision policy. Each requested rectangle is converted independently;
    /// high-depth storage is retained and no whole-canvas RGBA8 copy is needed.
    /// Exact conversion may fail when a requested sample is not representable.
    pub fn legacy_picker(&self, conversion: PrecisionConversion) -> LegacyPickAdapterEx<'_> {
        LegacyPickAdapterEx {
            image: self,
            conversion,
        }
    }

    // Only the private fully decoded native path may supply owned packed bytes.
    // It shares every initialization check and retains the requested policy.
    #[cfg(feature = "jpeg2000")]
    pub(crate) fn from_decoded_bytes(
        width: usize,
        height: usize,
        format: PixelFormatEx,
        conversion: PrecisionConversion,
        bytes: Vec<u8>,
    ) -> Result<Self, Error> {
        let mut image = Self::with_storage(format, conversion);
        image.initialize_with_bytes(
            width,
            height,
            InitOptionsEx {
                source_format: format,
                source_stride: None,
                legacy: InitOptions::new(),
            },
            Some(bytes),
        )?;
        Ok(image)
    }

    fn initialize(&mut self, width: usize, height: usize, option: InitOptionsEx) -> Response {
        self.initialize_with_bytes(width, height, option, None)
    }

    fn initialize_with_bytes(
        &mut self,
        width: usize,
        height: usize,
        option: InitOptionsEx,
        owned: Option<Vec<u8>>,
    ) -> Response {
        // Failed initialization invalidates the old callback target.
        self.buffer = None;
        self.background_color = None;
        let (stride, length) = self.format.extent(width, height, None)?;
        let source_row = option.source_format.row_bytes(width)?;
        let source_stride = if let Some(stride) = option.source_stride {
            option.source_format.validate_stride(stride)?;
            stride.max(source_row)
        } else {
            source_row
        };
        let source_length = source_stride
            .checked_mul(height)
            .ok_or_else(|| ex_error("EX source size overflow"))?;
        if option.legacy.as_ref().is_some_and(|o| o.animation) {
            return Err(ex_error("EX animation initialization is unsupported"));
        }
        check_conversion(option.source_format, self.format)?;
        let limits = crate::limits::current();
        let pixels = width
            .checked_mul(height)
            .ok_or_else(|| ex_error("EX pixel count overflow"))?;
        crate::limits::check(pixels, limits.pixels, "EX pixels")?;
        crate::limits::check(source_length, limits.expanded_bytes, "EX source canvas")?;
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
        let buffer = if let Some(bytes) = owned {
            validate_bytes(&bytes, width, height, self.format, stride, length)?;
            bytes
        } else {
            initialized_bytes(
                length,
                background.map(|_| &pixel[..self.format.bytes_per_pixel()]),
            )?
        };
        self.width = width;
        self.height = height;
        self.stride = stride;
        self.source_format = option.source_format;
        self.source_stride = option.source_stride;
        self.background_color = background.cloned();
        self.buffer = Some(buffer);
        Ok(None)
    }

    /// Converts pixels for legacy encoders, retaining EX callback metadata.
    /// No animation, timing, or highres color information is synthesized.
    pub fn to_rgba8(&self, conversion: PrecisionConversion) -> Result<ImageBuffer, Error> {
        let bytes = self.converted_bytes(PixelFormatEx::Rgba8, conversion)?;
        let mut image = ImageBuffer::from_buffer(self.width, self.height, bytes);
        image.metadata = self.metadata.clone();
        image.background_color = self.background_color.clone();
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
        Ok(convert_pixels_ex(
            source,
            self.width,
            self.height,
            self.format,
            Some(self.stride),
            PixelOutputEx::new(format, conversion),
        )?
        .into_bytes())
    }
}

/// A checked static legacy RGBA8 buffer can transfer ownership into EX without
/// copying its pixels. Background and callback metadata are retained. The
/// publicly mutable dimensions and exact byte length are validated afresh.
/// Animation/frame/timing state is rejected rather than silently discarded.
/// The dormant zero loop count set by legacy static initialization is accepted.
impl TryFrom<ImageBuffer> for ImageBufferEx {
    type Error = Error;

    fn try_from(mut image: ImageBuffer) -> Result<Self, Error> {
        if image.animation.is_some()
            || image.current.is_some()
            || image.loop_count.is_some_and(|count| count != 0)
            || image.first_wait_time.is_some()
        {
            return Err(ex_error(
                "EX legacy bridge requires static pixels without animation state",
            ));
        }
        image.ex_pixels()?;
        let mut target = Self::from_bytes(
            image.width,
            image.height,
            PixelFormatEx::Rgba8,
            None,
            image
                .buffer
                .take()
                .ok_or_else(|| ex_error("EX buffer is not initialized"))?,
        )?;
        target.background_color = image.background_color;
        target.metadata = image.metadata;
        Ok(target)
    }
}

/// Explicit RGBA8 encoder view of an EX canvas. Obtained from `legacy_picker`;
/// it implements the unchanged legacy PickCallback and never mutates storage.
/// Rectangles must be nonempty and fully inside the canvas, as for EX picking.
pub struct LegacyPickAdapterEx<'a> {
    image: &'a ImageBufferEx,
    conversion: PrecisionConversion,
}

impl PickCallback for LegacyPickAdapterEx<'_> {
    fn encode_start(&mut self, _: Option<EncoderOptions>) -> Result<Option<ImageProfiles>, Error> {
        self.image.ex_pixels()?;
        Ok(Some(ImageProfiles {
            width: self.image.width,
            height: self.image.height,
            background: self.image.background_color.clone(),
            metadata: self.image.metadata.clone(),
        }))
    }

    fn encode_pick(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        _: Option<PickOptions>,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(Some(
            pick_region(
                self.image.ex_pixels()?,
                (self.image.width, self.image.height),
                (self.image.format, self.image.stride),
                (x, y),
                (width, height),
                PixelOutputEx::new(PixelFormatEx::Rgba8, self.conversion),
            )?
            .into_bytes(),
        ))
    }

    fn encode_end(&mut self, _: Option<EndOptions>) -> Result<(), Error> {
        Ok(())
    }

    fn metadata(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error> {
        Ok(self.image.metadata.clone())
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
        check_pixel_budget(width, height)?;
        crate::limits::check(
            length,
            crate::limits::current().expanded_bytes,
            "EX draw source",
        )?;
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
            let converted = convert_validated_region(
                data,
                width,
                height,
                self.source_format,
                source_stride,
                PixelOutputEx::new(self.format, self.conversion),
            )?;
            if let Some((w, h)) = clip {
                copy_rows(
                    self.buffer.as_mut().unwrap(),
                    converted.bytes(),
                    converted.stride(),
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
    size: (usize, usize),
    source_layout: (PixelFormatEx, usize),
    output_layout: (PixelFormatEx, usize),
    conversion: PrecisionConversion,
) -> Result<(), Error> {
    let (width, height) = size;
    let (src, source_stride) = source_layout;
    let (dst, output_stride) = output_layout;
    check_conversion(src, dst)?;
    if src == dst {
        let row = dst.row_bytes(width)?;
        copy_rows(
            destination,
            source,
            source_stride,
            output_stride,
            0,
            row,
            height,
        );
        return Ok(());
    }
    for y in 0..height {
        for x in 0..width {
            let base = y * source_stride + x * src.bytes_per_pixel();
            let pixel = convert_pixel(
                &source[base..base + src.bytes_per_pixel()],
                src,
                dst,
                conversion,
            )?;
            let length = dst.bytes_per_pixel();
            let offset = y * output_stride + x * length;
            destination[offset..offset + length].copy_from_slice(&pixel[..length]);
        }
    }
    Ok(())
}

impl ImageBufferEx {
    fn ex_pixels(&self) -> Result<&[u8], Error> {
        let source = self
            .buffer
            .as_deref()
            .ok_or_else(|| ex_error("EX buffer is not initialized"))?;
        check_pixel_budget(self.width, self.height)?;
        crate::limits::check(
            source.len(),
            crate::limits::current().expanded_bytes,
            "EX picker source",
        )?;
        Ok(source)
    }
}

impl PickCallbackEx for ImageBufferEx {
    fn encode_start_ex(
        &mut self,
        _: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfilesEx>, Error> {
        self.ex_pixels()?;
        Ok(Some(ImageProfilesEx {
            legacy: ImageProfiles {
                width: self.width,
                height: self.height,
                background: self.background_color.clone(),
                metadata: self.metadata.clone(),
            },
            format: self.format,
            stride: self.stride,
        }))
    }
    fn encode_pick_ex(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        output: PixelOutputEx,
        _: Option<PickOptions>,
    ) -> Result<Option<PixelBytesEx>, Error> {
        let source = self.ex_pixels()?;
        Ok(Some(pick_region(
            source,
            (self.width, self.height),
            (self.format, self.stride),
            (x, y),
            (width, height),
            output,
        )?))
    }
    fn encode_end_ex(&mut self, _: Option<EndOptions>) -> Result<(), Error> {
        Ok(())
    }
    fn metadata_ex(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error> {
        Ok(self.metadata.clone())
    }
}

impl ImageBuffer {
    // The publicly mutable legacy fields are validated afresh for the strict
    // EX entry points, without changing the old permissive encoder behavior.
    fn ex_pixels(&self) -> Result<(&[u8], usize), Error> {
        if self
            .animation
            .as_ref()
            .is_some_and(|frames| !frames.is_empty())
        {
            return Err(ex_error("EX animation picking is unsupported"));
        }
        let source = self
            .buffer
            .as_deref()
            .ok_or_else(|| ex_error("EX buffer is not initialized"))?;
        let (stride, length) = PixelFormatEx::Rgba8.extent(self.width, self.height, None)?;
        check_pixel_budget(self.width, self.height)?;
        crate::limits::check(
            length,
            crate::limits::current().expanded_bytes,
            "EX picker source",
        )?;
        validate_bytes(
            source,
            self.width,
            self.height,
            PixelFormatEx::Rgba8,
            stride,
            length,
        )?;
        Ok((source, stride))
    }
}
impl PickCallbackEx for ImageBuffer {
    fn encode_start_ex(
        &mut self,
        option: Option<EncoderOptions>,
    ) -> Result<Option<ImageProfilesEx>, Error> {
        let (_, stride) = self.ex_pixels()?;
        Ok(
            PickCallback::encode_start(self, option)?.map(|legacy| ImageProfilesEx {
                legacy,
                format: PixelFormatEx::Rgba8,
                stride,
            }),
        )
    }
    fn encode_pick_ex(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        output: PixelOutputEx,
        _: Option<PickOptions>,
    ) -> Result<Option<PixelBytesEx>, Error> {
        let (source, stride) = self.ex_pixels()?;
        Ok(Some(pick_region(
            source,
            (self.width, self.height),
            (PixelFormatEx::Rgba8, stride),
            (x, y),
            (width, height),
            output,
        )?))
    }
    fn encode_end_ex(&mut self, option: Option<EndOptions>) -> Result<(), Error> {
        PickCallback::encode_end(self, option)
    }
    fn metadata_ex(&mut self) -> Result<Option<HashMap<String, DataMap>>, Error> {
        PickCallback::metadata(self)
    }
}

#[cfg(feature = "high-bit-depth")]
impl ImageBufferEx {
    /// Copies only pixels from a typed highres frame. Metadata, timing, and
    /// color information remain on the original frame; no such data is mapped
    /// to the unrelated legacy callback metadata. Full-resolution Gray without
    /// alpha or straight RGBA at uniformly 8/12/16 bits is supported. Eight-bit
    /// samples may reside in U8 or U16 planes. Planar,
    /// interleaved, and padded highres layouts are read by their channel roles.
    pub fn from_highres_pixels(frame: &crate::highres::ImageFrame) -> Result<Self, Error> {
        use crate::highres::{
            AlphaAssociation, ChannelModel, ChannelRole, PixelBuffer, Subsampling,
        };
        let descriptor = frame.descriptor();
        let width = usize::try_from(descriptor.width())?;
        let height = usize::try_from(descriptor.height())?;
        check_pixel_budget(width, height)?;
        frame.validate()?;
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
                    "EX highres adapter supports only U8/8 and U16/8/12/16",
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

#[cfg(test)]
mod conversion_budget_tests {
    use super::*;
    #[cfg(feature = "high-bit-depth")]
    #[test]
    fn highres_import_checks_pixel_budget_before_scanning_or_copying() {
        use crate::highres::{ImageDescriptor, ImageFrame, PixelBuffer, Plane};
        let descriptor = ImageDescriptor::gray(2, 1, 12).unwrap();
        let pixels = PixelBuffer::u16(vec![
            Plane::new(descriptor.planes()[0].layout().clone(), vec![1, 4095]).unwrap(),
        ])
        .unwrap();
        let mut frame = ImageFrame::new(descriptor, pixels).unwrap();
        crate::limits::scope(
            crate::limits::DecodeLimits {
                pixels: 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || {
                let error = ImageBufferEx::from_highres_pixels(&frame).err().unwrap();
                assert!(error.to_string().contains("EX pixels"));
            },
        );
        // Malformed samples would be scanned by validate: the pixel limit must
        // take precedence, demonstrating it is checked before the copy phase.
        frame.pixels_mut().u16_planes_mut().unwrap()[0].samples_mut()[1] = 0xf000;
        crate::limits::scope(
            crate::limits::DecodeLimits {
                pixels: 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || {
                let error = ImageBufferEx::from_highres_pixels(&frame).err().unwrap();
                assert!(error.to_string().contains("EX pixels"));
            },
        );
        assert!(ImageBufferEx::from_highres_pixels(&frame).is_err());
    }
    #[test]
    fn converter_and_pickers_enforce_source_output_and_canvas_budgets() {
        let mut image =
            ImageBufferEx::from_bytes(2, 1, PixelFormatEx::Rgba16, None, vec![0; 16]).unwrap();
        let mut legacy = ImageBuffer::from_buffer(2, 1, vec![0; 8]);
        let output = PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact);
        crate::limits::scope(
            crate::limits::DecodeLimits {
                expanded_bytes: 8,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || {
                let error = convert_pixels_ex(&[0; 16], 2, 1, PixelFormatEx::Rgba16, None, output)
                    .unwrap_err();
                assert!(error.to_string().contains("source"));
                assert!(image.encode_start_ex(None).is_err());
                assert!(image.encode_pick_ex(0, 0, 1, 1, output, None).is_err());
                assert!(image.pixel_bytes(0, 0).is_err());
                assert!(
                    image
                        .legacy_picker(PrecisionConversion::Round)
                        .encode_start(None)
                        .is_err()
                );
                assert!(
                    image
                        .legacy_picker(PrecisionConversion::Round)
                        .encode_pick(0, 0, 1, 1, None)
                        .is_err()
                );
                legacy.encode_start_ex(None).unwrap();
                legacy.encode_pick_ex(0, 0, 1, 1, output, None).unwrap();
                let error = convert_pixels_ex(
                    &[0; 2],
                    2,
                    1,
                    PixelFormatEx::Gray8,
                    None,
                    PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
                )
                .unwrap_err();
                assert!(error.to_string().contains("output"));
            },
        );
        crate::limits::scope(
            crate::limits::DecodeLimits {
                pixels: 1,
                ..crate::limits::DecodeLimits::unlimited()
            },
            || {
                assert!(
                    convert_pixels_ex(&[0; 2], 2, 1, PixelFormatEx::Gray8, None, output).is_err()
                );
                assert!(image.encode_start_ex(None).is_err());
                assert!(image.encode_pick_ex(0, 0, 1, 1, output, None).is_err());
                assert!(legacy.encode_start_ex(None).is_err());
                assert!(legacy.encode_pick_ex(0, 0, 1, 1, output, None).is_err());
                assert!(image.pixel_bytes(0, 0).is_err());
                assert!(
                    image
                        .legacy_picker(PrecisionConversion::Round)
                        .encode_start(None)
                        .is_err()
                );
                assert!(
                    ImageBufferEx::try_from(ImageBuffer::from_buffer(2, 1, vec![0; 8])).is_err()
                );
            },
        );
        // A scoped failure doesn't alter the stored pixels or future validity.
        image.encode_start_ex(None).unwrap();
        assert_eq!(image.bytes().unwrap(), [0; 16]);
    }
}

#[cfg(all(test, feature = "jpeg2000"))]
mod decoded_owned_tests {
    use super::*;
    #[test]
    fn decoded_native_bytes_move_with_policy_and_validate_before_adoption() {
        let bytes = vec![1, 0, 255, 15];
        let pointer = bytes.as_ptr();
        let image = ImageBufferEx::from_decoded_bytes(
            2,
            1,
            PixelFormatEx::Gray12,
            PrecisionConversion::Round,
            bytes,
        )
        .unwrap();
        assert_eq!(image.bytes().unwrap().as_ptr(), pointer);
        assert_eq!(image.bytes().unwrap(), &[1, 0, 255, 15]);
        assert_eq!(image.source_format(), Some(PixelFormatEx::Gray12));
        assert_eq!(image.source_stride(), None);
        assert_eq!(image.precision_conversion(), PrecisionConversion::Round);
        assert!(
            ImageBufferEx::from_decoded_bytes(
                1,
                1,
                PixelFormatEx::Gray12,
                PrecisionConversion::Exact,
                vec![0, 16]
            )
            .is_err()
        );
        assert!(
            ImageBufferEx::from_decoded_bytes(
                1,
                1,
                PixelFormatEx::Gray16,
                PrecisionConversion::Exact,
                vec![0]
            )
            .is_err()
        );
    }
    #[test]
    fn decoded_owned_initialization_keeps_storage_and_animation_limits() {
        for (expanded, animation) in [(1, 2), (2, 1)] {
            crate::limits::scope(
                crate::limits::DecodeLimits {
                    expanded_bytes: expanded,
                    animation_bytes: animation,
                    ..Default::default()
                },
                || {
                    assert!(
                        ImageBufferEx::from_decoded_bytes(
                            1,
                            1,
                            PixelFormatEx::Gray16,
                            PrecisionConversion::Exact,
                            vec![0, 0]
                        )
                        .is_err()
                    );
                },
            );
        }
        assert!(
            ImageBufferEx::from_decoded_bytes(
                1,
                1,
                PixelFormatEx::Gray16,
                PrecisionConversion::Exact,
                vec![0, 0]
            )
            .is_ok()
        );
    }
}
