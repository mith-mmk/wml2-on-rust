//! Thin, explicit adapter around the independent Gray/RGB ICC CMS.

use icc_profile::{Profile, Transform, TransformError};

pub use icc_profile::{RenderingIntent, TransformOptions};

use super::{
    ChannelModel, ChannelRole, ColorInformationSet, FrameMetadata, HighresError, ImageFrame,
    PixelBuffer, Plane, PlaneLayout,
};

/// Errors raised by an explicit WML2 ICC conversion.
#[derive(Debug)]
pub enum Error {
    Cms(TransformError),
    Frame(HighresError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cms(error) => error.fmt(formatter),
            Self::Frame(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {}

impl From<TransformError> for Error {
    fn from(error: TransformError) -> Self {
        Self::Cms(error)
    }
}

impl From<HighresError> for Error {
    fn from(error: HighresError) -> Self {
        Self::Frame(error)
    }
}

/// Convert one validated Gray or RGB frame with the default relative intent.
pub fn transform_frame(
    frame: &ImageFrame,
    source_icc: &[u8],
    destination_icc: &[u8],
) -> Result<ImageFrame, Error> {
    transform_frame_with_options(
        frame,
        source_icc,
        destination_icc,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..TransformOptions::default()
        },
    )
}

/// Convert one validated Gray or RGB frame using explicitly selected ICC
/// rendering options. The source and destination profiles are always supplied
/// by the caller; frame metadata is not consulted to infer a route.
pub fn transform_frame_with_options(
    frame: &ImageFrame,
    source_icc: &[u8],
    destination_icc: &[u8],
    options: TransformOptions,
) -> Result<ImageFrame, Error> {
    FrameTransform::new(source_icc, destination_icc, options)?.transform(frame)
}

/// Reusable compiled ICC conversion. Scratch storage is bounded to 256 RGB
/// pixels, independently of image dimensions. Integer samples use their own
/// plane precision; alpha and padding never enter the CMS.
pub struct FrameTransform {
    transform: Transform,
    destination_icc: Vec<u8>,
}

impl FrameTransform {
    pub fn new(
        source_icc: &[u8],
        destination_icc: &[u8],
        options: TransformOptions,
    ) -> Result<Self, Error> {
        let source = Profile::from_bytes(source_icc)?;
        let destination = Profile::from_bytes(destination_icc)?;
        Ok(Self {
            transform: Transform::new(&source, &destination, options)?,
            destination_icc: destination_icc.to_vec(),
        })
    }

    pub fn transform(&self, frame: &ImageFrame) -> Result<ImageFrame, Error> {
        frame.validate()?;
        if frame.descriptor().alpha() == super::AlphaAssociation::Premultiplied {
            return Err(HighresError::Unsupported(
                "ICC requires explicit unpremultiplication of alpha".into(),
            )
            .into());
        }
        let colors = frame.descriptor().color_information();
        if colors.nclx().is_some_and(|color| !color.full_range())
            || colors.av1().is_some_and(|color| !color.full_range())
        {
            return Err(HighresError::Unsupported(
                "ICC requires explicit conversion of limited-range samples".into(),
            )
            .into());
        }
        let channels = match frame.descriptor().model() {
            ChannelModel::Gray => 1,
            ChannelModel::RGB => 3,
            ChannelModel::YCbCr => {
                return Err(HighresError::Unsupported(
                    "ICC requires explicit YCbCr to RGB conversion".into(),
                )
                .into());
            }
        };
        let locations = sample_locations(frame, channels)?;
        if self.transform.input_channels() != channels
            || self.transform.output_channels() != channels
        {
            return Err(TransformError::UnsupportedProfileFeature(
                "WML2 ICC adapter requires matching Gray/RGB channel counts",
            )
            .into());
        }
        let mut pixels = frame.pixels().clone();
        match &mut pixels {
            PixelBuffer::U8(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
            PixelBuffer::U16(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
            PixelBuffer::F32(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
        }
        let colors = ColorInformationSet::new().with_icc_profile(self.destination_icc.clone())?;
        let descriptor = frame.descriptor().clone().with_color_information(colors);
        let metadata: FrameMetadata = frame.metadata().clone();
        let mut output = ImageFrame::new(descriptor, pixels)?.with_metadata(metadata);
        if let Some(timing) = frame.timing() {
            output = output.with_timing(timing);
        }
        Ok(output)
    }

    fn apply<T: ColorSample>(
        &self,
        planes: &mut [Plane<T>],
        locations: &[SampleLocation],
        frame: &ImageFrame,
    ) -> Result<(), Error> {
        let mut input = [0.0f32; 256 * 3];
        let mut output = [0.0f32; 256 * 3];
        let width = frame.descriptor().width() as usize;
        for y in 0..frame.descriptor().height() as usize {
            for start in (0..width).step_by(256) {
                let count = (width - start).min(256);
                for (x, pixel) in input[..count * locations.len()]
                    .chunks_exact_mut(locations.len())
                    .enumerate()
                {
                    for (channel, location) in locations.iter().enumerate() {
                        let plane = &planes[location.plane];
                        let index = sample_index(plane.layout(), start + x, y, location.offset)?;
                        pixel[channel] = plane.samples()[index].normalized(location.maximum);
                    }
                }
                self.transform.transform_f32(
                    &input[..count * locations.len()],
                    &mut output[..count * locations.len()],
                )?;
                for (x, pixel) in output[..count * locations.len()]
                    .chunks_exact(locations.len())
                    .enumerate()
                {
                    for (channel, location) in locations.iter().enumerate() {
                        let plane = &mut planes[location.plane];
                        let index = sample_index(plane.layout(), start + x, y, location.offset)?;
                        plane.samples_mut()[index] =
                            T::from_normalized(pixel[channel], location.maximum);
                    }
                }
            }
        }
        Ok(())
    }
}

trait ColorSample: Copy {
    fn normalized(self, maximum: f32) -> f32;
    fn from_normalized(value: f32, maximum: f32) -> Self;
}
macro_rules! integer_sample {
    ($ty:ty) => {
        impl ColorSample for $ty {
            fn normalized(self, maximum: f32) -> f32 {
                self as f32 / maximum
            }
            fn from_normalized(value: f32, maximum: f32) -> Self {
                (value.clamp(0.0, 1.0) * maximum).round() as Self
            }
        }
    };
}
integer_sample!(u8);
integer_sample!(u16);
impl ColorSample for f32 {
    fn normalized(self, _: f32) -> f32 {
        self
    }
    fn from_normalized(value: f32, _: f32) -> Self {
        value
    }
}

#[derive(Clone, Copy)]
struct SampleLocation {
    plane: usize,
    offset: usize,
    maximum: f32,
}

fn sample_locations(
    frame: &ImageFrame,
    channels: usize,
) -> Result<Vec<SampleLocation>, HighresError> {
    let roles: Vec<ChannelRole> = match frame.descriptor().model() {
        ChannelModel::Gray => vec![ChannelRole::Gray],
        ChannelModel::RGB => vec![ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
        ChannelModel::YCbCr => unreachable!("YCbCr is rejected before locating ICC samples"),
    };
    debug_assert_eq!(roles.len(), channels);
    let mut locations = Vec::with_capacity(channels);
    for role in roles {
        let mut found = None;
        for (plane_index, descriptor) in frame.descriptor().planes().iter().enumerate() {
            if let Some(role_index) = descriptor.roles().iter().position(|value| *value == role) {
                if found.is_some() {
                    return Err(HighresError::InvalidLayout(
                        "ICC source role appears more than once".into(),
                    ));
                }
                if descriptor.layout().subsampling() != super::Subsampling::FULL
                    || descriptor.layout().width() != frame.descriptor().width()
                    || descriptor.layout().height() != frame.descriptor().height()
                {
                    return Err(HighresError::Unsupported(
                        "ICC adapter requires full-resolution Gray/RGB planes".into(),
                    ));
                }
                found = Some(SampleLocation {
                    plane: plane_index,
                    maximum: ((1u64 << descriptor.meaningful_bits()) - 1) as f32,
                    offset: descriptor.layout().channel_offsets()[role_index],
                });
            }
        }
        locations.push(found.ok_or_else(|| {
            HighresError::InvalidLayout("ICC source color role is missing".into())
        })?);
    }
    Ok(locations)
}

fn sample_index(
    layout: &PlaneLayout,
    x: usize,
    y: usize,
    offset: usize,
) -> Result<usize, HighresError> {
    y.checked_mul(layout.row_stride())
        .and_then(|value| value.checked_add(x.checked_mul(layout.pixel_stride())?))
        .and_then(|value| value.checked_add(offset))
        .ok_or_else(|| HighresError::InvalidLayout("ICC sample index overflows".into()))
}
