//! Thin, explicit adapter around the independent Gray/RGB ICC CMS.

use icc_profile::{Profile, Transform, TransformError};

pub use icc_profile::{RenderingIntent, TransformOptions};

use super::{
    ChannelModel, ChannelRole, ColorInformationSet, FrameMetadata, HighresError, ImageDescriptor,
    ImageFrame, PixelBuffer, Plane, PlaneDescriptor, PlaneLayout, Subsampling,
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
        if !matches!(
            source.color_space(),
            icc_profile::ColorSpace::Gray | icc_profile::ColorSpace::Rgb
        ) || !matches!(
            destination.color_space(),
            icc_profile::ColorSpace::Gray | icc_profile::ColorSpace::Rgb
        ) {
            return Err(TransformError::UnsupportedProfileFeature(
                "WML2 ICC requires Gray/RGB device models,not only matching channel counts",
            )
            .into());
        }
        Ok(Self {
            transform: Transform::new(&source, &destination, options)?,
            destination_icc: destination_icc.to_vec(),
        })
    }

    /// Transform a frame. Gray/RGB model changes produce packed planar colour
    /// planes plus a separate unchanged alpha plane. Same-model layouts stay
    /// intact. Use `transform_with_output_limit` to cap output sample storage.
    pub fn transform(&self, frame: &ImageFrame) -> Result<ImageFrame, Error> {
        self.transform_with_output_limit(frame, usize::MAX)
    }

    /// Bound output sample-buffer bytes, including retained same-model padding.
    /// This covers sample storage; metadata clones are excluded. Profile
    /// compilation keeps the own engine's limits; scratch stays at256pixels.
    /// Admission and fallible new-layout allocations precede pixel conversion.
    pub fn transform_with_output_limit(
        &self,
        frame: &ImageFrame,
        output_bytes: usize,
    ) -> Result<ImageFrame, Error> {
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
        let destination_channels = self.transform.output_channels();
        if self.transform.input_channels() != channels || !matches!(destination_channels, 1 | 3) {
            return Err(TransformError::UnsupportedProfileFeature(
                "WML2 ICC source/output must match Gray/RGB device channels",
            )
            .into());
        }
        let (descriptor, pixels) = if destination_channels == channels {
            admit_output(frame_buffer_bytes(frame.pixels())?, output_bytes)?;
            let mut pixels = frame.pixels().clone();
            match &mut pixels {
                PixelBuffer::U8(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
                PixelBuffer::U16(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
                PixelBuffer::F32(planes) => self.apply(planes.as_mut_slice(), &locations, frame)?,
            }
            (frame.descriptor().clone(), pixels)
        } else {
            self.change_layout(frame, &locations, destination_channels, output_bytes)?
        };
        let colors = ColorInformationSet::new().with_icc_profile(self.destination_icc.clone())?;
        let descriptor = descriptor.with_color_information(colors);
        let mut metadata: FrameMetadata = frame.metadata().clone();
        // Native readers can expose the active profile in the generic tag as
        // well as in the typed descriptor. After this explicit CMS operation,
        // both describe destination samples; source_color remains provenance.
        if let Some(profile) = metadata.tags_mut().get_mut("ICC Profile") {
            *profile = crate::metadata::DataMap::ICCProfile(self.destination_icc.clone());
        }
        let mut output = ImageFrame::new(descriptor, pixels)?.with_metadata(metadata);
        if let Some(timing) = frame.timing() {
            output = output.with_timing(timing);
        }
        Ok(output)
    }

    fn change_layout(
        &self,
        frame: &ImageFrame,
        locations: &[SampleLocation],
        channels: usize,
        limit: usize,
    ) -> Result<(ImageDescriptor, PixelBuffer), Error> {
        let source = frame.descriptor();
        let bits = locations
            .iter()
            .map(|l| l.bits)
            .max()
            .expect("validated Gray/RGB roles");
        let alpha = if source.alpha() != super::AlphaAssociation::None {
            Some(sample_location(frame, ChannelRole::Alpha)?)
        } else {
            None
        };
        let descriptor = if channels == 1 {
            ImageDescriptor::gray(source.width(), source.height(), bits)?
        } else {
            ImageDescriptor::rgb(source.width(), source.height(), bits)?
        };
        let descriptor = if let Some(alpha) = alpha {
            let mut planes = descriptor.planes().to_vec();
            planes.push(PlaneDescriptor::planar(
                PlaneLayout::planar(source.width(), source.height(), Subsampling::FULL)?,
                ChannelRole::Alpha,
                alpha.bits,
            )?);
            ImageDescriptor::new(source.width(), source.height(), descriptor.model(), planes)?
                .with_alpha(source.alpha())?
        } else {
            descriptor
        };
        let pixels = match frame.pixels() {
            PixelBuffer::U8(p) => PixelBuffer::u8(self.apply_new_layout(
                p.as_slice(),
                locations,
                alpha,
                &descriptor,
                limit,
            )?)?,
            PixelBuffer::U16(p) => PixelBuffer::u16(self.apply_new_layout(
                p.as_slice(),
                locations,
                alpha,
                &descriptor,
                limit,
            )?)?,
            PixelBuffer::F32(p) => PixelBuffer::f32(self.apply_new_layout(
                p.as_slice(),
                locations,
                alpha,
                &descriptor,
                limit,
            )?)?,
        };
        Ok((descriptor, pixels))
    }

    fn apply_new_layout<T: ColorSample>(
        &self,
        source: &[Plane<T>],
        locations: &[SampleLocation],
        alpha: Option<SampleLocation>,
        descriptor: &ImageDescriptor,
        limit: usize,
    ) -> Result<Vec<Plane<T>>, Error> {
        let width = descriptor.width() as usize;
        let height = descriptor.height() as usize;
        let pixels = width.checked_mul(height).ok_or_else(output_overflow)?;
        let bytes = pixels
            .checked_mul(descriptor.planes().len())
            .and_then(|n| n.checked_mul(std::mem::size_of::<T>()))
            .ok_or_else(output_overflow)?;
        admit_output(bytes, limit)?;
        let channels = self.transform.output_channels();
        let maximum = ((1u64 << descriptor.planes()[0].meaningful_bits()) - 1) as f32;
        let mut planes = Vec::new();
        planes
            .try_reserve_exact(descriptor.planes().len())
            .map_err(|_| TransformError::ResourceLimit("WML2 ICC output plane allocation"))?;
        for desc in descriptor.planes() {
            let mut samples = Vec::new();
            samples
                .try_reserve_exact(pixels)
                .map_err(|_| TransformError::ResourceLimit("WML2 ICC output sample allocation"))?;
            samples.resize(pixels, T::from_normalized(0.0, 1.0));
            planes.push(Plane::new(desc.layout().clone(), samples)?);
        }
        let mut input = [0.0f32; 256 * 3];
        let mut output = [0.0f32; 256 * 3];
        for y in 0..height {
            for start in (0..width).step_by(256) {
                let count = (width - start).min(256);
                for x in 0..count {
                    for (c, location) in locations.iter().enumerate() {
                        let plane = &source[location.plane];
                        let at = sample_index(plane.layout(), start + x, y, location.offset)?;
                        input[x * locations.len() + c] =
                            plane.samples()[at].normalized(location.maximum);
                    }
                }
                self.transform.transform_f32(
                    &input[..count * locations.len()],
                    &mut output[..count * channels],
                )?;
                for x in 0..count {
                    let at = y * width + start + x;
                    for c in 0..channels {
                        planes[c].samples_mut()[at] =
                            T::from_normalized(output[x * channels + c], maximum);
                    }
                    if let Some(alpha) = alpha {
                        let plane = &source[alpha.plane];
                        let from = sample_index(plane.layout(), start + x, y, alpha.offset)?;
                        planes[channels].samples_mut()[at] = plane.samples()[from];
                    }
                }
            }
        }
        Ok(planes)
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
    bits: u8,
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
    roles
        .into_iter()
        .map(|role| sample_location(frame, role))
        .collect()
}
fn sample_location(frame: &ImageFrame, role: ChannelRole) -> Result<SampleLocation, HighresError> {
    let mut found = None;
    for (plane_index, descriptor) in frame.descriptor().planes().iter().enumerate() {
        if let Some(role_index) = descriptor.roles().iter().position(|value| *value == role) {
            if found.is_some() {
                return Err(HighresError::InvalidLayout(
                    "ICC source role appears more than once".into(),
                ));
            }
            if descriptor.layout().subsampling() != Subsampling::FULL
                || descriptor.layout().width() != frame.descriptor().width()
                || descriptor.layout().height() != frame.descriptor().height()
            {
                return Err(HighresError::Unsupported(
                    "ICC adapter requires full-resolution colour/alpha planes".into(),
                ));
            }
            found = Some(SampleLocation {
                plane: plane_index,
                offset: descriptor.layout().channel_offsets()[role_index],
                maximum: ((1u64 << descriptor.meaningful_bits()) - 1) as f32,
                bits: descriptor.meaningful_bits(),
            });
        }
    }
    found.ok_or_else(|| {
        HighresError::InvalidLayout("ICC source colour/alpha role is missing".into())
    })
}
fn output_overflow() -> Error {
    TransformError::ResourceLimit("WML2 ICC output storage overflow").into()
}
fn admit_output(bytes: usize, limit: usize) -> Result<(), Error> {
    if bytes > limit {
        return Err(TransformError::ResourceLimit("WML2 ICC output sample bytes").into());
    }
    Ok(())
}
fn frame_buffer_bytes(pixels: &PixelBuffer) -> Result<usize, Error> {
    fn bytes<T>(planes: &[Plane<T>]) -> Result<usize, Error> {
        planes.iter().try_fold(0usize, |n, p| {
            p.samples()
                .len()
                .checked_mul(std::mem::size_of::<T>())
                .and_then(|b| n.checked_add(b))
                .ok_or_else(output_overflow)
        })
    }
    match pixels {
        PixelBuffer::U8(p) => bytes(p.as_slice()),
        PixelBuffer::U16(p) => bytes(p.as_slice()),
        PixelBuffer::F32(p) => bytes(p.as_slice()),
    }
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
