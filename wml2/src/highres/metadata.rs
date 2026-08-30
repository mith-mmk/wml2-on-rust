//! Metadata retained by the checked high-resolution representation.

use super::HighresError;
use super::ProcessingError;
#[cfg(feature = "avif")]
use super::allocation::ConstructionLedger;
use super::domain::{RgbPrimaries, SampleDomain};
use super::output_plan::{OwnerElement, OwnerKey, OwnerSink};
#[cfg(feature = "avif")]
use super::ownership::LedgerOwnerSink;
use super::ownership::OrdinaryOwnerSink;
use crate::metadata::Metadata;
use std::mem::size_of;

fn copy_owner<T: Copy + OwnerElement, S: OwnerSink>(
    sink: &mut S,
    key: OwnerKey,
    source: &[T],
) -> Result<Vec<T>, ProcessingError> {
    let mut copy = sink.fresh::<T>(key, source.len())?;
    copy.extend_from_slice(source);
    Ok(copy)
}

/// Origin of one piece of colour signalling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProvenance {
    EmbeddedIcc,
    ContainerNclx,
    Av1,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccColorType {
    Prof,
    Ricc,
}

/// The authority used for one side of an explicit ICC conversion.
///
/// The profile bytes themselves remain in the frame's colour information;
/// this small copyable record makes the conversion history explicit without
/// duplicating the profile allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IccConversionAuthority {
    color_type: IccColorType,
}

impl IccConversionAuthority {
    pub(crate) const fn from_color_type(color_type: IccColorType) -> Self {
        Self { color_type }
    }

    pub const fn color_type(self) -> IccColorType {
        self.color_type
    }
}

/// Authority for the primaries recorded by a conversion route. ICC routes
/// retain the profile-defined primaries instead of pretending they are sRGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionPrimaries {
    Explicit(RgbPrimaries),
    IccDefined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownColorInformation {
    pub color_type: [u8; 4],
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSubsampling {
    subsampling_type: u8,
    subsampling_location: u8,
}

impl PixelSubsampling {
    pub const fn new(subsampling_type: u8, subsampling_location: u8) -> Self {
        Self {
            subsampling_type,
            subsampling_location,
        }
    }
    pub const fn subsampling_type(self) -> u8 {
        self.subsampling_type
    }
    pub const fn subsampling_location(self) -> u8 {
        self.subsampling_location
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelChannelInformation {
    channel_idc: u8,
    component_format: u8,
    subsampling: Option<PixelSubsampling>,
}

impl PixelChannelInformation {
    pub const fn new(
        channel_idc: u8,
        component_format: u8,
        subsampling: Option<PixelSubsampling>,
    ) -> Self {
        Self {
            channel_idc,
            component_format,
            subsampling,
        }
    }
    pub const fn channel_idc(self) -> u8 {
        self.channel_idc
    }
    pub const fn component_format(self) -> u8 {
        self.component_format
    }
    pub const fn subsampling(self) -> Option<PixelSubsampling> {
        self.subsampling
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelInformation {
    bits_per_channel: Vec<u8>,
    extended_channels: Option<Vec<PixelChannelInformation>>,
}

impl PixelInformation {
    pub fn new(
        bits_per_channel: Vec<u8>,
        extended_channels: Option<Vec<PixelChannelInformation>>,
    ) -> Result<Self, HighresError> {
        if bits_per_channel.is_empty() || bits_per_channel.contains(&0) {
            return Err(HighresError::InvalidMetadata(
                "pixi must contain non-zero channel precision".into(),
            ));
        }
        if let Some(channels) = &extended_channels
            && bits_per_channel.len() != channels.len()
        {
            return Err(HighresError::InvalidMetadata(
                "pixi precision count must match extended channel count".into(),
            ));
        }
        Ok(Self {
            bits_per_channel,
            extended_channels,
        })
    }
    pub fn bits_per_channel(&self) -> &[u8] {
        &self.bits_per_channel
    }
    pub fn extended_channels(&self) -> Option<&[PixelChannelInformation]> {
        self.extended_channels.as_deref()
    }
    #[cfg(test)]
    pub(crate) fn bits_capacity_for_test(&self) -> usize {
        self.bits_per_channel.capacity()
    }
    #[cfg(test)]
    pub(crate) fn extended_capacity_for_test(&self) -> Option<usize> {
        self.extended_channels.as_ref().map(Vec::capacity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawRational {
    numerator_bits: u32,
    denominator: u32,
}

impl RawRational {
    pub fn new(numerator_bits: u32, denominator: u32) -> Result<Self, HighresError> {
        if denominator == 0 {
            return Err(HighresError::InvalidMetadata(
                "rational denominator must be non-zero".into(),
            ));
        }
        Ok(Self {
            numerator_bits,
            denominator,
        })
    }
    pub const fn numerator_bits(self) -> u32 {
        self.numerator_bits
    }
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanAperture {
    width: RawRational,
    height: RawRational,
    horizontal_offset: RawRational,
    vertical_offset: RawRational,
}

impl CleanAperture {
    pub const fn new(
        width: RawRational,
        height: RawRational,
        horizontal_offset: RawRational,
        vertical_offset: RawRational,
    ) -> Self {
        Self {
            width,
            height,
            horizontal_offset,
            vertical_offset,
        }
    }
    pub const fn width(self) -> RawRational {
        self.width
    }
    pub const fn height(self) -> RawRational {
        self.height
    }
    pub const fn horizontal_offset(self) -> RawRational {
        self.horizontal_offset
    }
    pub const fn vertical_offset(self) -> RawRational {
        self.vertical_offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Av1Description {
    present: bool,
    monochrome: Option<bool>,
    full_range: Option<bool>,
    subsampling_x: Option<bool>,
    subsampling_y: Option<bool>,
    chroma_sample_position: Option<u8>,
}

impl Av1Description {
    pub fn new(present: bool) -> Self {
        Self {
            present,
            monochrome: None,
            full_range: None,
            subsampling_x: None,
            subsampling_y: None,
            chroma_sample_position: None,
        }
    }
    pub const fn present(self) -> bool {
        self.present
    }
    pub const fn monochrome(self) -> Option<bool> {
        self.monochrome
    }
    pub const fn full_range(self) -> Option<bool> {
        self.full_range
    }
    pub const fn subsampling_x(self) -> Option<bool> {
        self.subsampling_x
    }
    pub const fn subsampling_y(self) -> Option<bool> {
        self.subsampling_y
    }
    pub const fn chroma_sample_position(self) -> Option<u8> {
        self.chroma_sample_position
    }
    pub fn with_flags(
        mut self,
        monochrome: Option<bool>,
        full_range: Option<bool>,
        subsampling_x: Option<bool>,
        subsampling_y: Option<bool>,
        chroma_sample_position: Option<u8>,
    ) -> Self {
        self.monochrome = monochrome;
        self.full_range = full_range;
        self.subsampling_x = subsampling_x;
        self.subsampling_y = subsampling_y;
        self.chroma_sample_position = chroma_sample_position;
        self
    }
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
}

pub(crate) const AV1_COLOR_INFORMATION_BYTES: usize = size_of::<Av1ColorInformation>();

impl Av1ColorInformation {
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

/// All colour signalling associated with a frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorInformationSet {
    icc_profile: Option<Vec<u8>>,
    icc_color_type: Option<IccColorType>,
    nclx: Option<NclxColorInformation>,
    av1: Option<Av1ColorInformation>,
    provenance: Vec<ColorProvenance>,
    unknown_colr: Vec<UnknownColorInformation>,
}

impl ColorInformationSet {
    pub const fn new() -> Self {
        Self {
            icc_profile: None,
            icc_color_type: None,
            nclx: None,
            av1: None,
            provenance: Vec::new(),
            unknown_colr: Vec::new(),
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

    /// Replace the ICC payload when its provenance is supplied by an
    /// explicit conversion operation.  The ordinary setter intentionally
    /// records `EmbeddedIcc`; an adapter replacing an input profile must not
    /// manufacture that provenance (and must preserve the existing entries).
    pub(crate) fn set_icc_profile_admitted(
        &mut self,
        profile: Vec<u8>,
    ) -> Result<(), ProcessingError> {
        if profile.is_empty() {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "ICC profile is empty".into(),
            )));
        }
        self.icc_profile = Some(profile);
        Ok(())
    }
    pub fn with_icc_color_type(mut self, color_type: IccColorType) -> Self {
        self.icc_color_type = Some(color_type);
        self
    }
    pub fn set_icc_color_type(&mut self, color_type: IccColorType) {
        self.icc_color_type = Some(color_type);
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
    pub(crate) fn icc_profile_capacity(&self) -> usize {
        self.icc_profile.as_ref().map_or(0, Vec::capacity)
    }
    pub const fn icc_color_type(&self) -> Option<IccColorType> {
        self.icc_color_type
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
    #[cfg(test)]
    pub(crate) fn provenance_capacity_for_test(&self) -> usize {
        self.provenance.capacity()
    }
    /// Append provenance only when the owner has already admitted capacity.
    /// Conversion adapters use this feature-independent hook so they cannot
    /// grow the vector behind the construction ledger.
    pub(crate) fn push_admitted_provenance(
        &mut self,
        value: ColorProvenance,
    ) -> Result<(), ProcessingError> {
        if self.provenance.contains(&value) {
            return Ok(());
        }
        if self.provenance.len() == self.provenance.capacity() {
            return Err(ProcessingError::Allocation(
                "colour provenance capacity was not admitted".into(),
            ));
        }
        self.provenance.push(value);
        Ok(())
    }
    #[cfg(feature = "avif")]
    pub(crate) fn provenance_capacity(&self) -> usize {
        self.provenance.capacity()
    }
    #[cfg(feature = "avif")]
    pub(crate) fn try_reserve_provenance(&mut self, additional: usize) -> Result<(), HighresError> {
        self.provenance.try_reserve_exact(additional).map_err(|_| {
            HighresError::InvalidMetadata("colour provenance allocation failed".into())
        })
    }
    pub fn unknown_colr(&self) -> &[UnknownColorInformation] {
        &self.unknown_colr
    }
    #[cfg(test)]
    pub(crate) fn unknown_colr_capacity_for_test(&self) -> usize {
        self.unknown_colr.capacity()
    }
    #[cfg(feature = "avif")]
    pub(crate) fn unknown_colr_mut_bridge(&mut self) -> &mut Vec<UnknownColorInformation> {
        &mut self.unknown_colr
    }
    #[cfg(all(feature = "avif", test))]
    pub(crate) fn unknown_colr_capacity(&self) -> usize {
        self.unknown_colr.capacity()
    }
    #[allow(dead_code)]
    pub(crate) fn try_reserve_unknown_colr(
        &mut self,
        additional: usize,
    ) -> Result<(), HighresError> {
        self.unknown_colr
            .try_reserve_exact(additional)
            .map_err(|_| HighresError::InvalidMetadata("unknown colour allocation failed".into()))
    }
    #[cfg(feature = "avif")]
    pub(crate) fn try_reserve_unknown_colr_bridge(
        &mut self,
        additional: usize,
    ) -> Result<(), ProcessingError> {
        self.unknown_colr
            .try_reserve_exact(additional)
            .map_err(|_| ProcessingError::Allocation("unknown colour allocation failed".into()))
    }
    pub fn push_unknown_colr(&mut self, color: UnknownColorInformation) {
        self.unknown_colr.push(color);
    }

    pub(crate) fn try_clone_owned(&self) -> Result<Self, HighresError> {
        let mut sink = OrdinaryOwnerSink;
        self.try_clone_owned_with_sink(&mut sink)
            .map_err(|error| match error {
                ProcessingError::Allocation(message) => HighresError::InvalidMetadata(message),
                ProcessingError::Invalid(error) => error,
                other => HighresError::InvalidMetadata(other.to_string()),
            })
    }

    #[cfg(feature = "avif")]
    pub(crate) fn try_clone_owned_with_ledger(
        &self,
        ledger: &mut ConstructionLedger,
    ) -> Result<Self, ProcessingError> {
        let mut sink = LedgerOwnerSink { ledger };
        self.try_clone_owned_with_sink(&mut sink)
    }

    pub(crate) fn try_clone_owned_with_sink<S: OwnerSink>(
        &self,
        sink: &mut S,
    ) -> Result<Self, ProcessingError> {
        self.try_clone_owned_with_sink_mode(sink, true)
    }

    fn try_clone_owned_with_sink_mode<S: OwnerSink>(
        &self,
        sink: &mut S,
        include_icc: bool,
    ) -> Result<Self, ProcessingError> {
        self.try_clone_owned_with_sink_mode_extra(sink, include_icc, 0)
    }

    fn try_clone_owned_with_sink_mode_extra<S: OwnerSink>(
        &self,
        sink: &mut S,
        include_icc: bool,
        extra_provenance: usize,
    ) -> Result<Self, ProcessingError> {
        let provenance_count = self
            .provenance
            .len()
            .checked_add(extra_provenance)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("colour provenance count overflows".into())
            })?;
        let mut provenance =
            sink.fresh::<ColorProvenance>(OwnerKey::Provenance, provenance_count)?;
        provenance.extend_from_slice(&self.provenance);
        let icc_profile = include_icc
            .then_some(self.icc_profile.as_deref())
            .flatten()
            .map(|profile| copy_owner(sink, OwnerKey::IccProfile, profile))
            .transpose()?;
        let mut unknown_colr =
            sink.fresh::<UnknownColorInformation>(OwnerKey::UnknownOuter, self.unknown_colr.len())?;
        for (index, color) in self.unknown_colr.iter().enumerate() {
            let payload = copy_owner(sink, OwnerKey::UnknownPayload(index), &color.payload)?;
            unknown_colr.push(UnknownColorInformation {
                color_type: color.color_type,
                payload,
            });
        }
        Ok(Self::from_owned_parts(
            provenance,
            icc_profile,
            self.icc_color_type,
            self.nclx,
            self.av1,
            unknown_colr,
        ))
    }
    pub(crate) fn from_owned_parts(
        provenance: Vec<ColorProvenance>,
        icc_profile: Option<Vec<u8>>,
        icc_color_type: Option<IccColorType>,
        nclx: Option<NclxColorInformation>,
        av1: Option<Av1ColorInformation>,
        unknown_colr: Vec<UnknownColorInformation>,
    ) -> Self {
        Self {
            icc_profile,
            icc_color_type,
            nclx,
            av1,
            provenance,
            unknown_colr,
        }
    }

    /// Return the bytes requested by a fresh owned clone.  This deliberately
    /// uses lengths rather than the capacities of the borrowed owner; the
    /// bridge allocates each clone with `try_reserve_exact` for its contents.
    pub(crate) fn fresh_clone_owned_bytes(&self) -> Result<usize, HighresError> {
        self.fresh_clone_owned_bytes_mode(true)
    }

    pub(crate) fn fresh_clone_owned_bytes_without_icc(&self) -> Result<usize, HighresError> {
        self.fresh_clone_owned_bytes_mode(false)
    }

    fn fresh_clone_owned_bytes_mode(&self, include_icc: bool) -> Result<usize, HighresError> {
        let mut total = self.provenance.len();
        if include_icc {
            if let Some(profile) = &self.icc_profile {
                total = total.checked_add(profile.len()).ok_or_else(|| {
                    HighresError::InvalidMetadata("colour metadata size overflows".into())
                })?;
            }
        }
        // nclx and the AV1 colour description are copied as inline fields in
        // the destination metadata.  Only heap-backed owners belong in the
        // fresh-clone reservation ledger; their logical retained sizes stay
        // accounted by `owned_bytes`/`metadata_bytes`.
        total = total
            .checked_add(
                self.unknown_colr
                    .len()
                    .checked_mul(size_of::<UnknownColorInformation>())
                    .ok_or_else(|| {
                        HighresError::InvalidMetadata("colour metadata size overflows".into())
                    })?,
            )
            .ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        for color in &self.unknown_colr {
            total = total.checked_add(color.payload.len()).ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        }
        Ok(total)
    }

    pub(crate) fn fresh_clone_retained_bytes(&self) -> Result<usize, HighresError> {
        self.fresh_clone_retained_bytes_mode(true)
    }

    pub(crate) fn fresh_clone_retained_bytes_without_icc(&self) -> Result<usize, HighresError> {
        self.fresh_clone_retained_bytes_mode(false)
    }

    fn fresh_clone_retained_bytes_mode(&self, include_icc: bool) -> Result<usize, HighresError> {
        let mut total = self.fresh_clone_owned_bytes_mode(include_icc)?;
        if self.nclx.is_some() {
            total = total.checked_add(8).ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        }
        if self.av1.is_some() {
            total = total
                .checked_add(AV1_COLOR_INFORMATION_BYTES)
                .ok_or_else(|| {
                    HighresError::InvalidMetadata("colour metadata size overflows".into())
                })?;
        }
        Ok(total)
    }

    #[cfg(feature = "avif")]
    pub(crate) fn av1_additional_bytes(&self) -> Result<usize, HighresError> {
        let provenance = usize::from(!self.provenance.contains(&ColorProvenance::Av1));
        AV1_COLOR_INFORMATION_BYTES
            .checked_add(provenance)
            .ok_or_else(|| HighresError::InvalidMetadata("colour metadata size overflows".into()))
    }
    fn add_provenance(&mut self, value: ColorProvenance) {
        if !self.provenance.contains(&value) {
            self.provenance.push(value);
        }
    }

    pub(crate) fn owned_bytes(&self) -> Result<usize, HighresError> {
        let mut total = self.provenance.capacity();
        if let Some(profile) = &self.icc_profile {
            total = total.checked_add(profile.capacity()).ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        }
        if self.nclx.is_some() {
            total = total.checked_add(8).ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        }
        if self.av1.is_some() {
            total = total
                .checked_add(AV1_COLOR_INFORMATION_BYTES)
                .ok_or_else(|| {
                    HighresError::InvalidMetadata("colour metadata size overflows".into())
                })?;
        }
        total = total
            .checked_add(
                self.unknown_colr
                    .capacity()
                    .checked_mul(size_of::<UnknownColorInformation>())
                    .ok_or_else(|| {
                        HighresError::InvalidMetadata("colour metadata size overflows".into())
                    })?,
            )
            .ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        for color in &self.unknown_colr {
            total = total.checked_add(color.payload.capacity()).ok_or_else(|| {
                HighresError::InvalidMetadata("colour metadata size overflows".into())
            })?;
        }
        Ok(total)
    }

    #[cfg(feature = "avif")]
    pub(crate) fn fresh_owned_bytes(&self) -> Result<usize, HighresError> {
        self.fresh_clone_retained_bytes()
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
pub enum GeometryOperation {
    Crop(Rect),
    Rotate(Rotation),
    MirrorHorizontal,
    MirrorVertical,
    PixelAspect(PixelAspectRatio),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepetitionCount {
    Finite(u32),
    Infinite,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceInformation {
    frame_count: u64,
    duration: RawRational,
    repetition: RepetitionCount,
}

impl SequenceInformation {
    pub fn new(frame_count: u64, duration: RawRational, repetition: RepetitionCount) -> Self {
        Self {
            frame_count,
            duration,
            repetition,
        }
    }
    pub const fn frame_count(self) -> u64 {
        self.frame_count
    }
    pub const fn duration(self) -> RawRational {
        self.duration
    }
    pub const fn repetition(self) -> RepetitionCount {
        self.repetition
    }
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

impl Default for PixelAspectRatio {
    fn default() -> Self {
        Self::ONE_TO_ONE
    }
}

/// Non-pixel metadata owned by a frame. Source colour is separate from any
/// active output colour descriptor after an explicit conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastConversion {
    source: ConversionSource,
    source_cicp: Option<NclxColorInformation>,
    source_icc: Option<IccConversionAuthority>,
    source_primaries_authority: ConversionPrimaries,
    source_primaries: RgbPrimaries,
    source_domain: SampleDomain,
    destination: ConversionDestination,
    destination_icc: Option<IccConversionAuthority>,
    destination_primaries_authority: ConversionPrimaries,
    destination_domain: SampleDomain,
    destination_primaries: RgbPrimaries,
    intent: ConversionIntent,
    native: Option<NativeInterpretation>,
    white_adaptation: ConversionWhiteAdaptation,
    alpha: ConversionAlpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionSource {
    ActiveCicp,
    ExplicitCicp,
    LinearRelative,
    Icc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionDestination {
    LinearRelativeRgb,
    LinearAbsoluteNitsRgb,
    HlgSceneLinearRgb,
    Icc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionIntent {
    RelativeColorimetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeInterpretation {
    full_range: bool,
    matrix: u16,
    chroma_location: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionWhiteAdaptation {
    RequireSameWhite,
    IccD50Pcs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionAlpha {
    PreserveAssociation,
}

pub(crate) struct LastConversionParts {
    pub(crate) source: ConversionSource,
    pub(crate) source_cicp: Option<NclxColorInformation>,
    pub(crate) source_icc: Option<IccConversionAuthority>,
    pub(crate) source_primaries_authority: ConversionPrimaries,
    pub(crate) source_primaries: RgbPrimaries,
    pub(crate) source_domain: SampleDomain,
    pub(crate) destination: ConversionDestination,
    pub(crate) destination_icc: Option<IccConversionAuthority>,
    pub(crate) destination_primaries_authority: ConversionPrimaries,
    pub(crate) destination_domain: SampleDomain,
    pub(crate) destination_primaries: RgbPrimaries,
    pub(crate) intent: ConversionIntent,
    pub(crate) native: Option<NativeInterpretation>,
    pub(crate) white_adaptation: ConversionWhiteAdaptation,
    pub(crate) alpha: ConversionAlpha,
}

impl LastConversion {
    pub(crate) const fn from_parts(parts: LastConversionParts) -> Self {
        Self {
            source: parts.source,
            source_cicp: parts.source_cicp,
            source_icc: parts.source_icc,
            source_primaries_authority: parts.source_primaries_authority,
            source_primaries: parts.source_primaries,
            source_domain: parts.source_domain,
            destination: parts.destination,
            destination_icc: parts.destination_icc,
            destination_primaries_authority: parts.destination_primaries_authority,
            destination_domain: parts.destination_domain,
            destination_primaries: parts.destination_primaries,
            intent: parts.intent,
            native: parts.native,
            white_adaptation: parts.white_adaptation,
            alpha: parts.alpha,
        }
    }

    pub const fn source(self) -> ConversionSource {
        self.source
    }

    pub const fn source_cicp(self) -> Option<NclxColorInformation> {
        self.source_cicp
    }

    pub const fn source_icc(self) -> Option<IccConversionAuthority> {
        self.source_icc
    }

    pub const fn source_primaries_authority(self) -> ConversionPrimaries {
        self.source_primaries_authority
    }

    pub const fn source_primaries(self) -> RgbPrimaries {
        self.source_primaries
    }

    /// Return the truthful primaries authority for this route.
    ///
    /// In particular, ICC-defined primaries are reported as
    /// [`ConversionPrimaries::IccDefined`] instead of being represented by
    /// the CICP-oriented [`RgbPrimaries`] compatibility value returned by
    /// [`Self::source_primaries`].  Consumers that must not infer colour
    /// information from a compatibility value should use this accessor (or
    /// [`Self::source_primaries_if_defined`]).
    pub const fn source_primaries_info(self) -> ConversionPrimaries {
        self.source_primaries_authority
    }

    /// Return concrete source primaries when the route declares them.
    /// ICC profiles define their own primaries, so no RGB fallback is exposed
    /// through this accessor for ICC-defined routes.
    pub const fn source_primaries_if_defined(self) -> Option<RgbPrimaries> {
        match self.source_primaries_authority {
            ConversionPrimaries::Explicit(value) => Some(value),
            ConversionPrimaries::IccDefined => None,
        }
    }

    pub const fn source_domain(self) -> SampleDomain {
        self.source_domain
    }

    pub const fn destination(self) -> ConversionDestination {
        self.destination
    }

    pub const fn destination_icc(self) -> Option<IccConversionAuthority> {
        self.destination_icc
    }

    pub const fn destination_primaries_authority(self) -> ConversionPrimaries {
        self.destination_primaries_authority
    }

    pub const fn destination_domain(self) -> SampleDomain {
        self.destination_domain
    }

    pub const fn destination_primaries(self) -> RgbPrimaries {
        self.destination_primaries
    }

    /// Return the truthful destination primaries authority for this route.
    /// ICC-defined primaries are never exposed as a fabricated RGB value.
    pub const fn destination_primaries_info(self) -> ConversionPrimaries {
        self.destination_primaries_authority
    }

    /// Return concrete destination primaries when the route declares them.
    pub const fn destination_primaries_if_defined(self) -> Option<RgbPrimaries> {
        match self.destination_primaries_authority {
            ConversionPrimaries::Explicit(value) => Some(value),
            ConversionPrimaries::IccDefined => None,
        }
    }

    pub const fn intent(self) -> ConversionIntent {
        self.intent
    }

    pub const fn native(self) -> Option<NativeInterpretation> {
        self.native
    }

    pub const fn white_adaptation(self) -> ConversionWhiteAdaptation {
        self.white_adaptation
    }

    pub const fn alpha(self) -> ConversionAlpha {
        self.alpha
    }
}

impl NativeInterpretation {
    pub(crate) const fn from_parts(
        full_range: bool,
        matrix: u16,
        chroma_location: Option<u8>,
    ) -> Self {
        Self {
            full_range,
            matrix,
            chroma_location,
        }
    }

    pub const fn full_range(self) -> bool {
        self.full_range
    }

    pub const fn matrix(self) -> u16 {
        self.matrix
    }

    pub const fn chroma_location(self) -> Option<u8> {
        self.chroma_location
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameMetadata {
    source_color: ColorInformationSet,
    tags: Metadata,
    crop: Option<Rect>,
    rotation: Rotation,
    mirror_horizontal: bool,
    mirror_vertical: bool,
    pixel_aspect_ratio: Option<PixelAspectRatio>,
    clean_aperture: Option<CleanAperture>,
    coded_geometry: Vec<GeometryOperation>,
    render_geometry: Vec<GeometryOperation>,
    av1_description: Option<Av1Description>,
    pixel_information: Option<PixelInformation>,
    coded_dimensions: Option<(u32, u32)>,
    render_dimensions: Option<(u32, u32)>,
    sequence_information: Option<SequenceInformation>,
    last_conversion: Option<LastConversion>,
}

impl Default for FrameMetadata {
    fn default() -> Self {
        Self {
            source_color: ColorInformationSet::default(),
            tags: Metadata::new(),
            crop: None,
            rotation: Rotation::None,
            mirror_horizontal: false,
            mirror_vertical: false,
            pixel_aspect_ratio: None,
            clean_aperture: None,
            coded_geometry: Vec::new(),
            render_geometry: Vec::new(),
            av1_description: None,
            pixel_information: None,
            coded_dimensions: None,
            render_dimensions: None,
            sequence_information: None,
            last_conversion: None,
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
    pub fn source_color_mut(&mut self) -> &mut ColorInformationSet {
        &mut self.source_color
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
    pub fn clean_aperture(&self) -> Option<CleanAperture> {
        self.clean_aperture
    }
    pub fn coded_geometry(&self) -> &[GeometryOperation] {
        &self.coded_geometry
    }
    pub fn render_geometry(&self) -> &[GeometryOperation] {
        &self.render_geometry
    }
    #[cfg(test)]
    pub(crate) fn coded_geometry_capacity_for_test(&self) -> usize {
        self.coded_geometry.capacity()
    }
    #[cfg(test)]
    pub(crate) fn render_geometry_capacity_for_test(&self) -> usize {
        self.render_geometry.capacity()
    }
    pub fn av1_description(&self) -> Option<Av1Description> {
        self.av1_description
    }
    pub fn pixel_information(&self) -> Option<&PixelInformation> {
        self.pixel_information.as_ref()
    }
    pub fn coded_dimensions(&self) -> Option<(u32, u32)> {
        self.coded_dimensions
    }
    pub fn render_dimensions(&self) -> Option<(u32, u32)> {
        self.render_dimensions
    }
    pub fn sequence_information(&self) -> Option<SequenceInformation> {
        self.sequence_information
    }
    pub fn last_conversion(&self) -> Option<LastConversion> {
        self.last_conversion
    }
    pub fn set_crop(&mut self, crop: Option<Rect>) {
        self.crop = crop;
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
    pub fn set_clean_aperture(&mut self, aperture: Option<CleanAperture>) {
        self.clean_aperture = aperture;
    }
    pub fn set_coded_geometry(&mut self, operations: Vec<GeometryOperation>) {
        self.coded_geometry = operations;
    }
    pub fn set_render_geometry(&mut self, operations: Vec<GeometryOperation>) {
        self.render_geometry = operations;
    }
    pub fn set_av1_description(&mut self, description: Option<Av1Description>) {
        self.av1_description = description;
    }
    pub fn set_pixel_information(&mut self, information: Option<PixelInformation>) {
        self.pixel_information = information;
    }
    pub fn set_coded_dimensions(&mut self, dimensions: Option<(u32, u32)>) {
        self.coded_dimensions = dimensions;
    }
    pub fn set_render_dimensions(&mut self, dimensions: Option<(u32, u32)>) {
        self.render_dimensions = dimensions;
    }
    pub fn set_sequence_information(&mut self, information: Option<SequenceInformation>) {
        self.sequence_information = information;
    }

    pub(crate) fn set_last_conversion(&mut self, record: LastConversion) {
        self.last_conversion = Some(record);
    }

    pub(crate) fn try_clone_for_conversion_with_sink<S: OwnerSink>(
        &self,
        sink: &mut S,
    ) -> Result<Self, ProcessingError> {
        if self.tags.capacity() != 0 {
            return Err(ProcessingError::Unsupported(
                "metadata tags with retained capacity are unsupported by this conversion slice"
                    .into(),
            ));
        }
        let coded_geometry = copy_owner(sink, OwnerKey::CodedGeometry, &self.coded_geometry)?;
        let render_geometry = copy_owner(sink, OwnerKey::RenderGeometry, &self.render_geometry)?;
        let pixel_information = if let Some(information) = &self.pixel_information {
            let bits_per_channel =
                copy_owner(sink, OwnerKey::PixiBits, &information.bits_per_channel)?;
            let extended_channels = information
                .extended_channels
                .as_ref()
                .map(|channels| copy_owner(sink, OwnerKey::PixiExtended, channels))
                .transpose()?;
            Some(PixelInformation {
                bits_per_channel,
                extended_channels,
            })
        } else {
            None
        };
        let source_color = self.source_color.try_clone_owned_with_sink(sink)?;
        if self.source_color.nclx.is_some() {
            sink.commit_inline(OwnerKey::SourceNclx, 8)?;
        }
        if self.source_color.av1.is_some() {
            sink.commit_inline(OwnerKey::SourceAv1, AV1_COLOR_INFORMATION_BYTES)?;
        }
        if self.coded_dimensions.is_some() {
            sink.commit_inline(OwnerKey::CodedDimensions, 8)?;
        }
        if self.render_dimensions.is_some() {
            sink.commit_inline(OwnerKey::RenderDimensions, 8)?;
        }
        Ok(Self {
            source_color,
            tags: Metadata::new(),
            crop: self.crop,
            rotation: self.rotation,
            mirror_horizontal: self.mirror_horizontal,
            mirror_vertical: self.mirror_vertical,
            pixel_aspect_ratio: self.pixel_aspect_ratio,
            clean_aperture: self.clean_aperture,
            coded_geometry,
            render_geometry,
            av1_description: self.av1_description,
            pixel_information,
            coded_dimensions: self.coded_dimensions,
            render_dimensions: self.render_dimensions,
            sequence_information: self.sequence_information,
            last_conversion: self.last_conversion,
        })
    }

    pub(crate) fn try_clone_for_icc_conversion_with_sink<S: OwnerSink>(
        &self,
        sink: &mut S,
    ) -> Result<Self, ProcessingError> {
        if self.tags.capacity() != 0 {
            return Err(ProcessingError::Unsupported(
                "metadata tags with retained capacity are unsupported by this conversion slice"
                    .into(),
            ));
        }
        let coded_geometry = copy_owner(sink, OwnerKey::CodedGeometry, &self.coded_geometry)?;
        let render_geometry = copy_owner(sink, OwnerKey::RenderGeometry, &self.render_geometry)?;
        let pixel_information = if let Some(information) = &self.pixel_information {
            let bits_per_channel =
                copy_owner(sink, OwnerKey::PixiBits, &information.bits_per_channel)?;
            let extended_channels = information
                .extended_channels
                .as_ref()
                .map(|channels| copy_owner(sink, OwnerKey::PixiExtended, channels))
                .transpose()?;
            Some(PixelInformation {
                bits_per_channel,
                extended_channels,
            })
        } else {
            None
        };
        let source_color = self
            .source_color
            .try_clone_owned_with_sink_mode_extra(sink, false, 1)?;
        if self.source_color.nclx.is_some() {
            sink.commit_inline(OwnerKey::SourceNclx, 8)?;
        }
        if self.source_color.av1.is_some() {
            sink.commit_inline(OwnerKey::SourceAv1, AV1_COLOR_INFORMATION_BYTES)?;
        }
        if self.coded_dimensions.is_some() {
            sink.commit_inline(OwnerKey::CodedDimensions, 8)?;
        }
        if self.render_dimensions.is_some() {
            sink.commit_inline(OwnerKey::RenderDimensions, 8)?;
        }
        Ok(Self {
            source_color,
            tags: Metadata::new(),
            crop: self.crop,
            rotation: self.rotation,
            mirror_horizontal: self.mirror_horizontal,
            mirror_vertical: self.mirror_vertical,
            pixel_aspect_ratio: self.pixel_aspect_ratio,
            clean_aperture: self.clean_aperture,
            coded_geometry,
            render_geometry,
            av1_description: self.av1_description,
            pixel_information,
            coded_dimensions: self.coded_dimensions,
            render_dimensions: self.render_dimensions,
            sequence_information: self.sequence_information,
            // ICC has a separate typed colour authority and does not reuse a
            // record produced by the CICP/linear execution path.
            last_conversion: None,
        })
    }

    pub(crate) fn metadata_bytes(&self) -> Result<usize, HighresError> {
        let mut total = self.source_color.owned_bytes()?;
        total = total
            .checked_add(
                self.tags
                    .capacity()
                    .checked_mul(size_of::<(String, crate::metadata::DataMap)>())
                    .ok_or_else(|| {
                        HighresError::InvalidMetadata("metadata size overflows".into())
                    })?,
            )
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        for (key, value) in &self.tags {
            total = total
                .checked_add(key.capacity())
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
            let size = match value {
                crate::metadata::DataMap::Raw(v) | crate::metadata::DataMap::ICCProfile(v) => {
                    v.capacity()
                }
                crate::metadata::DataMap::Ascii(v)
                | crate::metadata::DataMap::JSON(v)
                | crate::metadata::DataMap::I18NString(v) => v.capacity(),
                crate::metadata::DataMap::SJISString(v) => v.capacity(),
                crate::metadata::DataMap::UIntAllay(v) => {
                    v.capacity().checked_mul(size_of::<u64>()).ok_or_else(|| {
                        HighresError::InvalidMetadata("metadata size overflows".into())
                    })?
                }
                crate::metadata::DataMap::SIntAllay(v) => {
                    v.capacity().checked_mul(size_of::<i64>()).ok_or_else(|| {
                        HighresError::InvalidMetadata("metadata size overflows".into())
                    })?
                }
                crate::metadata::DataMap::FloatAllay(v) => {
                    v.capacity().checked_mul(size_of::<f64>()).ok_or_else(|| {
                        HighresError::InvalidMetadata("metadata size overflows".into())
                    })?
                }
                crate::metadata::DataMap::UInt(_)
                | crate::metadata::DataMap::SInt(_)
                | crate::metadata::DataMap::Float(_) => 8,
                crate::metadata::DataMap::None => 0,
                #[cfg(feature = "exif")]
                crate::metadata::DataMap::Exif(_) => {
                    return Err(HighresError::Unsupported(
                        "EXIF metadata sizing requires a checked representation".into(),
                    ));
                }
            };
            total = total
                .checked_add(size)
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        }
        let coded_geometry_bytes = self
            .coded_geometry
            .capacity()
            .checked_mul(size_of::<GeometryOperation>())
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        let render_geometry_bytes = self
            .render_geometry
            .capacity()
            .checked_mul(size_of::<GeometryOperation>())
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        total = total
            .checked_add(coded_geometry_bytes)
            .and_then(|value| value.checked_add(render_geometry_bytes))
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        for dimensions in [self.coded_dimensions, self.render_dimensions] {
            if dimensions.is_some() {
                total = total.checked_add(8).ok_or_else(|| {
                    HighresError::InvalidMetadata("metadata size overflows".into())
                })?;
            }
        }
        if let Some(pixel_information) = &self.pixel_information {
            let extended_bytes =
                pixel_information
                    .extended_channels
                    .as_ref()
                    .map_or(Ok(0usize), |channels| {
                        channels
                            .capacity()
                            .checked_mul(size_of::<PixelChannelInformation>())
                            .ok_or_else(|| {
                                HighresError::InvalidMetadata("metadata size overflows".into())
                            })
                    })?;
            total = total
                .checked_add(pixel_information.bits_per_channel.capacity())
                .and_then(|value| value.checked_add(extended_bytes))
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        }
        Ok(total)
    }

    pub(crate) fn fresh_clone_bytes(&self) -> Result<usize, HighresError> {
        let mut total = self.source_color.fresh_clone_owned_bytes()?;
        for geometry in [&self.coded_geometry, &self.render_geometry] {
            total = total
                .checked_add(
                    geometry
                        .len()
                        .checked_mul(size_of::<GeometryOperation>())
                        .ok_or_else(|| {
                            HighresError::InvalidMetadata("metadata size overflows".into())
                        })?,
                )
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        }
        if let Some(pixel_information) = &self.pixel_information {
            let extended_bytes =
                pixel_information
                    .extended_channels
                    .as_ref()
                    .map_or(Ok(0usize), |channels| {
                        channels
                            .len()
                            .checked_mul(size_of::<PixelChannelInformation>())
                            .ok_or_else(|| {
                                HighresError::InvalidMetadata("metadata size overflows".into())
                            })
                    })?;
            total = total
                .checked_add(pixel_information.bits_per_channel.len())
                .and_then(|value| value.checked_add(extended_bytes))
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        }
        Ok(total)
    }

    pub(crate) fn fresh_clone_owned_bytes_without_icc(&self) -> Result<usize, HighresError> {
        let with_icc = self.source_color.fresh_clone_owned_bytes()?;
        let without_icc = self.source_color.fresh_clone_owned_bytes_without_icc()?;
        let rest = self
            .fresh_clone_bytes()?
            .checked_sub(with_icc)
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size underflows".into()))?;
        rest.checked_add(without_icc)
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))
    }

    pub(crate) fn fresh_clone_retained_bytes(&self) -> Result<usize, HighresError> {
        let mut total = self.source_color.fresh_clone_retained_bytes()?;
        let coded_geometry_bytes = self
            .coded_geometry
            .len()
            .checked_mul(size_of::<GeometryOperation>())
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        let render_geometry_bytes = self
            .render_geometry
            .len()
            .checked_mul(size_of::<GeometryOperation>())
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        total = total
            .checked_add(coded_geometry_bytes)
            .and_then(|value| value.checked_add(render_geometry_bytes))
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        for dimensions in [self.coded_dimensions, self.render_dimensions] {
            if dimensions.is_some() {
                total = total.checked_add(8).ok_or_else(|| {
                    HighresError::InvalidMetadata("metadata size overflows".into())
                })?;
            }
        }
        if let Some(pixel_information) = &self.pixel_information {
            let extended_bytes =
                pixel_information
                    .extended_channels
                    .as_ref()
                    .map_or(Ok(0usize), |channels| {
                        channels
                            .len()
                            .checked_mul(size_of::<PixelChannelInformation>())
                            .ok_or_else(|| {
                                HighresError::InvalidMetadata("metadata size overflows".into())
                            })
                    })?;
            total = total
                .checked_add(pixel_information.bits_per_channel.len())
                .and_then(|value| value.checked_add(extended_bytes))
                .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))?;
        }
        Ok(total)
    }

    pub(crate) fn fresh_clone_retained_bytes_without_icc(&self) -> Result<usize, HighresError> {
        let with_icc = self.source_color.fresh_clone_retained_bytes()?;
        let without_icc = self.source_color.fresh_clone_retained_bytes_without_icc()?;
        let rest = self
            .fresh_clone_retained_bytes()?
            .checked_sub(with_icc)
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size underflows".into()))?;
        rest.checked_add(without_icc)
            .ok_or_else(|| HighresError::InvalidMetadata("metadata size overflows".into()))
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
        for (dimension_width, dimension_height) in [self.coded_dimensions, self.render_dimensions]
            .into_iter()
            .flatten()
        {
            if dimension_width == 0 || dimension_height == 0 {
                return Err(HighresError::InvalidMetadata(
                    "geometry dimensions must be non-zero".into(),
                ));
            }
        }
        Ok(())
    }
}
