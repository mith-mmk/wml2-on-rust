//! Metadata retained by the checked high-resolution representation.

use super::HighresError;
use crate::metadata::Metadata;

/// Origin of one piece of colour signalling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProvenance {
    EmbeddedIcc,
    ContainerNclx,
    Av1,
    Explicit,
}

/// CICP/nclx colour information retained without interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NclxColorInformation {
    primaries: u16,
    transfer: u16,
    matrix: u16,
    full_range: bool,
}

impl NclxColorInformation {
    pub const fn new(primaries: u16, transfer: u16, matrix: u16, full_range: bool) -> Self {
        Self {
            primaries,
            transfer,
            matrix,
            full_range,
        }
    }
    pub const fn primaries(self) -> u16 {
        self.primaries
    }
    pub const fn transfer(self) -> u16 {
        self.transfer
    }
    pub const fn matrix(self) -> u16 {
        self.matrix
    }
    pub const fn full_range(self) -> bool {
        self.full_range
    }
}

/// AV1 sequence-header colour signalling retained independently from nclx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Av1ColorInformation {
    primaries: u16,
    transfer: u16,
    matrix: u16,
    full_range: bool,
    description_present: bool,
}

impl Av1ColorInformation {
    pub const fn new(primaries: u16, transfer: u16, matrix: u16, full_range: bool) -> Self {
        Self {
            primaries,
            transfer,
            matrix,
            full_range,
            description_present: true,
        }
    }
    /// Preserve an omitted AV1 color_description without inventing explicit CICP.
    pub const fn without_description(full_range: bool) -> Self {
        Self {
            primaries: 2,
            transfer: 2,
            matrix: 2,
            full_range,
            description_present: false,
        }
    }
    pub const fn description_present(self) -> bool {
        self.description_present
    }
    pub const fn primaries(self) -> u16 {
        self.primaries
    }
    pub const fn transfer(self) -> u16 {
        self.transfer
    }
    pub const fn matrix(self) -> u16 {
        self.matrix
    }
    pub const fn full_range(self) -> bool {
        self.full_range
    }
}

/// All colour signalling associated with a frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorInformationSet {
    icc_profile: Option<Vec<u8>>,
    nclx: Option<NclxColorInformation>,
    av1: Option<Av1ColorInformation>,
    provenance: Vec<ColorProvenance>,
}

impl ColorInformationSet {
    pub const fn new() -> Self {
        Self {
            icc_profile: None,
            nclx: None,
            av1: None,
            provenance: Vec::new(),
        }
    }

    pub fn with_icc_profile(mut self, profile: Vec<u8>) -> Result<Self, HighresError> {
        self.set_icc_profile(profile)?;
        Ok(self)
    }
    pub fn set_icc_profile(&mut self, profile: Vec<u8>) -> Result<(), HighresError> {
        if profile.is_empty() {
            return Err(HighresError::InvalidMetadata("ICC profile is empty".into()));
        }
        self.icc_profile = Some(profile);
        self.add_provenance(ColorProvenance::EmbeddedIcc);
        Ok(())
    }
    pub fn with_nclx(mut self, nclx: NclxColorInformation) -> Self {
        self.set_nclx(nclx);
        self
    }
    pub fn set_nclx(&mut self, nclx: NclxColorInformation) {
        self.nclx = Some(nclx);
        self.add_provenance(ColorProvenance::ContainerNclx);
    }
    pub fn with_av1(mut self, av1: Av1ColorInformation) -> Self {
        self.set_av1(av1);
        self
    }
    pub fn set_av1(&mut self, av1: Av1ColorInformation) {
        self.av1 = Some(av1);
        self.add_provenance(ColorProvenance::Av1);
    }
    pub fn icc_profile(&self) -> Option<&[u8]> {
        self.icc_profile.as_deref()
    }
    pub const fn nclx(&self) -> Option<NclxColorInformation> {
        self.nclx
    }
    pub const fn av1(&self) -> Option<Av1ColorInformation> {
        self.av1
    }
    pub fn provenance(&self) -> &[ColorProvenance] {
        &self.provenance
    }
    fn add_provenance(&mut self, value: ColorProvenance) {
        if !self.provenance.contains(&value) {
            self.provenance.push(value);
        }
    }
}

/// A crop rectangle in coded/display pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Result<Self, HighresError> {
        if width == 0 || height == 0 {
            return Err(HighresError::InvalidMetadata(
                "crop dimensions must be non-zero".into(),
            ));
        }
        x.checked_add(width)
            .ok_or_else(|| HighresError::InvalidMetadata("crop overflows".into()))?;
        y.checked_add(height)
            .ok_or_else(|| HighresError::InvalidMetadata("crop overflows".into()))?;
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
    pub const fn x(self) -> u32 {
        self.x
    }
    pub const fn y(self) -> u32 {
        self.y
    }
    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rotation {
    #[default]
    None,
    Degrees90,
    Degrees180,
    Degrees270,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelAspectRatio {
    horizontal: u32,
    vertical: u32,
}

impl PixelAspectRatio {
    pub const ONE_TO_ONE: Self = Self {
        horizontal: 1,
        vertical: 1,
    };
    pub fn new(horizontal: u32, vertical: u32) -> Result<Self, HighresError> {
        if horizontal == 0 || vertical == 0 {
            return Err(HighresError::InvalidMetadata(
                "pixel aspect ratio must be non-zero".into(),
            ));
        }
        Ok(Self {
            horizontal,
            vertical,
        })
    }
    pub const fn horizontal(self) -> u32 {
        self.horizontal
    }
    pub const fn vertical(self) -> u32 {
        self.vertical
    }
}

/// `clap` clean-aperture rationals retained without applying them to pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanAperture {
    width_n: u32,
    width_d: u32,
    height_n: u32,
    height_d: u32,
    horizontal_offset_n: u32,
    horizontal_offset_d: u32,
    vertical_offset_n: u32,
    vertical_offset_d: u32,
}

impl CleanAperture {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        width_n: u32,
        width_d: u32,
        height_n: u32,
        height_d: u32,
        horizontal_offset_n: u32,
        horizontal_offset_d: u32,
        vertical_offset_n: u32,
        vertical_offset_d: u32,
    ) -> Result<Self, HighresError> {
        if [width_d, height_d, horizontal_offset_d, vertical_offset_d].contains(&0) {
            return Err(HighresError::InvalidMetadata(
                "clean-aperture denominator must be non-zero".into(),
            ));
        }
        Ok(Self {
            width_n,
            width_d,
            height_n,
            height_d,
            horizontal_offset_n,
            horizontal_offset_d,
            vertical_offset_n,
            vertical_offset_d,
        })
    }

    pub const fn width(self) -> (u32, u32) {
        (self.width_n, self.width_d)
    }
    pub const fn height(self) -> (u32, u32) {
        (self.height_n, self.height_d)
    }
    pub const fn horizontal_offset(self) -> (u32, u32) {
        (self.horizontal_offset_n, self.horizontal_offset_d)
    }
    pub const fn vertical_offset(self) -> (u32, u32) {
        (self.vertical_offset_n, self.vertical_offset_d)
    }
}

impl Default for PixelAspectRatio {
    fn default() -> Self {
        Self::ONE_TO_ONE
    }
}

/// Non-pixel metadata owned by a frame. Source colour is separate from any
/// active output colour descriptor after an explicit conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameMetadata {
    source_color: ColorInformationSet,
    tags: Metadata,
    crop: Option<Rect>,
    clean_aperture: Option<CleanAperture>,
    rotation: Rotation,
    mirror_horizontal: bool,
    mirror_vertical: bool,
    pixel_aspect_ratio: Option<PixelAspectRatio>,
    coded_dimensions: Option<(u32, u32)>,
    render_dimensions: Option<(u32, u32)>,
}

impl Default for FrameMetadata {
    fn default() -> Self {
        Self {
            source_color: ColorInformationSet::default(),
            tags: Metadata::new(),
            crop: None,
            clean_aperture: None,
            rotation: Rotation::None,
            mirror_horizontal: false,
            mirror_vertical: false,
            pixel_aspect_ratio: None,
            coded_dimensions: None,
            render_dimensions: None,
        }
    }
}

impl FrameMetadata {
    pub fn new(source_color: ColorInformationSet) -> Self {
        Self {
            source_color,
            ..Self::default()
        }
    }
    pub fn source_color(&self) -> &ColorInformationSet {
        &self.source_color
    }
    pub fn tags(&self) -> &Metadata {
        &self.tags
    }
    pub fn tags_mut(&mut self) -> &mut Metadata {
        &mut self.tags
    }
    pub fn crop(&self) -> Option<Rect> {
        self.crop
    }
    pub fn clean_aperture(&self) -> Option<CleanAperture> {
        self.clean_aperture
    }
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }
    pub fn mirror_horizontal(&self) -> bool {
        self.mirror_horizontal
    }
    pub fn mirror_vertical(&self) -> bool {
        self.mirror_vertical
    }
    pub fn pixel_aspect_ratio(&self) -> Option<PixelAspectRatio> {
        self.pixel_aspect_ratio
    }
    pub fn coded_dimensions(&self) -> Option<(u32, u32)> {
        self.coded_dimensions
    }
    pub fn render_dimensions(&self) -> Option<(u32, u32)> {
        self.render_dimensions
    }
    pub fn set_crop(&mut self, crop: Option<Rect>) {
        self.crop = crop;
    }
    pub fn set_clean_aperture(&mut self, clean_aperture: Option<CleanAperture>) {
        self.clean_aperture = clean_aperture;
    }
    pub fn set_rotation(&mut self, rotation: Rotation) {
        self.rotation = rotation;
    }
    pub fn set_mirror(&mut self, horizontal: bool, vertical: bool) {
        self.mirror_horizontal = horizontal;
        self.mirror_vertical = vertical;
    }
    pub fn set_pixel_aspect_ratio(&mut self, ratio: Option<PixelAspectRatio>) {
        self.pixel_aspect_ratio = ratio;
    }
    pub fn set_coded_dimensions(&mut self, dimensions: Option<(u32, u32)>) {
        self.coded_dimensions = dimensions;
    }
    pub fn set_render_dimensions(&mut self, dimensions: Option<(u32, u32)>) {
        self.render_dimensions = dimensions;
    }

    pub(crate) fn validate_bounds(&self, width: u32, height: u32) -> Result<(), HighresError> {
        if let Some(crop) = self.crop {
            let right = crop
                .x
                .checked_add(crop.width)
                .ok_or_else(|| HighresError::InvalidMetadata("crop right edge overflows".into()))?;
            let bottom = crop.y.checked_add(crop.height).ok_or_else(|| {
                HighresError::InvalidMetadata("crop bottom edge overflows".into())
            })?;
            if right > width || bottom > height {
                return Err(HighresError::InvalidMetadata(
                    "crop rectangle exceeds image dimensions".into(),
                ));
            }
        }
        let ratio = self.pixel_aspect_ratio.unwrap_or_default();
        if ratio.horizontal == 0 || ratio.vertical == 0 {
            return Err(HighresError::InvalidMetadata(
                "pixel aspect ratio must be non-zero".into(),
            ));
        }
        if self
            .coded_dimensions
            .into_iter()
            .chain(self.render_dimensions)
            .any(|(width, height)| width == 0 || height == 0)
        {
            return Err(HighresError::InvalidMetadata(
                "metadata dimensions must be non-zero".into(),
            ));
        }
        Ok(())
    }
}
