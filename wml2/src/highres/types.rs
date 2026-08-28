//! Checked typed planes and frame descriptors.

use std::{fmt, mem::size_of};

use super::domain::{RgbPrimaries, SampleDomain};
use super::metadata::{ColorInformationSet, FrameMetadata};
use super::{ProcessingError, ResourceLimits};

const MAX_CHANNELS: usize = 4;

/// Errors raised before any high-resolution processing takes place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighresError {
    InvalidDimensions(String),
    InvalidLayout(String),
    InvalidSamples(String),
    InvalidMetadata(String),
    Unsupported(String),
}

impl fmt::Display for HighresError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions(s) => write!(f, "invalid dimensions: {s}"),
            Self::InvalidLayout(s) => write!(f, "invalid layout: {s}"),
            Self::InvalidSamples(s) => write!(f, "invalid samples: {s}"),
            Self::InvalidMetadata(s) => write!(f, "invalid metadata: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported highres operation: {s}"),
        }
    }
}

impl std::error::Error for HighresError {}

pub type Result<T, E = HighresError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    U8,
    U16,
    F32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelModel {
    Gray,
    RGB,
    YCbCr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelRole {
    Gray,
    Red,
    Green,
    Blue,
    Y,
    Cb,
    Cr,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaAssociation {
    None,
    Straight,
    Premultiplied,
}

/// Independent horizontal and vertical subsampling factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subsampling {
    x: u8,
    y: u8,
}

impl Subsampling {
    pub const FULL: Self = Self { x: 1, y: 1 };
    pub fn new(x: u8, y: u8) -> Result<Self> {
        if x == 0 || y == 0 {
            return Err(HighresError::InvalidLayout(
                "subsampling factors must be non-zero".into(),
            ));
        }
        Ok(Self { x, y })
    }
    pub const fn x(self) -> u8 {
        self.x
    }
    pub const fn y(self) -> u8 {
        self.y
    }
    pub fn dimensions(self, width: u32, height: u32) -> Result<(u32, u32)> {
        let x = u32::from(self.x);
        let y = u32::from(self.y);
        let plane_width = width
            .checked_add(x - 1)
            .ok_or_else(|| HighresError::InvalidDimensions("subsampled width overflows".into()))?
            / x;
        let plane_height = height
            .checked_add(y - 1)
            .ok_or_else(|| HighresError::InvalidDimensions("subsampled height overflows".into()))?
            / y;
        Ok((plane_width, plane_height))
    }
}

/// A sample-unit layout. Strides and offsets are counts of `T`, never bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneLayout {
    width: u32,
    height: u32,
    row_stride: usize,
    pixel_stride: usize,
    channel_offsets: Vec<usize>,
    subsampling: Subsampling,
}

impl PlaneLayout {
    pub fn new(
        width: u32,
        height: u32,
        row_stride: usize,
        pixel_stride: usize,
        channel_offsets: Vec<usize>,
        subsampling: Subsampling,
    ) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(HighresError::InvalidDimensions(
                "plane dimensions must be non-zero".into(),
            ));
        }
        if row_stride == 0 || pixel_stride == 0 {
            return Err(HighresError::InvalidLayout(
                "strides must be positive".into(),
            ));
        }
        if channel_offsets.is_empty() {
            return Err(HighresError::InvalidLayout(
                "a plane needs at least one channel offset".into(),
            ));
        }
        for (index, offset) in channel_offsets.iter().enumerate() {
            if *offset >= pixel_stride {
                return Err(HighresError::InvalidLayout(format!(
                    "channel offset {offset} exceeds pixel stride"
                )));
            }
            if channel_offsets[..index].contains(offset) {
                return Err(HighresError::InvalidLayout(
                    "channel offsets overlap".into(),
                ));
            }
        }
        let row = usize::try_from(height - 1)
            .ok()
            .and_then(|v| v.checked_mul(row_stride));
        let pixel = usize::try_from(width - 1)
            .ok()
            .and_then(|v| v.checked_mul(pixel_stride));
        let last = row
            .and_then(|v| pixel.and_then(|p| v.checked_add(p)))
            .and_then(|v| v.checked_add(*channel_offsets.iter().max().unwrap()))
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| HighresError::InvalidLayout("last addressed sample overflows".into()))?;
        let minimum_row_stride = usize::try_from(width - 1)
            .ok()
            .and_then(|v| v.checked_mul(pixel_stride))
            .and_then(|v| v.checked_add(*channel_offsets.iter().max().unwrap()))
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| HighresError::InvalidLayout("minimum row stride overflows".into()))?;
        if row_stride < minimum_row_stride {
            return Err(HighresError::InvalidLayout(
                "row stride overlaps adjacent rows".into(),
            ));
        }
        if last == 0 {
            return Err(HighresError::InvalidLayout("empty addressed range".into()));
        }
        Ok(Self {
            width,
            height,
            row_stride,
            pixel_stride,
            channel_offsets,
            subsampling,
        })
    }
    pub fn planar(width: u32, height: u32, subsampling: Subsampling) -> Result<Self> {
        let row_stride = usize::try_from(width)
            .map_err(|_| HighresError::InvalidDimensions("width does not fit usize".into()))?;
        let mut channel_offsets = Vec::new();
        channel_offsets
            .try_reserve_exact(1)
            .map_err(|_| HighresError::InvalidLayout("channel offset allocation failed".into()))?;
        channel_offsets.push(0);
        Self::new(width, height, row_stride, 1, channel_offsets, subsampling)
    }
    pub fn interleaved(
        width: u32,
        height: u32,
        channels: usize,
        subsampling: Subsampling,
    ) -> Result<Self> {
        if channels == 0 {
            return Err(HighresError::InvalidLayout(
                "interleaved plane has no channels".into(),
            ));
        }
        if channels > MAX_CHANNELS {
            return Err(HighresError::Unsupported(
                "interleaved planes support at most four channels".into(),
            ));
        }
        let width_usize = usize::try_from(width)
            .map_err(|_| HighresError::InvalidDimensions("width does not fit usize".into()))?;
        let row_stride = width_usize
            .checked_mul(channels)
            .ok_or_else(|| HighresError::InvalidLayout("row stride overflows".into()))?;
        Self::new(
            width,
            height,
            row_stride,
            channels,
            (0..channels).collect(),
            subsampling,
        )
    }
    pub const fn width(&self) -> u32 {
        self.width
    }
    pub const fn height(&self) -> u32 {
        self.height
    }
    pub const fn row_stride(&self) -> usize {
        self.row_stride
    }
    pub const fn pixel_stride(&self) -> usize {
        self.pixel_stride
    }
    pub fn channel_offsets(&self) -> &[usize] {
        &self.channel_offsets
    }
    pub const fn subsampling(&self) -> Subsampling {
        self.subsampling
    }
    pub fn addressed_samples(&self) -> usize {
        (self.height as usize - 1) * self.row_stride
            + (self.width as usize - 1) * self.pixel_stride
            + self.channel_offsets.iter().copied().max().unwrap()
            + 1
    }

    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        self.channel_offsets
            .capacity()
            .checked_mul(size_of::<usize>())
            .ok_or_else(|| HighresError::InvalidLayout("layout allocation size overflows".into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneDescriptor {
    layout: PlaneLayout,
    roles: Vec<ChannelRole>,
    meaningful_bits: u8,
}

impl PlaneDescriptor {
    pub fn new(layout: PlaneLayout, roles: Vec<ChannelRole>, meaningful_bits: u8) -> Result<Self> {
        if roles.is_empty() || roles.len() != layout.channel_offsets().len() {
            return Err(HighresError::InvalidLayout(
                "roles must match channel offsets".into(),
            ));
        }
        if roles.len() > MAX_CHANNELS {
            return Err(HighresError::Unsupported(
                "planes support at most four channels".into(),
            ));
        }
        if !(1..=32).contains(&meaningful_bits) {
            return Err(HighresError::InvalidSamples(
                "meaningful precision must be 1..=32 bits".into(),
            ));
        }
        for (index, role) in roles.iter().enumerate() {
            if roles[..index].contains(role) {
                return Err(HighresError::InvalidLayout("duplicate channel role".into()));
            }
        }
        Ok(Self {
            layout,
            roles,
            meaningful_bits,
        })
    }
    pub fn planar(layout: PlaneLayout, role: ChannelRole, meaningful_bits: u8) -> Result<Self> {
        Self::new(layout, vec![role], meaningful_bits)
    }
    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        let roles = self
            .roles
            .capacity()
            .checked_mul(size_of::<ChannelRole>())
            .ok_or_else(|| HighresError::InvalidLayout("role allocation size overflows".into()))?;
        self.layout
            .owned_bytes()?
            .checked_add(roles)
            .ok_or_else(|| HighresError::InvalidLayout("descriptor size overflows".into()))
    }
    pub fn layout(&self) -> &PlaneLayout {
        &self.layout
    }
    pub fn roles(&self) -> &[ChannelRole] {
        &self.roles
    }
    pub const fn meaningful_bits(&self) -> u8 {
        self.meaningful_bits
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plane<T> {
    layout: PlaneLayout,
    samples: Vec<T>,
}

impl<T> Plane<T> {
    pub fn new(layout: PlaneLayout, samples: Vec<T>) -> Result<Self> {
        if samples.len() < layout.addressed_samples() {
            return Err(HighresError::InvalidLayout(format!(
                "plane contains {} samples but needs at least {}",
                samples.len(),
                layout.addressed_samples()
            )));
        }
        Ok(Self { layout, samples })
    }
    pub fn from_slice(layout: PlaneLayout, samples: &[T]) -> Result<Self>
    where
        T: Clone,
    {
        Self::new(layout, samples.to_vec())
    }
    pub fn layout(&self) -> &PlaneLayout {
        &self.layout
    }
    pub fn samples(&self) -> &[T] {
        &self.samples
    }
    pub fn samples_mut(&mut self) -> &mut [T] {
        &mut self.samples
    }
    pub fn into_samples(self) -> Vec<T> {
        self.samples
    }
    pub fn sample_len(&self) -> usize {
        self.samples.len()
    }
    pub fn sample_capacity(&self) -> usize {
        self.samples.capacity()
    }
    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        let samples = self
            .samples
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or_else(|| {
                HighresError::InvalidLayout("sample allocation size overflows".into())
            })?;
        self.layout
            .owned_bytes()?
            .checked_add(samples)
            .ok_or_else(|| HighresError::InvalidLayout("plane size overflows".into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Planes<T> {
    planes: Vec<Plane<T>>,
}

impl<T> Planes<T> {
    pub fn new(planes: Vec<Plane<T>>) -> Result<Self> {
        if planes.is_empty() {
            return Err(HighresError::InvalidLayout(
                "pixel buffer has no planes".into(),
            ));
        }
        Ok(Self { planes })
    }
    pub fn as_slice(&self) -> &[Plane<T>] {
        &self.planes
    }
    pub fn as_mut_slice(&mut self) -> &mut [Plane<T>] {
        &mut self.planes
    }
    pub fn into_vec(self) -> Vec<Plane<T>> {
        self.planes
    }
    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        let outer = self
            .planes
            .capacity()
            .checked_mul(size_of::<Plane<T>>())
            .ok_or_else(|| HighresError::InvalidLayout("plane vector size overflows".into()))?;
        self.planes.iter().try_fold(outer, |total, plane| {
            total
                .checked_add(plane.owned_bytes()?)
                .ok_or_else(|| HighresError::InvalidLayout("plane size overflows".into()))
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PixelBuffer {
    U8(Planes<u8>),
    U16(Planes<u16>),
    F32(Planes<f32>),
}

impl PixelBuffer {
    pub fn u8(planes: Vec<Plane<u8>>) -> Result<Self> {
        Ok(Self::U8(Planes::new(planes)?))
    }
    pub fn u16(planes: Vec<Plane<u16>>) -> Result<Self> {
        Ok(Self::U16(Planes::new(planes)?))
    }
    pub fn f32(planes: Vec<Plane<f32>>) -> Result<Self> {
        Ok(Self::F32(Planes::new(planes)?))
    }
    pub const fn format(&self) -> PixelFormat {
        match self {
            Self::U8(_) => PixelFormat::U8,
            Self::U16(_) => PixelFormat::U16,
            Self::F32(_) => PixelFormat::F32,
        }
    }
    pub fn plane_count(&self) -> usize {
        match self {
            Self::U8(p) => p.as_slice().len(),
            Self::U16(p) => p.as_slice().len(),
            Self::F32(p) => p.as_slice().len(),
        }
    }
    pub fn u8_planes(&self) -> Option<&[Plane<u8>]> {
        if let Self::U8(p) = self {
            Some(p.as_slice())
        } else {
            None
        }
    }
    pub fn u16_planes(&self) -> Option<&[Plane<u16>]> {
        if let Self::U16(p) = self {
            Some(p.as_slice())
        } else {
            None
        }
    }
    pub fn f32_planes(&self) -> Option<&[Plane<f32>]> {
        if let Self::F32(p) = self {
            Some(p.as_slice())
        } else {
            None
        }
    }
    pub fn u8_planes_mut(&mut self) -> Option<&mut [Plane<u8>]> {
        if let Self::U8(p) = self {
            Some(p.as_mut_slice())
        } else {
            None
        }
    }
    pub fn u16_planes_mut(&mut self) -> Option<&mut [Plane<u16>]> {
        if let Self::U16(p) = self {
            Some(p.as_mut_slice())
        } else {
            None
        }
    }
    pub fn f32_planes_mut(&mut self) -> Option<&mut [Plane<f32>]> {
        if let Self::F32(p) = self {
            Some(p.as_mut_slice())
        } else {
            None
        }
    }
    pub(crate) fn allocated_sample_count(&self) -> Result<usize> {
        match self {
            Self::U8(p) => p.as_slice().iter().try_fold(0usize, |total, plane| {
                total.checked_add(plane.sample_capacity())
            }),
            Self::U16(p) => p.as_slice().iter().try_fold(0usize, |total, plane| {
                total.checked_add(plane.sample_capacity())
            }),
            Self::F32(p) => p.as_slice().iter().try_fold(0usize, |total, plane| {
                total.checked_add(plane.sample_capacity())
            }),
        }
        .ok_or_else(|| HighresError::InvalidLayout("allocated sample count overflows".into()))
    }

    pub(crate) fn max_allocated_plane_samples(&self) -> Result<usize> {
        let max = match self {
            Self::U8(p) => p.as_slice().iter().map(Plane::sample_capacity).max(),
            Self::U16(p) => p.as_slice().iter().map(Plane::sample_capacity).max(),
            Self::F32(p) => p.as_slice().iter().map(Plane::sample_capacity).max(),
        };
        max.ok_or_else(|| HighresError::InvalidLayout("pixel buffer has no planes".into()))
    }

    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        match self {
            Self::U8(planes) => planes.owned_bytes(),
            Self::U16(planes) => planes.owned_bytes(),
            Self::F32(planes) => planes.owned_bytes(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameTiming {
    timescale: u64,
    pts: u64,
    duration: u64,
}

impl FrameTiming {
    pub fn new(timescale: u64, pts: u64, duration: u64) -> Result<Self> {
        if timescale == 0 {
            return Err(HighresError::InvalidMetadata(
                "timing timescale must be non-zero".into(),
            ));
        }
        Ok(Self {
            timescale,
            pts,
            duration,
        })
    }
    pub const fn timescale(self) -> u64 {
        self.timescale
    }
    pub const fn pts(self) -> u64 {
        self.pts
    }
    pub const fn duration(self) -> u64 {
        self.duration
    }
}

/// Native image descriptor. Fields are private so every externally-created
/// value has passed the role/layout invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageDescriptor {
    width: u32,
    height: u32,
    model: ChannelModel,
    planes: Vec<PlaneDescriptor>,
    alpha: AlphaAssociation,
    color_information: ColorInformationSet,
    domain: SampleDomain,
    primaries: Option<RgbPrimaries>,
}

impl ImageDescriptor {
    pub fn new(
        width: u32,
        height: u32,
        model: ChannelModel,
        planes: Vec<PlaneDescriptor>,
    ) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(HighresError::InvalidDimensions(
                "image dimensions must be non-zero".into(),
            ));
        }
        if planes.is_empty() {
            return Err(HighresError::InvalidLayout("image has no planes".into()));
        }
        let required: &[ChannelRole] = match model {
            ChannelModel::Gray => &[ChannelRole::Gray],
            ChannelModel::RGB => &[ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue],
            ChannelModel::YCbCr => &[ChannelRole::Y, ChannelRole::Cb, ChannelRole::Cr],
        };
        let mut roles = 0u16;
        for plane in &planes {
            let (expected_width, expected_height) =
                plane.layout.subsampling.dimensions(width, height)?;
            if plane.layout.width != expected_width || plane.layout.height != expected_height {
                return Err(HighresError::InvalidLayout(
                    "plane dimensions do not match image/subsampling".into(),
                ));
            }
            for role in plane.roles() {
                let bit = channel_role_bit(*role);
                if roles & bit != 0 {
                    return Err(HighresError::InvalidLayout(
                        "duplicate channel role across planes".into(),
                    ));
                }
                roles |= bit;
            }
        }
        for role in required {
            if roles & channel_role_bit(*role) == 0 {
                return Err(HighresError::InvalidLayout(format!(
                    "missing required channel role {role:?}"
                )));
            }
        }
        let required_mask = required
            .iter()
            .fold(0u16, |mask, role| mask | channel_role_bit(*role));
        if roles & !(required_mask | channel_role_bit(ChannelRole::Alpha)) != 0 {
            return Err(HighresError::Unsupported(
                "channel role is not supported by this model".into(),
            ));
        }
        if roles & channel_role_bit(ChannelRole::Alpha) != 0 {
            // Alpha is valid for every model, but it is always full resolution.
            let alpha_plane = planes
                .iter()
                .find(|p| p.roles().contains(&ChannelRole::Alpha))
                .unwrap();
            if alpha_plane.layout.subsampling != Subsampling::FULL {
                return Err(HighresError::InvalidLayout(
                    "alpha must be full resolution".into(),
                ));
            }
        }
        Ok(Self {
            width,
            height,
            model,
            planes,
            alpha: AlphaAssociation::None,
            color_information: ColorInformationSet::default(),
            domain: SampleDomain::Unknown,
            primaries: None,
        })
    }
    pub fn gray(width: u32, height: u32, meaningful_bits: u8) -> Result<Self> {
        let layout = PlaneLayout::planar(width, height, Subsampling::FULL)?;
        Self::new(
            width,
            height,
            ChannelModel::Gray,
            vec![PlaneDescriptor::planar(
                layout,
                ChannelRole::Gray,
                meaningful_bits,
            )?],
        )
    }
    pub fn rgb(width: u32, height: u32, meaningful_bits: u8) -> Result<Self> {
        let planes = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
            .into_iter()
            .map(|role| {
                PlaneDescriptor::planar(
                    PlaneLayout::planar(width, height, Subsampling::FULL)?,
                    role,
                    meaningful_bits,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        Self::new(width, height, ChannelModel::RGB, planes)
    }
    pub fn ycbcr(
        width: u32,
        height: u32,
        meaningful_bits: u8,
        subsampling: [Subsampling; 3],
    ) -> Result<Self> {
        let roles = [ChannelRole::Y, ChannelRole::Cb, ChannelRole::Cr];
        let planes = roles
            .into_iter()
            .zip(subsampling)
            .map(|(role, sub)| {
                let (w, h) = sub.dimensions(width, height)?;
                PlaneDescriptor::planar(PlaneLayout::planar(w, h, sub)?, role, meaningful_bits)
            })
            .collect::<Result<Vec<_>>>()?;
        Self::new(width, height, ChannelModel::YCbCr, planes)
    }
    pub fn with_alpha(mut self, alpha: AlphaAssociation) -> Result<Self> {
        let has_alpha = self
            .planes
            .iter()
            .any(|p| p.roles().contains(&ChannelRole::Alpha));
        if has_alpha != (alpha != AlphaAssociation::None) {
            return Err(HighresError::InvalidLayout(
                "alpha association does not match alpha plane".into(),
            ));
        }
        self.alpha = alpha;
        Ok(self)
    }
    pub fn with_color_information(mut self, color: ColorInformationSet) -> Self {
        self.color_information = color;
        self
    }
    pub fn with_domain(mut self, domain: SampleDomain) -> Self {
        self.domain = domain;
        self
    }
    pub fn with_primaries(mut self, primaries: RgbPrimaries) -> Self {
        self.primaries = Some(primaries);
        self
    }
    pub const fn width(&self) -> u32 {
        self.width
    }
    pub const fn height(&self) -> u32 {
        self.height
    }
    pub const fn model(&self) -> ChannelModel {
        self.model
    }
    pub fn planes(&self) -> &[PlaneDescriptor] {
        &self.planes
    }
    pub const fn alpha(&self) -> AlphaAssociation {
        self.alpha
    }
    pub fn color_information(&self) -> &ColorInformationSet {
        &self.color_information
    }
    pub const fn domain(&self) -> SampleDomain {
        self.domain
    }
    pub const fn primaries(&self) -> Option<RgbPrimaries> {
        self.primaries
    }
    pub(crate) fn owned_bytes(&self) -> Result<usize> {
        let outer = self
            .planes
            .capacity()
            .checked_mul(size_of::<PlaneDescriptor>())
            .ok_or_else(|| {
                HighresError::InvalidLayout("descriptor vector size overflows".into())
            })?;
        let total = self.planes.iter().try_fold(outer, |total, plane| {
            total
                .checked_add(plane.owned_bytes()?)
                .ok_or_else(|| HighresError::InvalidLayout("descriptor size overflows".into()))
        })?;
        total
            .checked_add(self.color_information.owned_bytes()?)
            .ok_or_else(|| HighresError::InvalidMetadata("descriptor size overflows".into()))
    }
}

const fn channel_role_bit(role: ChannelRole) -> u16 {
    match role {
        ChannelRole::Gray => 1 << 0,
        ChannelRole::Red => 1 << 1,
        ChannelRole::Green => 1 << 2,
        ChannelRole::Blue => 1 << 3,
        ChannelRole::Y => 1 << 4,
        ChannelRole::Cb => 1 << 5,
        ChannelRole::Cr => 1 << 6,
        ChannelRole::Alpha => 1 << 7,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageFrame {
    descriptor: ImageDescriptor,
    pixels: PixelBuffer,
    metadata: FrameMetadata,
    timing: Option<FrameTiming>,
}

impl ImageFrame {
    pub fn new(descriptor: ImageDescriptor, pixels: PixelBuffer) -> Result<Self> {
        let source_color = descriptor.color_information().try_clone_owned()?;
        let frame = Self {
            metadata: FrameMetadata::new(source_color),
            descriptor,
            pixels,
            timing: None,
        };
        frame.validate()?;
        Ok(frame)
    }
    #[cfg(feature = "avif")]
    pub(crate) fn from_parts(
        descriptor: ImageDescriptor,
        pixels: PixelBuffer,
        metadata: FrameMetadata,
        timing: Option<FrameTiming>,
    ) -> Result<Self> {
        let frame = Self {
            descriptor,
            pixels,
            metadata,
            timing,
        };
        frame.validate()?;
        Ok(frame)
    }
    pub fn with_metadata(mut self, metadata: FrameMetadata) -> Self {
        self.metadata = metadata;
        self
    }
    pub fn with_timing(mut self, timing: FrameTiming) -> Self {
        self.timing = Some(timing);
        self
    }
    pub fn descriptor(&self) -> &ImageDescriptor {
        &self.descriptor
    }
    pub fn pixels(&self) -> &PixelBuffer {
        &self.pixels
    }
    pub fn pixels_mut(&mut self) -> &mut PixelBuffer {
        &mut self.pixels
    }
    pub fn metadata(&self) -> &FrameMetadata {
        &self.metadata
    }
    pub fn metadata_mut(&mut self) -> &mut FrameMetadata {
        &mut self.metadata
    }
    pub const fn timing(&self) -> Option<FrameTiming> {
        self.timing
    }
    pub fn validate(&self) -> Result<()> {
        let has_alpha = self
            .descriptor
            .planes
            .iter()
            .any(|plane| plane.roles().contains(&ChannelRole::Alpha));
        if has_alpha != (self.descriptor.alpha != AlphaAssociation::None) {
            return Err(HighresError::InvalidLayout(
                "alpha plane requires an explicit association".into(),
            ));
        }
        self.metadata
            .validate_bounds(self.descriptor.width, self.descriptor.height)?;
        if self.pixels.plane_count() != self.descriptor.planes.len() {
            return Err(HighresError::InvalidLayout(
                "pixel plane count does not match descriptor".into(),
            ));
        }
        match &self.pixels {
            PixelBuffer::U8(p) => {
                for (plane, desc) in p.as_slice().iter().zip(&self.descriptor.planes) {
                    validate_plane_layout(plane.layout(), desc.layout())?;
                    validate_integer(plane, desc.meaningful_bits, 8)?;
                }
            }
            PixelBuffer::U16(p) => {
                for (plane, desc) in p.as_slice().iter().zip(&self.descriptor.planes) {
                    validate_plane_layout(plane.layout(), desc.layout())?;
                    validate_integer(plane, desc.meaningful_bits, 16)?;
                }
            }
            PixelBuffer::F32(p) => {
                for (plane, desc) in p.as_slice().iter().zip(&self.descriptor.planes) {
                    validate_plane_layout(plane.layout(), desc.layout())?;
                    validate_float(plane, desc.roles())?;
                }
            }
        }
        Ok(())
    }
    pub fn validate_with_limits(
        &self,
        limits: &ResourceLimits,
    ) -> std::result::Result<(), ProcessingError> {
        self.validate().map_err(ProcessingError::from)?;
        limits.check_frame(self)
    }
}

fn validate_plane_layout(actual: &PlaneLayout, expected: &PlaneLayout) -> Result<()> {
    if actual != expected {
        return Err(HighresError::InvalidLayout(
            "pixel plane layout does not match descriptor".into(),
        ));
    }
    Ok(())
}

fn validate_integer<T>(plane: &Plane<T>, meaningful_bits: u8, storage_bits: u8) -> Result<()>
where
    T: Copy + Into<u64>,
{
    if meaningful_bits > storage_bits {
        return Err(HighresError::InvalidSamples(
            "meaningful precision exceeds storage width".into(),
        ));
    }
    let max = if meaningful_bits == 64 {
        u64::MAX
    } else {
        (1u64 << meaningful_bits) - 1
    };
    let layout = plane.layout();
    let samples = plane.samples();
    for y in 0..layout.height() as usize {
        for x in 0..layout.width() as usize {
            let base = y * layout.row_stride() + x * layout.pixel_stride();
            for offset in layout.channel_offsets() {
                if samples[base + offset].into() > max {
                    return Err(HighresError::InvalidSamples(
                        "integer sample exceeds meaningful bit depth".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_float(plane: &Plane<f32>, roles: &[ChannelRole]) -> Result<()> {
    let layout = plane.layout();
    let samples = plane.samples();
    let alpha_offset = roles
        .iter()
        .position(|role| *role == ChannelRole::Alpha)
        .map(|index| layout.channel_offsets()[index]);
    for y in 0..layout.height() as usize {
        for x in 0..layout.width() as usize {
            let base = y * layout.row_stride() + x * layout.pixel_stride();
            for offset in layout.channel_offsets() {
                let sample = samples[base + offset];
                if !sample.is_finite() {
                    return Err(HighresError::InvalidSamples(
                        "F32 samples must be finite".into(),
                    ));
                }
                if Some(*offset) == alpha_offset && !(0.0..=1.0).contains(&sample) {
                    return Err(HighresError::InvalidSamples(
                        "F32 alpha must be in [0, 1]".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}
