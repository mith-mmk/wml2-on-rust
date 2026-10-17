//! Explicit adapter from typed WML2 frames to the standalone AVIF encoder.

#[cfg(test)]
#[path = "avif_encode_bench.rs"]
mod review_bench;

use avifenc_codec::{EncoderError, FrameBuffers, PlaneBuffer, PlaneLayout};

pub use avifenc_codec::{NativeEncodeOptions, NativeSubsampling};

use super::{AlphaAssociation, ChannelModel, ChannelRole, HighresError, ImageFrame, PixelBuffer};

/// Integer conversion requested before encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantization {
    /// Require source meaningful precision to equal the output depth.
    None,
    /// Discard low-order bits when reducing an integer source to the explicit
    /// output depth. The shift is derived from the source descriptor.
    RightShift,
}

/// Errors raised while mapping one typed WML2 frame to AVIF native planes.
#[derive(Debug)]
pub enum EncodeError {
    Codec(EncoderError),
    Frame(HighresError),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(error) => error.fmt(formatter),
            Self::Frame(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EncodeError {}

impl From<EncoderError> for EncodeError {
    fn from(error: EncoderError) -> Self {
        Self::Codec(error)
    }
}

impl From<HighresError> for EncodeError {
    fn from(error: HighresError) -> Self {
        Self::Frame(error)
    }
}

/// Encode one Gray, RGB, or YCbCr typed frame with explicit native options.
///
/// The encoder options, including CICP and optional ICC signalling, are
/// supplied by the caller. No colour conversion or metadata inference is
/// performed by this adapter.
pub fn encode_native(
    frame: &ImageFrame,
    options: &NativeEncodeOptions,
) -> Result<Vec<u8>, EncodeError> {
    encode_native_with_quantization(frame, options, Quantization::None)
}

/// Encode one typed frame after an explicitly requested integer downshift.
///
/// `RightShift` is the only precision-reduction operation exposed here. It is
/// applied to U16 samples when their descriptor precision exceeds the
/// explicitly selected AVIF output depth. F32 input and implicit depth
/// inference remain unsupported.
pub fn encode_native_with_quantization(
    frame: &ImageFrame,
    options: &NativeEncodeOptions,
    quantization: Quantization,
) -> Result<Vec<u8>, EncodeError> {
    frame.validate()?;
    validate_options(frame, options)?;
    for plane in frame.descriptor().planes() {
        quantize(0, plane.meaningful_bits(), options.bit_depth, quantization)?;
    }
    let (color_roles, subsampling) = color_layout(frame.descriptor().model(), options.subsampling)?;
    // Reject a spatial format mismatch before extracting any owned plane.
    for (index, role) in color_roles.iter().copied().enumerate() {
        let (_, descriptor, _) = find_role(frame, role)?;
        let (sx, sy) = if index == 0 { (0, 0) } else { subsampling };
        let expected_width = frame.descriptor().width().div_ceil(1u32 << sx);
        let expected_height = frame.descriptor().height().div_ceil(1u32 << sy);
        if descriptor.layout().width() != expected_width
            || descriptor.layout().height() != expected_height
        {
            return Err(HighresError::Unsupported(
                "AVIF output subsampling must match the native source planes".into(),
            )
            .into());
        }
    }
    let mut planes = Vec::with_capacity(color_roles.len() + 1);
    for (plane_index, role) in color_roles.into_iter().enumerate() {
        let plane = encode_role(frame, role, options.bit_depth, quantization)?;
        let (subsampling_x, subsampling_y) = if plane_index == 0 {
            (0, 0)
        } else {
            subsampling
        };
        planes.push(plane_buffer(
            plane.samples,
            plane.width,
            plane.height,
            plane_index as u8,
            subsampling_x,
            subsampling_y,
        )?);
    }
    if frame
        .descriptor()
        .planes()
        .iter()
        .any(|plane| plane.roles().contains(&ChannelRole::Alpha))
    {
        if frame.descriptor().alpha() == AlphaAssociation::Premultiplied {
            return Err(HighresError::Unsupported(
                "premultiplied alpha requires explicit unpremultiplication before AVIF encoding"
                    .into(),
            )
            .into());
        }
        let alpha = encode_role(frame, ChannelRole::Alpha, options.bit_depth, quantization)?;
        planes.push(plane_buffer(
            alpha.samples,
            alpha.width,
            alpha.height,
            3,
            0,
            0,
        )?);
    }
    let buffers = FrameBuffers {
        width: usize::try_from(frame.descriptor().width()).map_err(|_| {
            HighresError::InvalidDimensions("image width does not fit the AVIF encoder".into())
        })?,
        height: usize::try_from(frame.descriptor().height()).map_err(|_| {
            HighresError::InvalidDimensions("image height does not fit the AVIF encoder".into())
        })?,
        planes,
    };
    Ok(avifenc_codec::encode_native_bytes(&buffers, options)?)
}

fn validate_options(frame: &ImageFrame, options: &NativeEncodeOptions) -> Result<(), HighresError> {
    if !matches!(options.bit_depth, 8 | 10 | 12) {
        return Err(HighresError::Unsupported(
            "AVIF high-bit-depth encoding requires an explicit 8, 10, or 12-bit depth".into(),
        ));
    }
    if frame.descriptor().alpha() == AlphaAssociation::Premultiplied {
        return Err(HighresError::Unsupported(
            "premultiplied alpha requires explicit unpremultiplication before AVIF encoding".into(),
        ));
    }
    if matches!(frame.pixels(), PixelBuffer::F32(_)) {
        return Err(HighresError::Unsupported(
            "AVIF high-bit-depth encoding accepts U8 and U16 input only".into(),
        ));
    }
    Ok(())
}

fn color_layout(
    model: ChannelModel,
    subsampling: NativeSubsampling,
) -> Result<(Vec<ChannelRole>, (u8, u8)), HighresError> {
    match model {
        ChannelModel::Gray => {
            if !matches!(subsampling, NativeSubsampling::Cs400) {
                return Err(HighresError::Unsupported(
                    "Gray AVIF encoding requires 4:0:0 native subsampling".into(),
                ));
            }
            Ok((vec![ChannelRole::Gray], (0, 0)))
        }
        ChannelModel::RGB => {
            if !matches!(subsampling, NativeSubsampling::Cs444) {
                return Err(HighresError::Unsupported(
                    "RGB AVIF encoding requires full-resolution identity planes".into(),
                ));
            }
            Ok((
                vec![ChannelRole::Green, ChannelRole::Blue, ChannelRole::Red],
                (0, 0),
            ))
        }
        ChannelModel::YCbCr => {
            let factors = match subsampling {
                NativeSubsampling::Cs420 => (1, 1),
                NativeSubsampling::Cs422 => (1, 0),
                NativeSubsampling::Cs444 => (0, 0),
                NativeSubsampling::Cs400 => {
                    return Err(HighresError::Unsupported(
                        "YCbCr AVIF encoding requires 4:2:0, 4:2:2, or 4:4:4".into(),
                    ));
                }
            };
            Ok((
                vec![ChannelRole::Y, ChannelRole::Cb, ChannelRole::Cr],
                factors,
            ))
        }
    }
}

fn encode_role(
    frame: &ImageFrame,
    role: ChannelRole,
    target_bits: u8,
    _quantization: Quantization,
) -> Result<EncodedSamples, EncodeError> {
    let (plane_index, descriptor, offset) = find_role(frame, role)?;
    let layout = descriptor.layout();
    let width = usize::try_from(layout.width())
        .map_err(|_| HighresError::InvalidDimensions("plane width does not fit usize".into()))?;
    let height = usize::try_from(layout.height())
        .map_err(|_| HighresError::InvalidDimensions("plane height does not fit usize".into()))?;
    let length = width
        .checked_mul(height)
        .ok_or_else(|| HighresError::InvalidDimensions("plane sample count overflows".into()))?;
    let shift = descriptor.meaningful_bits() - target_bits;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(length)
        .map_err(|_| HighresError::InvalidDimensions("AVIF sample allocation failed".into()))?;
    match frame.pixels() {
        PixelBuffer::U8(planes) => extract_samples(
            planes.as_slice()[plane_index].samples(),
            layout,
            offset,
            shift,
            &mut samples,
        ),
        PixelBuffer::U16(planes) => extract_samples(
            planes.as_slice()[plane_index].samples(),
            layout,
            offset,
            shift,
            &mut samples,
        ),
        PixelBuffer::F32(_) => unreachable!("F32 rejected before extraction"),
    }
    Ok(EncodedSamples {
        samples,
        width,
        height,
    })
}

struct EncodedSamples {
    samples: Vec<u16>,
    width: usize,
    height: usize,
}

fn extract_samples<T: Copy + Into<u16>>(
    source: &[T],
    layout: &super::PlaneLayout,
    offset: usize,
    shift: u8,
    output: &mut Vec<u16>,
) {
    for y in 0..layout.height() as usize {
        let row = y * layout.row_stride() + offset;
        for x in 0..layout.width() as usize {
            output.push(source[row + x * layout.pixel_stride()].into() >> shift);
        }
    }
}

fn find_role(
    frame: &ImageFrame,
    role: ChannelRole,
) -> Result<(usize, &super::PlaneDescriptor, usize), HighresError> {
    let mut found = None;
    for (plane_index, descriptor) in frame.descriptor().planes().iter().enumerate() {
        if let Some(role_index) = descriptor.roles().iter().position(|value| *value == role) {
            if found.is_some() {
                return Err(HighresError::InvalidLayout(
                    "AVIF source role appears more than once".into(),
                ));
            }
            found = Some((
                plane_index,
                descriptor,
                descriptor.layout().channel_offsets()[role_index],
            ));
        }
    }
    found.ok_or_else(|| HighresError::InvalidLayout("AVIF source role is missing".into()))
}

fn quantize(
    sample: u16,
    source_bits: u8,
    target_bits: u8,
    quantization: Quantization,
) -> Result<u16, HighresError> {
    if source_bits > 16 || target_bits > 16 {
        return Err(HighresError::InvalidSamples(
            "AVIF integer precision is outside U16 storage".into(),
        ));
    }
    if source_bits < target_bits {
        return Err(HighresError::Unsupported(
            "increasing integer precision requires an explicit conversion".into(),
        ));
    }
    if source_bits > target_bits {
        if quantization == Quantization::None {
            return Err(HighresError::Unsupported(
                "reducing integer precision requires explicit RightShift quantization".into(),
            ));
        }
        return Ok(sample >> (source_bits - target_bits));
    }
    Ok(sample)
}

fn plane_buffer(
    samples: Vec<u16>,
    width: usize,
    height: usize,
    plane: u8,
    subsampling_x: u8,
    subsampling_y: u8,
) -> Result<PlaneBuffer, HighresError> {
    let sample_count = samples.len();
    let expected = width.checked_mul(height).ok_or_else(|| {
        HighresError::InvalidDimensions("AVIF plane sample count overflows".into())
    })?;
    if sample_count != expected {
        return Err(HighresError::InvalidLayout(
            "AVIF plane sample count does not match its dimensions".into(),
        ));
    }
    Ok(PlaneBuffer {
        layout: PlaneLayout {
            plane,
            width,
            height,
            subsampling_x,
            subsampling_y,
            sample_count,
        },
        samples,
    })
}
