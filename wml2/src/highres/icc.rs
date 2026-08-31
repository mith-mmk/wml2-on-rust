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
    frame.validate()?;
    let channels = match frame.descriptor().model() {
        ChannelModel::Gray => 1,
        ChannelModel::RGB => 3,
        ChannelModel::YCbCr => {
            return Err(HighresError::Unsupported(
                "ICC conversion requires Gray or RGB samples; YCbCr must be explicitly converted first".into(),
            )
            .into());
        }
    };
    let locations = sample_locations(frame, channels)?;
    let source_profile = Profile::from_bytes(source_icc)?;
    let destination_profile = Profile::from_bytes(destination_icc)?;
    let transform = Transform::new(&source_profile, &destination_profile, options)?;
    if transform.input_channels() != channels || transform.output_channels() != channels {
        return Err(TransformError::UnsupportedProfileFeature(
            "WML2 ICC adapter requires matching Gray/RGB channel counts",
        )
        .into());
    }

    let mut pixels = frame.pixels().clone();
    match &mut pixels {
        PixelBuffer::U8(planes) => {
            let input = read_color_samples(planes.as_slice(), &locations, frame)?;
            let mut output = vec![0u8; input.len()];
            transform.transform_u8(&input, &mut output)?;
            write_color_samples(planes.as_mut_slice(), &locations, frame, &output)?;
        }
        PixelBuffer::U16(planes) => {
            let input = read_color_samples(planes.as_slice(), &locations, frame)?;
            let mut output = vec![0u16; input.len()];
            transform.transform_u16(&input, &mut output)?;
            write_color_samples(planes.as_mut_slice(), &locations, frame, &output)?;
        }
        PixelBuffer::F32(planes) => {
            let input = read_color_samples(planes.as_slice(), &locations, frame)?;
            let mut output = vec![0.0f32; input.len()];
            transform.transform_f32(&input, &mut output)?;
            write_color_samples(planes.as_mut_slice(), &locations, frame, &output)?;
        }
    }

    let destination_color =
        ColorInformationSet::new().with_icc_profile(destination_icc.to_vec())?;
    let descriptor = frame
        .descriptor()
        .clone()
        .with_color_information(destination_color);
    let metadata: FrameMetadata = frame.metadata().clone();
    Ok(ImageFrame::new(descriptor, pixels)?.with_metadata(metadata))
}

#[derive(Clone, Copy)]
struct SampleLocation {
    plane: usize,
    offset: usize,
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

fn read_color_samples<T: Copy>(
    planes: &[Plane<T>],
    locations: &[SampleLocation],
    frame: &ImageFrame,
) -> Result<Vec<T>, HighresError> {
    let pixels = usize::try_from(frame.descriptor().width())
        .ok()
        .and_then(|width| {
            usize::try_from(frame.descriptor().height())
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| HighresError::InvalidDimensions("ICC pixel count overflows".into()))?;
    let length = pixels
        .checked_mul(locations.len())
        .ok_or_else(|| HighresError::InvalidDimensions("ICC sample count overflows".into()))?;
    let mut output = Vec::with_capacity(length);
    for y in 0..frame.descriptor().height() as usize {
        for x in 0..frame.descriptor().width() as usize {
            for location in locations {
                let plane = planes.get(location.plane).ok_or_else(|| {
                    HighresError::InvalidLayout("ICC plane index is out of range".into())
                })?;
                let index = sample_index(plane.layout(), x, y, location.offset)?;
                output.push(*plane.samples().get(index).ok_or_else(|| {
                    HighresError::InvalidLayout("ICC sample index is out of range".into())
                })?);
            }
        }
    }
    Ok(output)
}

fn write_color_samples<T: Copy>(
    planes: &mut [Plane<T>],
    locations: &[SampleLocation],
    frame: &ImageFrame,
    samples: &[T],
) -> Result<(), HighresError> {
    let expected = usize::try_from(frame.descriptor().width())
        .ok()
        .and_then(|width| {
            usize::try_from(frame.descriptor().height())
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(locations.len()))
        .ok_or_else(|| HighresError::InvalidDimensions("ICC sample count overflows".into()))?;
    if samples.len() != expected {
        return Err(HighresError::InvalidLayout(
            "ICC transform output length does not match the frame".into(),
        ));
    }
    let mut sample_index_in_buffer = 0usize;
    for y in 0..frame.descriptor().height() as usize {
        for x in 0..frame.descriptor().width() as usize {
            for location in locations {
                let plane = planes.get_mut(location.plane).ok_or_else(|| {
                    HighresError::InvalidLayout("ICC plane index is out of range".into())
                })?;
                let index = sample_index(plane.layout(), x, y, location.offset)?;
                let destination = plane.samples_mut().get_mut(index).ok_or_else(|| {
                    HighresError::InvalidLayout("ICC sample index is out of range".into())
                })?;
                *destination = samples[sample_index_in_buffer];
                sample_index_in_buffer += 1;
            }
        }
    }
    Ok(())
}
