use std::mem::size_of;

use avif_codec::{DecodedFrame, RichAvifInfo};

use crate::highres::{
    AlphaAssociation, ChannelModel, ChannelRole, ColorInformationSet, FrameTiming, HighresError,
    ImageDescriptor, ImageFrame, PixelBuffer, Plane, PlaneDescriptor, PlaneLayout, ProcessingError,
    ResourceLimits, Subsampling,
};

use super::allocation::ConstructionLedger;
#[cfg(test)]
pub(super) use super::allocation::construction_failpoint;
#[cfg(test)]
#[allow(unused_imports)]
use super::metadata::frame_metadata;
use super::metadata::{
    av1_description, frame_metadata_with_ledger, metadata_borrowed_bytes,
    metadata_final_owned_bytes, projected_color_bytes,
};

#[derive(Clone, Copy)]
struct NativePlanePlan {
    id: u8,
    role: ChannelRole,
    width: u32,
    height: u32,
    subsampling: Subsampling,
}

#[derive(Clone, Copy)]
struct NativeMapPlan {
    width: u32,
    height: u32,
    bit_depth: u8,
    monochrome: bool,
    identity: bool,
    has_alpha: bool,
    plane_count: usize,
    planes: [Option<NativePlanePlan>; 4],
    sample_bytes: usize,
    final_frame_bytes: usize,
    live_peak_bytes: usize,
}

#[derive(Clone, Copy)]
struct NativeOwnershipPlan {
    pre_release_live_bytes: usize,
    post_release_live_bytes: usize,
}

fn late_dimension_bytes(
    coded_present: bool,
    render_present: bool,
) -> Result<usize, ProcessingError> {
    let late_count = usize::from(!coded_present)
        .checked_add(usize::from(!render_present))
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata dimensions overflow".into()))?;
    late_count
        .checked_mul(size_of::<(u32, u32)>())
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata dimensions overflow".into()))
}

impl NativeOwnershipPlan {
    fn new(
        final_frame_bytes: usize,
        active_fresh_bytes: usize,
        late_dimensions_bytes: usize,
        borrowed_live_bytes: usize,
        native_outer_bytes: usize,
    ) -> Result<Self, ProcessingError> {
        let pre_release_live_bytes = final_frame_bytes
            .checked_sub(active_fresh_bytes)
            .and_then(|value| value.checked_sub(late_dimensions_bytes))
            .and_then(|value| value.checked_add(borrowed_live_bytes))
            .and_then(|value| value.checked_add(native_outer_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("native live bytes overflow".into()))?;
        let post_release_live_bytes = final_frame_bytes
            .checked_add(borrowed_live_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("native live bytes overflow".into()))?;
        Ok(Self {
            pre_release_live_bytes,
            post_release_live_bytes,
        })
    }

    fn for_rich(
        frame: &DecodedFrame,
        rich: &RichAvifInfo,
        plan: NativeMapPlan,
    ) -> Result<Self, ProcessingError> {
        let borrowed_bytes = metadata_borrowed_bytes(rich)?;
        let native_outer_bytes = frame
            .buffers
            .planes
            .capacity()
            .checked_mul(size_of::<avif_codec::av1::PlaneBuffer>())
            .ok_or_else(|| ProcessingError::ResourceLimit("native outer bytes overflow".into()))?;
        let active_color_bytes =
            projected_color_bytes(rich, frame.color_config.color_description.is_some())?;
        Self::new(
            plan.final_frame_bytes,
            active_color_bytes,
            late_dimension_bytes(false, false)?,
            borrowed_bytes,
            native_outer_bytes,
        )
    }

    fn check(&self, limits: &ResourceLimits) -> Result<(), ProcessingError> {
        if self
            .pre_release_live_bytes
            .max(self.post_release_live_bytes)
            > limits.max_total_live_decoded_bytes
        {
            return Err(ProcessingError::ResourceLimit(
                "native ownership phases exceed live limits".into(),
            ));
        }
        Ok(())
    }
}

impl NativeMapPlan {
    /// Validate every bridge-relevant property without allocating a metadata
    /// projection, descriptor, or destination plane.  The fixed-size plan is
    /// intentionally stack-only so all limit failures occur before copying
    /// owned codec data.
    fn inspect(
        frame: &DecodedFrame,
        rich: &RichAvifInfo,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        if rich.info.rotation.is_some() || rich.info.mirror.is_some() {
            return Err(ProcessingError::Unsupported(
                "ordered geometry metadata is unavailable in this bridge".into(),
            ));
        }
        validate_native_configuration(frame)?;
        validate_native_pixel_information(frame, rich)?;
        validate_icc_kinds(rich)?;
        let metadata_bytes = metadata_final_owned_bytes(rich)?;
        if metadata_bytes > limits.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "native metadata exceeds resource limits".into(),
            ));
        }
        let source_icc_capacity = rich
            .color_information
            .icc_profile
            .as_ref()
            .map_or(0, Vec::capacity);
        let projected_icc_capacity = rich
            .info
            .color_information
            .as_ref()
            .filter(|color| color.color_type == *b"prof" || color.color_type == *b"rICC")
            .map_or(0, |color| color.payload.capacity());
        if source_icc_capacity > limits.max_icc_bytes
            || projected_icc_capacity > limits.max_icc_bytes
        {
            return Err(ProcessingError::ResourceLimit(
                "native ICC profile exceeds resource limits".into(),
            ));
        }
        Self::inspect_frame_with_live_extra(
            frame,
            metadata_bytes,
            metadata_borrowed_bytes(rich)?,
            limits,
            true,
        )
        .and_then(|plan| {
            NativeOwnershipPlan::for_rich(frame, rich, plan)?.check(limits)?;
            Ok(plan)
        })
    }

    #[cfg(test)]
    fn inspect_borrowed_frame(
        frame: &DecodedFrame,
        metadata_bytes: usize,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        Self::inspect_frame_with_ownership(frame, metadata_bytes, limits, false)
    }

    #[cfg(test)]
    fn inspect_frame_with_ownership(
        frame: &DecodedFrame,
        metadata_bytes: usize,
        limits: &ResourceLimits,
        include_nested_ownership: bool,
    ) -> Result<Self, ProcessingError> {
        Self::inspect_frame_with_live_extra(
            frame,
            metadata_bytes,
            0,
            limits,
            include_nested_ownership,
        )
    }

    fn inspect_frame_with_live_extra(
        frame: &DecodedFrame,
        metadata_bytes: usize,
        live_extra_bytes: usize,
        limits: &ResourceLimits,
        include_nested_ownership: bool,
    ) -> Result<Self, ProcessingError> {
        validate_native_configuration(frame)?;
        let width = u32::try_from(frame.width)
            .map_err(|_| ProcessingError::ResourceLimit("native width exceeds u32".into()))?;
        let height = u32::try_from(frame.height)
            .map_err(|_| ProcessingError::ResourceLimit("native height exceeds u32".into()))?;
        if !matches!(frame.bit_depth, 8 | 10 | 12) {
            return Err(ProcessingError::Unsupported(
                "native AV1 bit depth requires derived-sample provenance for highres mapping"
                    .into(),
            ));
        }
        if frame.buffers.width != frame.width || frame.buffers.height != frame.height {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native buffer dimensions differ from frame".into(),
            )));
        }
        if frame.buffers.planes.is_empty() {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native frame has no planes".into(),
            )));
        }
        let pixels = frame
            .width
            .checked_mul(frame.height)
            .ok_or_else(|| ProcessingError::ResourceLimit("native pixel count overflows".into()))?;
        if width > limits.max_width || height > limits.max_height || pixels > limits.max_pixels {
            return Err(ProcessingError::ResourceLimit(
                "native frame exceeds dimension limits".into(),
            ));
        }
        let plane_count = frame.buffers.planes.len();
        if plane_count > limits.max_planes || plane_count > 4 {
            return Err(ProcessingError::ResourceLimit(
                "native plane count exceeds resource limits".into(),
            ));
        }
        let monochrome = frame.color_config.monochrome;
        let identity = frame
            .color_config
            .color_description
            .map(|description| description.matrix_coefficients == 0)
            .unwrap_or(false);
        let expected_channels = if monochrome { 1 } else { 3 };
        let mut plans = [None; 4];
        let mut seen = [false; 4];
        let mut has_alpha = false;
        let mut sample_bytes = 0usize;
        for plane in &frame.buffers.planes {
            let id = plane.layout.plane;
            if id > 3 {
                return Err(ProcessingError::Unsupported(
                    "native plane id is not supported".into(),
                ));
            }
            let index = usize::from(id);
            if seen[index] {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "duplicate native plane id".into(),
                )));
            }
            seen[index] = true;
            let role = match id {
                0 if monochrome => ChannelRole::Gray,
                0 if identity => ChannelRole::Green,
                1 if identity => ChannelRole::Blue,
                2 if identity => ChannelRole::Red,
                0 => ChannelRole::Y,
                1 => ChannelRole::Cb,
                2 => ChannelRole::Cr,
                3 => {
                    has_alpha = true;
                    ChannelRole::Alpha
                }
                _ => {
                    return Err(ProcessingError::Unsupported(
                        "native plane id is not supported".into(),
                    ));
                }
            };
            let (expected_x, expected_y) = match id {
                0 => (0, 0),
                1 | 2 if !monochrome => (
                    u8::from(frame.color_config.subsampling_x),
                    u8::from(frame.color_config.subsampling_y),
                ),
                3 => (0, 0),
                _ => (0, 0),
            };
            if plane.layout.subsampling_x != expected_x || plane.layout.subsampling_y != expected_y
            {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "native plane subsampling disagrees with AV1 configuration".into(),
                )));
            }
            let x_factor = 1usize << expected_x;
            let y_factor = 1usize << expected_y;
            let expected_width = frame.width.checked_add(x_factor - 1).ok_or_else(|| {
                ProcessingError::ResourceLimit("native plane width overflows".into())
            })? / x_factor;
            let expected_height = frame.height.checked_add(y_factor - 1).ok_or_else(|| {
                ProcessingError::ResourceLimit("native plane height overflows".into())
            })? / y_factor;
            if plane.layout.width != expected_width || plane.layout.height != expected_height {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "native plane dimensions disagree with subsampling".into(),
                )));
            }
            let expected_samples =
                expected_width.checked_mul(expected_height).ok_or_else(|| {
                    ProcessingError::ResourceLimit("native sample count overflows".into())
                })?;
            if plane.layout.sample_count != expected_samples
                || plane.samples.len() != expected_samples
            {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "native sample count disagrees with layout".into(),
                )));
            }
            let plane_bytes = plane
                .samples
                .capacity()
                .checked_mul(size_of::<u16>())
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("native plane bytes overflow".into())
                })?;
            if plane_bytes > limits.max_plane_bytes {
                return Err(ProcessingError::ResourceLimit(
                    "native plane bytes exceed resource limits".into(),
                ));
            }
            sample_bytes = sample_bytes.checked_add(plane_bytes).ok_or_else(|| {
                ProcessingError::ResourceLimit("native sample bytes overflow".into())
            })?;
            plans[index] = Some(NativePlanePlan {
                id,
                role,
                width: u32::try_from(plane.layout.width).map_err(|_| {
                    ProcessingError::ResourceLimit("native plane width exceeds u32".into())
                })?,
                height: u32::try_from(plane.layout.height).map_err(|_| {
                    ProcessingError::ResourceLimit("native plane height exceeds u32".into())
                })?,
                subsampling: Subsampling::new(
                    native_subsampling_factor(plane.layout.subsampling_x)?,
                    native_subsampling_factor(plane.layout.subsampling_y)?,
                )
                .map_err(ProcessingError::from)?,
            });
        }
        let color_planes = seen[..3].iter().filter(|present| **present).count();
        if color_planes != expected_channels {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native colour plane count disagrees with AV1 configuration".into(),
            )));
        }
        let channels = color_planes + usize::from(has_alpha);
        if channels > limits.max_channels {
            return Err(ProcessingError::ResourceLimit(
                "native channel count exceeds resource limits".into(),
            ));
        }
        let layout_bytes = plane_count
            .checked_mul(size_of::<usize>())
            .ok_or_else(|| ProcessingError::ResourceLimit("layout bytes overflow".into()))?;
        let role_bytes = plane_count
            .checked_mul(size_of::<ChannelRole>())
            .ok_or_else(|| ProcessingError::ResourceLimit("role bytes overflow".into()))?;
        let descriptor_nested_bytes = if include_nested_ownership {
            layout_bytes
                .checked_add(role_bytes)
                .ok_or_else(|| ProcessingError::ResourceLimit("descriptor bytes overflow".into()))?
        } else {
            0
        };
        let descriptor_bytes = size_of::<ImageDescriptor>()
            .checked_add(
                plane_count
                    .checked_mul(size_of::<PlaneDescriptor>())
                    .ok_or_else(|| {
                        ProcessingError::ResourceLimit("descriptor bytes overflow".into())
                    })?,
            )
            .and_then(|value| value.checked_add(descriptor_nested_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("descriptor bytes overflow".into()))?;
        let pixel_nested_bytes = if include_nested_ownership {
            layout_bytes
        } else {
            0
        };
        let pixel_bytes = size_of::<PixelBuffer>()
            .checked_add(
                size_of::<Plane<u16>>()
                    .checked_mul(plane_count)
                    .ok_or_else(|| {
                        ProcessingError::ResourceLimit("pixel descriptor bytes overflow".into())
                    })?,
            )
            .and_then(|value| value.checked_add(sample_bytes))
            .and_then(|value| value.checked_add(pixel_nested_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("pixel bytes overflow".into()))?;
        let header_bytes = if include_nested_ownership {
            size_of::<crate::highres::FrameMetadata>()
        } else {
            0
        };
        let final_frame_bytes = header_bytes
            .checked_add(descriptor_bytes)
            .and_then(|value| value.checked_add(pixel_bytes))
            .and_then(|value| value.checked_add(metadata_bytes))
            .ok_or_else(|| ProcessingError::ResourceLimit("native frame bytes overflow".into()))?;
        if final_frame_bytes > limits.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "native frame allocations exceed frame limits".into(),
            ));
        }
        let live_peak_bytes = final_frame_bytes
            .checked_add(live_extra_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("native live bytes overflow".into()))?;
        if live_peak_bytes > limits.max_total_live_decoded_bytes {
            return Err(ProcessingError::ResourceLimit(
                "native allocations exceed live limits".into(),
            ));
        }
        Ok(Self {
            width,
            height,
            bit_depth: frame.bit_depth,
            monochrome,
            identity,
            has_alpha,
            plane_count,
            planes: plans,
            sample_bytes,
            final_frame_bytes,
            live_peak_bytes,
        })
    }
}

#[cfg(test)]
pub(super) fn reserve_construction_for_test(
    limits: &ResourceLimits,
    count: usize,
) -> (Result<(), ProcessingError>, usize) {
    let mut ledger = match ConstructionLedger::new(0, 0, limits) {
        Ok(ledger) => ledger,
        Err(error) => return (Err(error), 0),
    };
    let result = ledger.try_new_vec::<u8>(count);
    let capacity = result.as_ref().map_or(0, Vec::capacity);
    (result.map(|_| ()), capacity)
}

#[cfg(test)]
pub(super) fn reserve_u16_construction_for_test(
    limits: &ResourceLimits,
    count: usize,
) -> (Result<(), ProcessingError>, usize) {
    let mut ledger = match ConstructionLedger::new(0, 0, limits) {
        Ok(ledger) => ledger,
        Err(error) => return (Err(error), 0),
    };
    let result = ledger.try_new_vec::<u16>(count);
    let capacity = result.as_ref().map_or(0, Vec::capacity);
    (result.map(|_| ()), capacity)
}

#[cfg(test)]
pub(super) fn inspect_native_frame_for_test(
    frame: &DecodedFrame,
    metadata_bytes: usize,
    limits: &ResourceLimits,
) -> Result<(usize, usize), ProcessingError> {
    NativeMapPlan::inspect_borrowed_frame(frame, metadata_bytes, limits)
        .map(|plan| (plan.final_frame_bytes, plan.live_peak_bytes))
}

#[cfg(test)]
pub(super) fn inspect_native_for_test(
    frame: &DecodedFrame,
    rich: &RichAvifInfo,
    limits: &ResourceLimits,
) -> Result<(), ProcessingError> {
    NativeMapPlan::inspect(frame, rich, limits).map(|_| ())
}

fn clone_color_information_with_ledger(
    source: &ColorInformationSet,
    ledger: &mut ConstructionLedger,
) -> Result<ColorInformationSet, ProcessingError> {
    let provenance = ledger.try_copy_metadata(source.provenance())?;
    let icc_profile = source
        .icc_profile()
        .map(|profile| ledger.try_copy_icc(profile))
        .transpose()?;
    let mut unknown_colr = ledger.try_new_metadata_vec(source.unknown_colr().len())?;
    for color in source.unknown_colr() {
        unknown_colr.push(crate::highres::UnknownColorInformation {
            color_type: color.color_type,
            payload: ledger.try_copy_metadata(&color.payload)?,
        });
    }
    Ok(ColorInformationSet::from_owned_parts(
        provenance,
        icc_profile,
        source.icc_color_type(),
        source.nclx(),
        source.av1(),
        unknown_colr,
    ))
}

pub(super) fn update_av1_metadata_with_ledger(
    metadata: &mut crate::highres::FrameMetadata,
    description: avif_codec::av1::ColorDescription,
    full_range: bool,
    ledger: &mut ConstructionLedger,
) -> Result<(), ProcessingError> {
    // The AV1 description is an inline 8-byte owner.  Its presence and the
    // provenance entry are independent states for metadata assembled through
    // the internal bridge, so account for each one separately.
    let inline_delta = usize::from(metadata.source_color().av1().is_none())
        .checked_mul(crate::highres::AV1_COLOR_INFORMATION_BYTES)
        .ok_or_else(|| ProcessingError::ResourceLimit("AV1 metadata bytes overflow".into()))?;
    let needs_provenance = !metadata
        .source_color()
        .provenance()
        .contains(&crate::highres::ColorProvenance::Av1);
    if needs_provenance || inline_delta != 0 {
        if needs_provenance {
            ledger.try_grow_metadata_vec(
                metadata.source_color_mut().provenance_mut_bridge(),
                1,
                inline_delta,
                super::allocation::try_new_metadata_candidate::<crate::highres::ColorProvenance>,
            )?;
        } else {
            ledger.charge_metadata(inline_delta)?;
        }
    }
    metadata
        .source_color_mut()
        .set_av1(crate::highres::Av1ColorInformation::new(
            u16::from(description.color_primaries),
            u16::from(description.transfer_characteristics),
            u16::from(description.matrix_coefficients),
            full_range,
        ));
    Ok(())
}

// The public bridge is intentionally deferred until the standalone strict
// decode gates (C1-C3). Keep the ownership adapter compiled and fixture-tested
// in this checkpoint without exposing an unbounded byte-decoding API.
#[allow(dead_code)]
pub(crate) fn consume_native_frame(
    frame: DecodedFrame,
    rich: &RichAvifInfo,
    limits: &ResourceLimits,
    timing: Option<FrameTiming>,
) -> Result<ImageFrame, ProcessingError> {
    let plan = NativeMapPlan::inspect(&frame, rich, limits)?;
    let borrowed_bytes = metadata_borrowed_bytes(rich)?;
    let native_outer_bytes = frame
        .buffers
        .planes
        .capacity()
        .checked_mul(size_of::<avif_codec::av1::PlaneBuffer>())
        .ok_or_else(|| ProcessingError::ResourceLimit("native outer bytes overflow".into()))?;
    let mut ledger = ConstructionLedger::new_with_ownership(
        0,
        0,
        borrowed_bytes
            .checked_add(native_outer_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("native live bytes overflow".into()))?,
        limits,
    )?;
    ledger.charge(plan.sample_bytes)?;
    let header_bytes = size_of::<crate::highres::FrameMetadata>()
        .checked_add(size_of::<ImageDescriptor>())
        .and_then(|value| value.checked_add(size_of::<PixelBuffer>()))
        .ok_or_else(|| {
            ProcessingError::ResourceLimit("construction header bytes overflow".into())
        })?;
    ledger.charge(header_bytes)?;
    let metadata = frame_metadata_with_ledger(rich, &mut ledger)?;
    consume_native_frame_with_ledger_state(
        frame,
        metadata,
        limits,
        timing,
        &mut ledger,
        NativeFrameLedgerState {
            samples_precharged: true,
            headers_precharged: true,
            live_only_bytes: borrowed_bytes,
        },
    )
}

#[cfg(test)]
pub(super) fn consume_native_frame_with_metadata(
    frame: DecodedFrame,
    metadata: crate::highres::FrameMetadata,
    limits: &ResourceLimits,
    timing: Option<FrameTiming>,
) -> Result<ImageFrame, ProcessingError> {
    let retained_bytes = metadata.metadata_bytes().map_err(ProcessingError::from)?;
    let native_outer_bytes = frame
        .buffers
        .planes
        .capacity()
        .checked_mul(size_of::<avif_codec::av1::PlaneBuffer>())
        .ok_or_else(|| ProcessingError::ResourceLimit("native outer bytes overflow".into()))?;
    let mut ledger = ConstructionLedger::new_with_ownership(
        retained_bytes,
        retained_bytes,
        native_outer_bytes,
        limits,
    )?;
    consume_native_frame_with_ledger_state(
        frame,
        metadata,
        limits,
        timing,
        &mut ledger,
        NativeFrameLedgerState {
            samples_precharged: false,
            headers_precharged: false,
            live_only_bytes: 0,
        },
    )
}

struct NativeFrameLedgerState {
    samples_precharged: bool,
    headers_precharged: bool,
    live_only_bytes: usize,
}

fn consume_native_frame_with_ledger_state(
    frame: DecodedFrame,
    mut metadata: crate::highres::FrameMetadata,
    limits: &ResourceLimits,
    timing: Option<FrameTiming>,
    ledger: &mut ConstructionLedger,
    ledger_state: NativeFrameLedgerState,
) -> Result<ImageFrame, ProcessingError> {
    let retained_bytes = metadata.metadata_bytes().map_err(ProcessingError::from)?;
    let native_outer_bytes = frame
        .buffers
        .planes
        .capacity()
        .checked_mul(size_of::<avif_codec::av1::PlaneBuffer>())
        .ok_or_else(|| ProcessingError::ResourceLimit("native outer bytes overflow".into()))?;
    let active_color_base_bytes = metadata
        .source_color()
        .fresh_owned_bytes()
        .map_err(ProcessingError::from)?;
    let active_icc_bytes = metadata.source_color().icc_profile().map_or(0, <[u8]>::len);
    if metadata.source_color().icc_profile_capacity() > limits.max_icc_bytes
        || active_icc_bytes > limits.max_icc_bytes
    {
        return Err(ProcessingError::ResourceLimit(
            "ICC profile exceeds resource limits".into(),
        ));
    }
    let av1_delta = if frame.color_config.color_description.is_some()
        && metadata.source_color().av1().is_none()
    {
        metadata
            .source_color()
            .av1_additional_bytes()
            .map_err(ProcessingError::from)?
    } else {
        0
    };
    let active_color_bytes = active_color_base_bytes
        .checked_add(av1_delta)
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata bytes overflow".into()))?;
    // The AV1 description uses the shared metadata size constant. A provenance
    // byte is only an additional owned allocation when the existing provenance
    // vector is already full; `av1_additional_bytes` is intentionally
    // conservative for a fresh clone, but must not overcharge the retained
    // metadata owner.
    let av1_metadata_delta = if av1_delta == 0 {
        0
    } else {
        crate::highres::AV1_COLOR_INFORMATION_BYTES
            .checked_add(usize::from(
                metadata.source_color().provenance().len()
                    == metadata.source_color().provenance_capacity(),
            ))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata bytes overflow".into()))?
    };
    let metadata_budget_bytes = retained_bytes
        .checked_add(av1_metadata_delta)
        .and_then(|value| value.checked_add(active_color_bytes))
        .and_then(|value| value.checked_add(2 * size_of::<(u32, u32)>()))
        .ok_or_else(|| ProcessingError::ResourceLimit("metadata bytes overflow".into()))?;
    if metadata_budget_bytes > limits.max_metadata_bytes {
        return Err(ProcessingError::ResourceLimit(
            "metadata ownership exceeds resource limits".into(),
        ));
    }
    let plan = NativeMapPlan::inspect_frame_with_live_extra(
        &frame,
        metadata_budget_bytes,
        ledger_state.live_only_bytes,
        limits,
        true,
    )?;
    let late_dimensions = late_dimension_bytes(
        metadata.coded_dimensions().is_some(),
        metadata.render_dimensions().is_some(),
    )?;
    NativeOwnershipPlan::new(
        plan.final_frame_bytes,
        active_color_bytes,
        late_dimensions,
        ledger_state.live_only_bytes,
        native_outer_bytes,
    )?
    .check(limits)?;
    if !ledger_state.samples_precharged {
        ledger.charge(plan.sample_bytes)?;
    }
    if !ledger_state.headers_precharged {
        ledger.charge(
            size_of::<crate::highres::FrameMetadata>()
                .checked_add(size_of::<ImageDescriptor>())
                .and_then(|value| value.checked_add(size_of::<PixelBuffer>()))
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("construction header bytes overflow".into())
                })?,
        )?;
    }
    let width = plan.width;
    let height = plan.height;
    let monochrome = plan.monochrome;
    let identity = plan.identity;
    debug_assert!(plan.final_frame_bytes <= limits.max_frame_bytes);
    debug_assert!(plan.live_peak_bytes <= limits.max_total_live_decoded_bytes);
    let av1_info = av1_description(&frame);
    if let Some(description) = frame.color_config.color_description {
        update_av1_metadata_with_ledger(
            &mut metadata,
            description,
            matches!(
                frame.color_config.color_range,
                avif_codec::av1::ColorRange::Full
            ),
            ledger,
        )?;
    }
    let mut native = frame.buffers.planes;
    native.sort_by_key(|plane| plane.layout.plane);
    let mut descriptors = ledger.try_new_vec::<PlaneDescriptor>(plan.plane_count)?;
    let mut planes = ledger.try_new_vec::<Plane<u16>>(plan.plane_count)?;
    for native_plane in native {
        let id = native_plane.layout.plane;
        let planned = plan.planes[usize::from(id)].ok_or_else(|| {
            ProcessingError::Invalid(HighresError::InvalidLayout(
                "native plane was absent from its checked map plan".into(),
            ))
        })?;
        if planned.id != id {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native plane id disagrees with its checked map plan".into(),
            )));
        }
        let row_stride = usize::try_from(planned.width)
            .map_err(|_| ProcessingError::ResourceLimit("plane width exceeds usize".into()))?;
        let mut descriptor_offsets = ledger.try_new_vec::<usize>(1)?;
        descriptor_offsets.push(0);
        let descriptor_layout = PlaneLayout::new(
            planned.width,
            planned.height,
            row_stride,
            1,
            descriptor_offsets,
            planned.subsampling,
        )
        .map_err(ProcessingError::from)?;
        let mut roles = ledger.try_new_vec::<ChannelRole>(1)?;
        roles.push(planned.role);
        let descriptor = PlaneDescriptor::new(descriptor_layout, roles, plan.bit_depth)
            .map_err(ProcessingError::from)?;
        let mut plane_offsets = ledger.try_new_vec::<usize>(1)?;
        plane_offsets.push(0);
        let plane_layout = PlaneLayout::new(
            planned.width,
            planned.height,
            row_stride,
            1,
            plane_offsets,
            planned.subsampling,
        )
        .map_err(ProcessingError::from)?;
        let plane =
            Plane::new(plane_layout, native_plane.samples).map_err(ProcessingError::from)?;
        descriptors.push(descriptor);
        planes.push(plane);
    }
    ledger.release_live(native_outer_bytes)?;
    let model = if monochrome {
        ChannelModel::Gray
    } else if identity {
        ChannelModel::RGB
    } else {
        ChannelModel::YCbCr
    };
    let mut descriptor =
        ImageDescriptor::new(width, height, model, descriptors).map_err(ProcessingError::from)?;
    let active_color = clone_color_information_with_ledger(metadata.source_color(), ledger)?;
    descriptor = descriptor.with_color_information(active_color);
    if plan.has_alpha {
        descriptor = descriptor
            .with_alpha(if frame.alpha_premultiplied {
                AlphaAssociation::Premultiplied
            } else {
                AlphaAssociation::Straight
            })
            .map_err(ProcessingError::from)?;
    }
    metadata.set_coded_dimensions(Some((width, height)));
    let render_width = u32::try_from(frame.render_width)
        .map_err(|_| ProcessingError::ResourceLimit("native render width exceeds u32".into()))?;
    let render_height = u32::try_from(frame.render_height)
        .map_err(|_| ProcessingError::ResourceLimit("native render height exceeds u32".into()))?;
    metadata.set_render_dimensions(Some((render_width, render_height)));
    metadata.set_av1_description(Some(av1_info));
    let output = ImageFrame::from_parts(
        descriptor,
        PixelBuffer::u16(planes).map_err(ProcessingError::from)?,
        metadata,
        timing,
    )
    .map_err(ProcessingError::from)?;
    output.validate_with_limits(limits)?;
    Ok(output)
}

fn validate_native_pixel_information(
    frame: &DecodedFrame,
    rich: &RichAvifInfo,
) -> Result<(), ProcessingError> {
    let Some(pixi) = &rich.info.pixel_information else {
        return Ok(());
    };
    let color_planes = if frame.color_config.monochrome { 1 } else { 3 };
    if pixi.bits_per_channel.len() != color_planes {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "pixi channel count disagrees with native colour planes".into(),
        )));
    }
    if pixi
        .bits_per_channel
        .iter()
        .any(|bits| *bits != frame.bit_depth)
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
            "pixi precision disagrees with native AV1 precision".into(),
        )));
    }
    if let Some(channels) = &pixi.extended_channels {
        if channels.len() != pixi.bits_per_channel.len() {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "extended pixi channel count disagrees with colour channels".into(),
            )));
        }
        if channels.iter().any(|channel| channel.channel_idc == 3) {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "alpha pixi channel is auxiliary and cannot extend native colour count".into(),
            )));
        }
    }
    Ok(())
}

fn validate_icc_kinds(rich: &RichAvifInfo) -> Result<(), ProcessingError> {
    if let Some(kind) = rich.color_information.icc_color_type
        && kind != *b"prof"
        && kind != *b"rICC"
    {
        return Err(ProcessingError::Unsupported(
            "unsupported ICC colour type".into(),
        ));
    }
    Ok(())
}

fn validate_native_configuration(frame: &DecodedFrame) -> Result<(), ProcessingError> {
    let config = frame.color_config;
    if frame.bit_depth != config.bit_depth {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "frame and AV1 configuration bit depth differ".into(),
        )));
    }
    if config.high_bitdepth != (config.bit_depth > 8)
        || config.twelve_bit != (config.bit_depth == 12)
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "AV1 bit-depth flags do not match bit depth".into(),
        )));
    }
    let identity = config
        .color_description
        .map(|description| description.matrix_coefficients == 0)
        .unwrap_or(false);
    if identity && (config.subsampling_x || config.subsampling_y) {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "identity RGB AV1 must use full-resolution colour planes".into(),
        )));
    }
    let mut seen = [false; 4];
    for plane in &frame.buffers.planes {
        let id = usize::from(plane.layout.plane);
        if id >= seen.len() {
            return Err(ProcessingError::Unsupported(
                "native plane id is not supported".into(),
            ));
        }
        if seen[id] {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "duplicate native plane id".into(),
            )));
        }
        seen[id] = true;
        let (expected_x, expected_y) = match id {
            0 => (0, 0),
            1 | 2 if !config.monochrome => (
                u8::from(config.subsampling_x),
                u8::from(config.subsampling_y),
            ),
            3 => (0, 0),
            _ => {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "monochrome native frame has chroma planes".into(),
                )));
            }
        };
        if plane.layout.subsampling_x != expected_x || plane.layout.subsampling_y != expected_y {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native plane subsampling disagrees with AV1 configuration".into(),
            )));
        }
        let x_factor = 1usize << expected_x;
        let y_factor = 1usize << expected_y;
        let expected_width =
            frame.width.checked_add(x_factor - 1).ok_or_else(|| {
                ProcessingError::ResourceLimit("native plane width overflows".into())
            })? / x_factor;
        let expected_height = frame.height.checked_add(y_factor - 1).ok_or_else(|| {
            ProcessingError::ResourceLimit("native plane height overflows".into())
        })? / y_factor;
        if plane.layout.width != expected_width || plane.layout.height != expected_height {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native plane dimensions disagree with subsampling".into(),
            )));
        }
        let expected_samples = expected_width.checked_mul(expected_height).ok_or_else(|| {
            ProcessingError::ResourceLimit("native sample count overflows".into())
        })?;
        if plane.layout.sample_count != expected_samples || plane.samples.len() != expected_samples
        {
            return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                "native sample count disagrees with layout".into(),
            )));
        }
        let maximum = 1u32
            .checked_shl(u32::from(frame.bit_depth))
            .map(|value| value - 1)
            .ok_or_else(|| {
                ProcessingError::Invalid(HighresError::InvalidSamples(
                    "native bit depth cannot be represented".into(),
                ))
            })?;
        if plane
            .samples
            .iter()
            .any(|sample| u32::from(*sample) > maximum)
        {
            return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
                "native sample exceeds declared bit depth".into(),
            )));
        }
    }
    let expected_color_planes = if config.monochrome { 1 } else { 3 };
    let color_planes = seen[..3].iter().filter(|present| **present).count();
    if color_planes != expected_color_planes {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "native colour plane count disagrees with AV1 configuration".into(),
        )));
    }
    if seen[3]
        && (frame
            .buffers
            .planes
            .iter()
            .find(|plane| plane.layout.plane == 3)
            .is_none())
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
            "native alpha plane is missing".into(),
        )));
    }
    Ok(())
}

fn native_subsampling_factor(log2: u8) -> Result<u8, ProcessingError> {
    if log2 > 1 {
        return Err(ProcessingError::Unsupported(
            "native subsampling exponent is too large".into(),
        ));
    }
    1u8.checked_shl(u32::from(log2))
        .ok_or_else(|| ProcessingError::Unsupported("native subsampling factor overflows".into()))
}
