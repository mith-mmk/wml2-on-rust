//! Borrowed output ownership planning and admission for conversion.

use super::ResourceLimits;
use super::allocation::{ConstructionCheckpoint, ConstructionLedger};
use super::metadata::{
    ColorProvenance, FrameMetadata, GeometryOperation, PixelChannelInformation,
    UnknownColorInformation,
};
use super::processing::ProcessingError;
use super::types::{ImageFrame, Plane, PlaneDescriptor};
use std::mem::size_of;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerKey {
    Sample(usize),
    DescriptorOuter,
    PixelOuter,
    DescriptorLayout { plane: usize },
    Role { plane: usize },
    PixelLayout { plane: usize },
    CodedGeometry,
    RenderGeometry,
    PixiBits,
    PixiExtended,
    Provenance,
    IccProfile,
    IccSourceProfile,
    IccDestinationProfile,
    IccDestinationProvenance,
    UnknownOuter,
    UnknownPayload(usize),
    SourceNclx,
    SourceAv1,
    CodedDimensions,
    RenderDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerKind {
    U8,
    F32,
    Usize,
    ChannelRole,
    PlaneDescriptor,
    PlaneF32,
    Geometry,
    PixelChannel,
    Provenance,
    Unknown,
}

mod owner_element_sealed {
    pub trait Sealed {}
}

pub(crate) trait OwnerElement: owner_element_sealed::Sealed {
    const KIND: OwnerKind;
}

fn expected_kind(key: OwnerKey) -> Option<OwnerKind> {
    Some(match key {
        OwnerKey::Sample(_) => OwnerKind::F32,
        OwnerKey::DescriptorOuter => OwnerKind::PlaneDescriptor,
        OwnerKey::PixelOuter => OwnerKind::PlaneF32,
        OwnerKey::DescriptorLayout { .. } | OwnerKey::PixelLayout { .. } => OwnerKind::Usize,
        OwnerKey::Role { .. } => OwnerKind::ChannelRole,
        OwnerKey::CodedGeometry | OwnerKey::RenderGeometry => OwnerKind::Geometry,
        OwnerKey::PixiBits
        | OwnerKey::IccProfile
        | OwnerKey::IccSourceProfile
        | OwnerKey::IccDestinationProfile
        | OwnerKey::UnknownPayload(_) => OwnerKind::U8,
        OwnerKey::IccDestinationProvenance => OwnerKind::Provenance,
        OwnerKey::PixiExtended => OwnerKind::PixelChannel,
        OwnerKey::Provenance => OwnerKind::Provenance,
        OwnerKey::UnknownOuter => OwnerKind::Unknown,
        OwnerKey::SourceNclx
        | OwnerKey::SourceAv1
        | OwnerKey::CodedDimensions
        | OwnerKey::RenderDimensions => return None,
    })
}

fn is_inline_key(key: OwnerKey) -> bool {
    matches!(
        key,
        OwnerKey::SourceNclx
            | OwnerKey::SourceAv1
            | OwnerKey::CodedDimensions
            | OwnerKey::RenderDimensions
    )
}
impl OwnerElement for u8 {
    const KIND: OwnerKind = OwnerKind::U8;
}
impl owner_element_sealed::Sealed for u8 {}
impl OwnerElement for f32 {
    const KIND: OwnerKind = OwnerKind::F32;
}
impl owner_element_sealed::Sealed for f32 {}
impl OwnerElement for usize {
    const KIND: OwnerKind = OwnerKind::Usize;
}
impl owner_element_sealed::Sealed for usize {}
impl OwnerElement for super::types::ChannelRole {
    const KIND: OwnerKind = OwnerKind::ChannelRole;
}
impl owner_element_sealed::Sealed for super::types::ChannelRole {}
impl OwnerElement for PlaneDescriptor {
    const KIND: OwnerKind = OwnerKind::PlaneDescriptor;
}
impl owner_element_sealed::Sealed for PlaneDescriptor {}
impl OwnerElement for Plane<f32> {
    const KIND: OwnerKind = OwnerKind::PlaneF32;
}
impl owner_element_sealed::Sealed for Plane<f32> {}
impl OwnerElement for GeometryOperation {
    const KIND: OwnerKind = OwnerKind::Geometry;
}
impl owner_element_sealed::Sealed for GeometryOperation {}
impl OwnerElement for PixelChannelInformation {
    const KIND: OwnerKind = OwnerKind::PixelChannel;
}
impl owner_element_sealed::Sealed for PixelChannelInformation {}
impl OwnerElement for ColorProvenance {
    const KIND: OwnerKind = OwnerKind::Provenance;
}
impl owner_element_sealed::Sealed for ColorProvenance {}
impl OwnerElement for UnknownColorInformation {
    const KIND: OwnerKind = OwnerKind::Unknown;
}
impl owner_element_sealed::Sealed for UnknownColorInformation {}

pub(crate) trait CandidateMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError>;
}

pub(crate) trait OwnerSink {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError>;
    fn commit_inline(&mut self, key: OwnerKey, bytes: usize) -> Result<(), ProcessingError>;
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OutputOwnershipPlan {
    sample_owner_bytes: [usize; 4],
    sample_owner_count: usize,
    sample_count: usize,
    descriptor_outer_bytes: usize,
    pixel_outer_bytes: usize,
    layout_offset_bytes: usize,
    role_bytes: usize,
    metadata_bytes: usize,
    total_bytes: usize,
    metadata_owner_count: usize,
    omit_source_icc: bool,
    icc_source_bytes: usize,
    icc_destination_bytes: usize,
    icc_destination_provenance: bool,
}

impl OutputOwnershipPlan {
    pub(crate) fn inspect(
        source: &ImageFrame,
        plane_count: usize,
        pixel_count: usize,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        Self::inspect_with_icc_profiles(source, plane_count, pixel_count, 0, 0, false, limits)
    }

    pub(crate) fn inspect_with_icc_profiles(
        source: &ImageFrame,
        plane_count: usize,
        pixel_count: usize,
        source_profile_bytes: usize,
        destination_profile_bytes: usize,
        omit_source_icc: bool,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        // A converted destination may be Gray (one plane), RGB (three
        // planes), or either model with one independent alpha plane.  The
        // generic plan is also used by the ICC adapter, so rejecting Gray
        // here would make its preflight disagree with construction.
        if !(1..=4).contains(&plane_count)
            || plane_count > limits.max_planes
            || plane_count > limits.max_channels
        {
            return Err(ProcessingError::ResourceLimit(
                "conversion output plane/channel count exceeds limits".into(),
            ));
        }
        let per_plane_samples = pixel_count
            .checked_mul(size_of::<f32>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output plane size overflows".into()))?;
        if per_plane_samples > limits.max_plane_bytes {
            return Err(ProcessingError::ResourceLimit(
                "conversion output plane exceeds limits".into(),
            ));
        }
        let sample_bytes = per_plane_samples
            .checked_mul(plane_count)
            .ok_or_else(|| ProcessingError::ResourceLimit("output samples overflow".into()))?;
        let layout_offset_bytes = plane_count
            .checked_mul(2)
            .and_then(|count| count.checked_mul(size_of::<usize>()))
            .ok_or_else(|| ProcessingError::ResourceLimit("output layouts overflow".into()))?;
        let role_bytes = plane_count
            .checked_mul(size_of::<super::ChannelRole>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output roles overflow".into()))?;
        let descriptor_outer_bytes = plane_count
            .checked_mul(size_of::<super::PlaneDescriptor>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output descriptors overflow".into()))?;
        let pixel_outer_bytes = plane_count
            .checked_mul(size_of::<Plane<f32>>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output planes overflow".into()))?;
        if omit_source_icc && source_profile_bytes == 0 {
            return Err(ProcessingError::ResourceLimit(
                "ICC source profile owner is missing from the output plan".into(),
            ));
        }
        let metadata_bytes = if omit_source_icc {
            source
                .metadata()
                .fresh_clone_retained_bytes_without_icc()
                .map_err(ProcessingError::from)?
        } else {
            source
                .metadata()
                .fresh_clone_retained_bytes()
                .map_err(ProcessingError::from)?
        };
        let metadata_bytes = metadata_bytes
            .checked_add(source_profile_bytes)
            .and_then(|value| value.checked_add(destination_profile_bytes))
            .and_then(|value| {
                value.checked_add(if omit_source_icc && source_profile_bytes != 0 {
                    size_of::<ColorProvenance>()
                } else {
                    0
                })
            })
            .and_then(|value| {
                value.checked_add(if destination_profile_bytes != 0 {
                    size_of::<ColorProvenance>()
                } else {
                    0
                })
            })
            .ok_or_else(|| ProcessingError::ResourceLimit("output metadata overflows".into()))?;
        if metadata_bytes > limits.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "conversion output metadata exceeds the configured budget".into(),
            ));
        }
        let metadata_heap_bytes = if omit_source_icc {
            source
                .metadata()
                .fresh_clone_owned_bytes_without_icc()
                .map_err(ProcessingError::from)?
        } else {
            source
                .metadata()
                .fresh_clone_bytes()
                .map_err(ProcessingError::from)?
        };
        if metadata_heap_bytes > metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "fresh metadata ownership exceeds retained metadata size".into(),
            ));
        }
        let total_bytes = metadata_bytes
            .checked_add(sample_bytes)
            .and_then(|value| value.checked_add(layout_offset_bytes))
            .and_then(|value| value.checked_add(role_bytes))
            .and_then(|value| value.checked_add(descriptor_outer_bytes))
            .and_then(|value| value.checked_add(pixel_outer_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("output ownership overflows".into()))?;
        if total_bytes > limits.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "conversion output exceeds the configured ownership limits".into(),
            ));
        }
        let mut sample_owner_bytes = [0; 4];
        sample_owner_bytes[..plane_count].fill(per_plane_samples);
        let metadata_owner_count = metadata_owner_count(
            source.metadata(),
            omit_source_icc,
            source_profile_bytes != 0,
            destination_profile_bytes != 0,
        )?;
        Ok(Self {
            sample_owner_bytes,
            sample_owner_count: plane_count,
            sample_count: pixel_count,
            descriptor_outer_bytes,
            pixel_outer_bytes,
            layout_offset_bytes,
            role_bytes,
            metadata_bytes,
            total_bytes,
            metadata_owner_count,
            omit_source_icc,
            icc_source_bytes: source_profile_bytes,
            icc_destination_bytes: destination_profile_bytes,
            icc_destination_provenance: destination_profile_bytes != 0,
        })
    }

    pub(crate) fn admit(
        self,
        mut ledger: ConstructionLedger,
        metadata: &FrameMetadata,
    ) -> Result<AdmittedOutputPlan<'_>, ProcessingError> {
        self.planned_output_bytes()?;
        let base_frame_bytes = ledger.frame_bytes;
        let base_live_bytes = ledger.live_bytes;
        let base_metadata_bytes = ledger.metadata_bytes_for_test();
        ledger.reserve_output(self.total_bytes, self.metadata_bytes)?;
        Ok(AdmittedOutputPlan {
            plan: self,
            ledger,
            owner_cursor: 0,
            metadata,
            base_frame_bytes,
            base_live_bytes,
            base_metadata_bytes,
        })
    }

    pub(crate) fn planned_output_bytes(&self) -> Result<usize, ProcessingError> {
        let sample_bytes = self.sample_owner_bytes[..self.sample_owner_count]
            .iter()
            .try_fold(0usize, |total, bytes| total.checked_add(*bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("output samples overflow".into()))?;
        let non_sample_bytes = self
            .layout_offset_bytes
            .checked_add(self.role_bytes)
            .and_then(|value| value.checked_add(self.descriptor_outer_bytes))
            .and_then(|value| value.checked_add(self.pixel_outer_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("output ownership overflows".into()))?;
        let expected_total = self
            .metadata_bytes
            .checked_add(sample_bytes)
            .and_then(|value| value.checked_add(non_sample_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("output ownership overflows".into()))?;
        if expected_total != self.total_bytes {
            return Err(ProcessingError::ResourceLimit(
                "output ownership plan is internally inconsistent".into(),
            ));
        }
        Ok(self.total_bytes)
    }

    pub(crate) fn planned_metadata_bytes(&self) -> usize {
        self.metadata_bytes
    }

    fn owner_count(&self) -> usize {
        self.sample_owner_count + 2 + self.sample_owner_count * 3 + self.metadata_owner_count
    }

    #[cfg(test)]
    fn expected_owner_bytes(&self, cursor: usize, metadata: &FrameMetadata) -> Option<usize> {
        if cursor < self.sample_owner_count {
            return Some(self.sample_owner_bytes[cursor]);
        }
        let descriptor_cursor = self.sample_owner_count;
        if cursor == descriptor_cursor {
            return Some(self.descriptor_outer_bytes);
        }
        if cursor == descriptor_cursor + 1 {
            return Some(self.pixel_outer_bytes);
        }
        let nested_cursor = cursor.checked_sub(descriptor_cursor + 2)?;
        let nested_count = self.sample_owner_count.checked_mul(3)?;
        if nested_cursor < nested_count {
            let slot = nested_cursor % 3;
            return Some(match slot {
                0 | 2 => size_of::<usize>(),
                1 => size_of::<super::ChannelRole>(),
                _ => return None,
            });
        }
        metadata_event(
            metadata,
            nested_cursor.checked_sub(nested_count)?,
            false,
            self.omit_source_icc,
            self.icc_source_bytes,
            self.icc_destination_bytes,
            self.icc_destination_provenance,
        )
        .map(|event| event.1)
    }

    fn expected_owner(
        &self,
        cursor: usize,
        metadata: &FrameMetadata,
    ) -> Option<(OwnerKey, usize, usize)> {
        if cursor < self.sample_owner_count {
            return Some((
                OwnerKey::Sample(cursor),
                self.sample_owner_bytes[cursor],
                self.sample_count,
            ));
        }
        let descriptor_cursor = self.sample_owner_count;
        if cursor == descriptor_cursor {
            return Some((
                OwnerKey::DescriptorOuter,
                self.descriptor_outer_bytes,
                self.sample_owner_count,
            ));
        }
        if cursor == descriptor_cursor + 1 {
            return Some((
                OwnerKey::PixelOuter,
                self.pixel_outer_bytes,
                self.sample_owner_count,
            ));
        }
        let nested_cursor = cursor.checked_sub(descriptor_cursor + 2)?;
        let nested_count = self.sample_owner_count.checked_mul(3)?;
        if nested_cursor < nested_count {
            let plane = nested_cursor / 3;
            return Some((
                match nested_cursor % 3 {
                    0 => OwnerKey::DescriptorLayout { plane },
                    1 => OwnerKey::Role { plane },
                    2 => OwnerKey::PixelLayout { plane },
                    _ => return None,
                },
                match nested_cursor % 3 {
                    1 => size_of::<super::ChannelRole>(),
                    _ => size_of::<usize>(),
                },
                1,
            ));
        }
        metadata_event(
            metadata,
            nested_cursor.checked_sub(nested_count)?,
            true,
            self.omit_source_icc,
            self.icc_source_bytes,
            self.icc_destination_bytes,
            self.icc_destination_provenance,
        )
    }
}

fn metadata_owner_count(
    metadata: &FrameMetadata,
    omit_source_icc: bool,
    include_icc_source: bool,
    include_icc_destination: bool,
) -> Result<usize, ProcessingError> {
    let mut count = 2usize; // coded/render geometry, including empty vectors
    if let Some(info) = metadata.pixel_information() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
        if info.extended_channels().is_some() {
            count = count.checked_add(1).ok_or_else(|| {
                ProcessingError::ResourceLimit("metadata owner count overflows".into())
            })?;
        }
    }
    count = count
        .checked_add(1)
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata owner count overflows".into()))?; // provenance
    if !omit_source_icc && metadata.source_color().icc_profile().is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    count = count
        .checked_add(1 + metadata.source_color().unknown_colr().len())
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata owner count overflows".into()))?;
    if metadata.source_color().nclx().is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if metadata.source_color().av1().is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if metadata.coded_dimensions().is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if metadata.render_dimensions().is_some() {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if include_icc_source {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if include_icc_destination {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    if include_icc_destination {
        count = count.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner count overflows".into())
        })?;
    }
    Ok(count)
}

fn metadata_event(
    metadata: &FrameMetadata,
    index: usize,
    with_count: bool,
    omit_source_icc: bool,
    icc_source_bytes: usize,
    icc_destination_bytes: usize,
    icc_destination_provenance: bool,
) -> Option<(OwnerKey, usize, usize)> {
    let mut cursor = 0usize;
    let geometry = [
        (
            OwnerKey::CodedGeometry,
            metadata.coded_geometry().len(),
            size_of::<GeometryOperation>(),
        ),
        (
            OwnerKey::RenderGeometry,
            metadata.render_geometry().len(),
            size_of::<GeometryOperation>(),
        ),
    ];
    for (key, count, element_size) in geometry {
        if cursor == index {
            return Some((
                key,
                count.checked_mul(element_size)?,
                if with_count { count } else { 0 },
            ));
        }
        cursor += 1;
    }
    if let Some(info) = metadata.pixel_information() {
        if cursor == index {
            let count = info.bits_per_channel().len();
            return Some((
                OwnerKey::PixiBits,
                count,
                if with_count { count } else { 0 },
            ));
        }
        cursor += 1;
        if let Some(channels) = info.extended_channels() {
            if cursor == index {
                let count = channels.len();
                return Some((
                    OwnerKey::PixiExtended,
                    count.checked_mul(size_of::<PixelChannelInformation>())?,
                    if with_count { count } else { 0 },
                ));
            }
            cursor += 1;
        }
    }
    let provenance = metadata.source_color().provenance();
    let provenance_count = provenance
        .len()
        .checked_add(usize::from(omit_source_icc && icc_source_bytes != 0))?;
    if cursor == index {
        return Some((
            OwnerKey::Provenance,
            provenance_count.checked_mul(size_of::<ColorProvenance>())?,
            if with_count { provenance_count } else { 0 },
        ));
    }
    cursor += 1;
    if !omit_source_icc {
        if let Some(profile) = metadata.source_color().icc_profile() {
            if cursor == index {
                return Some((
                    OwnerKey::IccProfile,
                    profile.len(),
                    if with_count { profile.len() } else { 0 },
                ));
            }
            cursor += 1;
        }
    }
    let unknown = metadata.source_color().unknown_colr();
    if cursor == index {
        return Some((
            OwnerKey::UnknownOuter,
            unknown
                .len()
                .checked_mul(size_of::<UnknownColorInformation>())?,
            if with_count { unknown.len() } else { 0 },
        ));
    }
    cursor += 1;
    if index >= cursor {
        let payload_index = index - cursor;
        if let Some(color) = unknown.get(payload_index) {
            return Some((
                OwnerKey::UnknownPayload(payload_index),
                color.payload.len(),
                if with_count { color.payload.len() } else { 0 },
            ));
        }
        cursor = cursor.checked_add(unknown.len())?;
    }
    if metadata.source_color().nclx().is_some() {
        if cursor == index {
            return Some((OwnerKey::SourceNclx, 8, 0));
        }
        cursor += 1;
    }
    if metadata.source_color().av1().is_some() {
        if cursor == index {
            return Some((
                OwnerKey::SourceAv1,
                super::metadata::AV1_COLOR_INFORMATION_BYTES,
                0,
            ));
        }
        cursor += 1;
    }
    for (key, present) in [
        (OwnerKey::CodedDimensions, metadata.coded_dimensions()),
        (OwnerKey::RenderDimensions, metadata.render_dimensions()),
    ] {
        if present.is_some() {
            if cursor == index {
                return Some((key, 8, 0));
            }
            cursor += 1;
        }
    }
    for (key, bytes) in [
        (OwnerKey::IccSourceProfile, icc_source_bytes),
        (OwnerKey::IccDestinationProfile, icc_destination_bytes),
    ] {
        if bytes != 0 {
            if cursor == index {
                return Some((key, bytes, if with_count { bytes } else { 0 }));
            }
            cursor += 1;
        }
    }
    if icc_destination_provenance {
        if cursor == index {
            return Some((
                OwnerKey::IccDestinationProvenance,
                size_of::<ColorProvenance>(),
                if with_count { 1 } else { 0 },
            ));
        }
    }
    None
}

pub(crate) struct AdmittedOutputPlan<'a> {
    plan: OutputOwnershipPlan,
    ledger: ConstructionLedger,
    owner_cursor: usize,
    metadata: &'a FrameMetadata,
    base_frame_bytes: usize,
    base_live_bytes: usize,
    base_metadata_bytes: usize,
}

impl<'a> AdmittedOutputPlan<'a> {
    pub(crate) fn fresh_with<T: OwnerElement, M>(
        &mut self,
        key: OwnerKey,
        count: usize,
        maker: &mut M,
    ) -> Result<Vec<T>, ProcessingError>
    where
        M: CandidateMaker,
    {
        let next_cursor = self.claim_keyed_owner::<T>(key, count)?;
        let result = self
            .ledger
            .try_new_reserved_vec_with(count, |count| maker.make(key, count));
        if result.is_ok() {
            self.owner_cursor = next_cursor;
        }
        result
    }

    pub(crate) fn fresh_with_limit<T: OwnerElement, M>(
        &mut self,
        key: OwnerKey,
        count: usize,
        owner_limit: usize,
        maker: &mut M,
    ) -> Result<Vec<T>, ProcessingError>
    where
        M: CandidateMaker,
    {
        let next_cursor = self.claim_keyed_owner::<T>(key, count)?;
        let result = self
            .ledger
            .try_new_reserved_vec_with_limit(count, owner_limit, |count| maker.make(key, count));
        if result.is_ok() {
            self.owner_cursor = next_cursor;
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn try_new_vec<T: OwnerElement>(
        &mut self,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let next_cursor = self.claim_owner::<T>(count)?;
        let result = self.ledger.try_new_vec(count);
        if result.is_ok() {
            self.owner_cursor = next_cursor;
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn try_new_vec_with<T: OwnerElement, F>(
        &mut self,
        count: usize,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        let next_cursor = self.claim_owner::<T>(count)?;
        let result = self.ledger.try_new_reserved_vec_with(count, make_candidate);
        if result.is_ok() {
            self.owner_cursor = next_cursor;
        }
        result
    }

    pub(crate) fn fresh_metadata<T: OwnerElement, M: CandidateMaker>(
        &mut self,
        key: OwnerKey,
        count: usize,
        maker: &mut M,
    ) -> Result<Vec<T>, ProcessingError> {
        let next_cursor = self.claim_keyed_owner::<T>(key, count)?;
        // All ICC owners are subject to the same per-profile ceiling.  The
        // conversion adapter has distinct keys for the source and
        // destination profile so that provenance remains observable, but
        // that distinction must not create an unbounded allocation path.
        let owner_limit = matches!(
            key,
            OwnerKey::IccProfile | OwnerKey::IccSourceProfile | OwnerKey::IccDestinationProfile
        )
        .then(|| self.ledger.max_icc_bytes());
        let result = self
            .ledger
            .try_new_metadata_vec_with_limit(count, owner_limit, |count| maker.make(key, count));
        if result.is_ok() {
            self.owner_cursor = next_cursor;
        }
        result
    }

    pub(crate) fn commit_metadata_event(
        &mut self,
        key: OwnerKey,
        bytes: usize,
    ) -> Result<(), ProcessingError> {
        if !is_inline_key(key) {
            return Err(ProcessingError::ResourceLimit(
                "heap metadata owner cannot be committed as inline".into(),
            ));
        }
        let next_cursor = self.owner_cursor.checked_add(1).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata owner cursor overflows".into())
        })?;
        if self.plan.expected_owner(self.owner_cursor, self.metadata) != Some((key, bytes, 0)) {
            return Err(ProcessingError::ResourceLimit(
                "metadata inline owner does not match admitted plan".into(),
            ));
        }
        self.ledger.commit_pending_inline(bytes, bytes)?;
        self.owner_cursor = next_cursor;
        Ok(())
    }

    #[cfg(test)]
    fn claim_owner<T>(&self, count: usize) -> Result<usize, ProcessingError> {
        let bytes = count
            .checked_mul(size_of::<T>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output owner bytes overflow".into()))?;
        self.claim_owner_bytes(bytes)
    }

    fn claim_keyed_owner<T: OwnerElement>(
        &self,
        key: OwnerKey,
        count: usize,
    ) -> Result<usize, ProcessingError> {
        if expected_kind(key) != Some(T::KIND) {
            return Err(ProcessingError::ResourceLimit(
                "output owner element kind does not match admitted plan".into(),
            ));
        }
        let bytes = count
            .checked_mul(size_of::<T>())
            .ok_or_else(|| ProcessingError::ResourceLimit("output owner bytes overflow".into()))?;
        if self.owner_cursor >= self.plan.owner_count()
            || self.plan.expected_owner(self.owner_cursor, self.metadata)
                != Some((key, bytes, count))
        {
            return Err(ProcessingError::ResourceLimit(
                "output owner key or count does not match admitted plan".into(),
            ));
        }
        self.owner_cursor
            .checked_add(1)
            .ok_or_else(|| ProcessingError::ResourceLimit("output owner cursor overflows".into()))
    }

    #[cfg(test)]
    fn claim_owner_bytes(&self, bytes: usize) -> Result<usize, ProcessingError> {
        if self.owner_cursor >= self.plan.owner_count() {
            return Err(ProcessingError::ResourceLimit(
                "output owner cursor exceeds admitted plan".into(),
            ));
        }
        if self
            .plan
            .expected_owner_bytes(self.owner_cursor, self.metadata)
            != Some(bytes)
        {
            return Err(ProcessingError::ResourceLimit(
                "output owner request does not match admitted plan".into(),
            ));
        }
        self.owner_cursor
            .checked_add(1)
            .ok_or_else(|| ProcessingError::ResourceLimit("output owner cursor overflows".into()))
    }

    pub(crate) fn checkpoint(&self) -> AdmittedOutputCheckpoint {
        AdmittedOutputCheckpoint {
            ledger: self.ledger.checkpoint(),
            owner_cursor: self.owner_cursor,
        }
    }

    pub(crate) fn restore(&mut self, checkpoint: AdmittedOutputCheckpoint) {
        self.ledger.restore(checkpoint.ledger);
        self.owner_cursor = checkpoint.owner_cursor;
    }

    pub(crate) fn pending_bytes(&self) -> (usize, usize) {
        (
            self.ledger.pending_frame_bytes(),
            self.ledger.pending_metadata_bytes(),
        )
    }

    pub(crate) fn complete_for(&self, frame: &ImageFrame) -> Result<(), ProcessingError> {
        if self.pending_bytes() != (0, 0) || self.owner_cursor != self.plan.owner_count() {
            return Err(ProcessingError::ResourceLimit(
                "admitted output owners remain pending".into(),
            ));
        }
        let descriptor_bytes = frame
            .descriptor()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let pixel_bytes = frame
            .pixels()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let frame_metadata_bytes = frame
            .metadata()
            .metadata_bytes()
            .map_err(ProcessingError::from)?;
        // The destination ICC payload and its provenance are owned by the
        // descriptor's ColorInformationSet, but they are admitted through
        // the metadata ledger (after the frame metadata clone).  Keep the
        // reconciliation in the same ownership domain: comparing the
        // ledger's metadata debit only with FrameMetadata would make every
        // ICC conversion appear to have an unaccounted metadata surplus.
        let descriptor_color_bytes = frame
            .descriptor()
            .color_information()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        // `ImageDescriptor::owned_bytes` includes its ColorInformationSet,
        // while the output plan admits that set as a metadata owner.  Split
        // the descriptor accounting at this boundary before composing the
        // actual total; otherwise ICC output is charged twice (once through
        // the descriptor and once through the metadata ledger).
        let descriptor_structure_bytes = descriptor_bytes
            .checked_sub(descriptor_color_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("output descriptor ownership underflows".into())
            })?;
        let metadata_bytes = frame_metadata_bytes
            .checked_add(descriptor_color_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("output metadata overflows".into()))?;
        let actual_total = descriptor_structure_bytes
            .checked_add(pixel_bytes)
            .and_then(|bytes| bytes.checked_add(metadata_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("output ownership overflows".into()))?;
        let actual_frame = self
            .ledger
            .frame_bytes
            .checked_sub(self.base_frame_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("output frame ledger underflows".into())
            })?;
        let actual_live = self
            .ledger
            .live_bytes
            .checked_sub(self.base_live_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("output live ledger underflows".into())
            })?;
        let actual_metadata = self
            .ledger
            .metadata_bytes_for_test()
            .checked_sub(self.base_metadata_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("output metadata ledger underflows".into())
            })?;
        if actual_frame != actual_total
            || actual_live != actual_total
            || actual_metadata != metadata_bytes
        {
            return Err(ProcessingError::ResourceLimit(
                "materialized output ledger differs from owned bytes".into(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn materialize_with<T, F>(&mut self, make: F) -> Result<T, ProcessingError>
    where
        F: FnOnce(&mut Self) -> Result<T, ProcessingError>,
    {
        let checkpoint = self.checkpoint();
        let planned_output = self.plan.planned_output_bytes()?;
        match make(self) {
            Ok(value)
                if self.pending_bytes() == (0, 0)
                    && self.owner_cursor == self.plan.owner_count()
                    && self.ledger.frame_bytes >= planned_output =>
            {
                Ok(value)
            }
            Ok(value) => {
                drop(value);
                self.restore(checkpoint);
                Err(ProcessingError::ResourceLimit(
                    "admitted output owners remain pending".into(),
                ))
            }
            Err(error) => {
                self.restore(checkpoint);
                Err(error)
            }
        }
    }
}

pub(crate) struct AdmittedOutputCheckpoint {
    ledger: ConstructionCheckpoint,
    owner_cursor: usize,
}

#[cfg(test)]
#[path = "output_plan_tests.rs"]
pub(crate) mod tests;
