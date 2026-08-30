//! Explicit ICC-to-ICC conversion for encoded high-resolution RGB and Gray.
//!
//! This adapter is intentionally separate from [`super::convert_frame`].  It
//! accepts only normalized encoded F32 device samples, uses the ICC crate's
//! physical D50 PCS bridge, and never applies tone mapping, HDR conversion,
//! CICP signalling, or alpha colour transforms implicitly.

use std::mem::size_of;

use icc_profile::{
    ColorSpace as IccColorSpace, ParseLimits, Profile, RenderingIntent, Transform, TransformLimits,
    TransformOptions,
};

use super::allocation::ConstructionLedger;
use super::metadata::{
    ColorInformationSet, ConversionAlpha, ConversionDestination, ConversionIntent,
    ConversionPrimaries, ConversionSource, ConversionWhiteAdaptation, IccColorType,
    IccConversionAuthority, LastConversion, LastConversionParts,
};
use super::output_plan::{
    AdmittedOutputPlan, CandidateMaker, OutputOwnershipPlan, OwnerElement, OwnerKey, OwnerSink,
};
use super::processing::ProcessingError;
use super::types::{
    AlphaAssociation, ChannelModel, ChannelRole, ImageDescriptor, ImageFrame, PixelBuffer, Plane,
    PlaneDescriptor, PlaneLayout,
};
use super::{ResourceLimits, RgbPrimaries, SampleDomain, Subsampling};

/// Explicit source and destination ICC profiles for [`convert_frame_with_icc`].
///
/// Both profiles describe encoded device samples.  The adapter does not infer
/// a transfer function from CICP or from the frame's metadata; the caller must
/// provide the profiles and the frame domain must be [`SampleDomain::Encoded`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IccConversionOptions<'a> {
    source_profile: &'a [u8],
    destination_profile: &'a [u8],
    source_color_type: IccColorType,
    destination_color_type: IccColorType,
}

impl<'a> IccConversionOptions<'a> {
    pub const fn new(source_profile: &'a [u8], destination_profile: &'a [u8]) -> Self {
        Self {
            source_profile,
            destination_profile,
            source_color_type: IccColorType::Prof,
            destination_color_type: IccColorType::Prof,
        }
    }

    pub const fn with_source_color_type(mut self, color_type: IccColorType) -> Self {
        self.source_color_type = color_type;
        self
    }

    pub const fn with_destination_color_type(mut self, color_type: IccColorType) -> Self {
        self.destination_color_type = color_type;
        self
    }

    pub const fn source_profile(self) -> &'a [u8] {
        self.source_profile
    }

    pub const fn destination_profile(self) -> &'a [u8] {
        self.destination_profile
    }

    pub const fn source_color_type(self) -> IccColorType {
        self.source_color_type
    }

    pub const fn destination_color_type(self) -> IccColorType {
        self.destination_color_type
    }
}

struct IccCandidateMaker;

impl CandidateMaker for IccCandidateMaker {
    fn make<T: OwnerElement>(
        &mut self,
        _key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|error| {
            ProcessingError::Allocation(format!("ICC output owner allocation failed: {error}"))
        })?;
        Ok(values)
    }
}

struct IccOutputSink<'p, 'a> {
    admitted: &'p mut AdmittedOutputPlan<'a>,
    maker: &'p mut IccCandidateMaker,
}

impl OwnerSink for IccOutputSink<'_, '_> {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        self.admitted.fresh_metadata(key, count, self.maker)
    }

    fn commit_inline(&mut self, key: OwnerKey, bytes: usize) -> Result<(), ProcessingError> {
        self.admitted.commit_metadata_event(key, bytes)
    }
}

/// Convert encoded F32 Gray/RGB device samples through two ICC profiles.
///
/// The source and destination profiles are compiled with relative
/// colorimetric intent, black-point compensation disabled, and unclamped
/// execution.  Input samples are checked by the ICC transform in its
/// normalized device domain.  Alpha is copied as an independent straight
/// channel and is never passed through the ICC transform.  U8/U16, YCbCr,
/// HDR, CMYK, premultiplied alpha, CICP destinations, and implicit colour
/// conversion are intentionally outside this API.
pub fn convert_frame_with_icc(
    source: &ImageFrame,
    options: IccConversionOptions<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<ImageFrame, ProcessingError> {
    convert_frame_with_icc_planned(source, options, limits)
}

fn convert_frame_with_icc_planned(
    source: &ImageFrame,
    options: IccConversionOptions<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<ImageFrame, ProcessingError> {
    source.validate_with_limits(limits)?;
    let source_channels = validate_source_frame(source)?;
    validate_icc_inputs(options, limits)?;

    let source_owned = source_owned_bytes(source)?;
    let source_metadata = source
        .metadata()
        .metadata_bytes()
        .map_err(ProcessingError::from)?;
    if source_metadata > limits.max_metadata_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC source metadata exceeds the configured limit".into(),
        ));
    }

    // Profile parsing owns both payloads.  Reject a request whose borrowed
    // inputs plus the two parser-owned payloads cannot fit before invoking
    // the parser; otherwise an oversized request could allocate the first
    // Profile and fail only after the second ownership peak.
    let borrowed_profiles = borrowed_profile_union_bytes_excluding_owned(
        options.source_profile,
        options.destination_profile,
        [
            source
                .metadata()
                .source_color()
                .icc_profile()
                .map(|profile| {
                    (
                        profile,
                        source.metadata().source_color().icc_profile_capacity(),
                    )
                }),
            source
                .descriptor()
                .color_information()
                .icc_profile()
                .map(|profile| {
                    (
                        profile,
                        source
                            .descriptor()
                            .color_information()
                            .icc_profile_capacity(),
                    )
                }),
        ],
    )?;
    let parse_limits = icc_parse_limits(limits)?;
    let source_profile_bound = profile_parse_bound(options.source_profile, parse_limits)?;
    let destination_profile_bound = profile_parse_bound(options.destination_profile, parse_limits)?;
    let parser_profile_bytes = source_profile_bound
        .checked_add(destination_profile_bound)
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC parser ownership overflows".into()))?;
    let parser_peak = source_owned
        .checked_add(borrowed_profiles)
        .and_then(|bytes| bytes.checked_add(parser_profile_bytes))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC parser peak overflows".into()))?;
    // `max_frame_bytes` governs retained/output frame ownership.  Parser
    // handles and borrowed option storage are live-only and therefore belong
    // exclusively to the aggregate live check here.
    if parser_peak > limits.max_total_live_decoded_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC parser ownership exceeds construction limits".into(),
        ));
    }

    let source_profile =
        Profile::parse_with_limits(options.source_profile, parse_limits).map_err(map_icc_error)?;
    let source_profile_memory = source_profile.memory_usage().map_err(map_icc_error)?;
    if source_profile_memory.profile_bytes() > source_profile_bound {
        return Err(ProcessingError::ResourceLimit(
            "ICC source profile exceeds admitted parse memory".into(),
        ));
    }
    let second_parse_peak = source_owned
        .checked_add(borrowed_profiles)
        .and_then(|bytes| bytes.checked_add(source_profile_memory.profile_bytes()))
        .and_then(|bytes| bytes.checked_add(destination_profile_bound))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC second parse peak overflows".into()))?;
    if second_parse_peak > limits.max_total_live_decoded_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC destination parse exceeds construction limits".into(),
        ));
    }
    let destination_profile = Profile::parse_with_limits(options.destination_profile, parse_limits)
        .map_err(map_icc_error)?;
    let destination_profile_memory = destination_profile.memory_usage().map_err(map_icc_error)?;
    if destination_profile_memory.profile_bytes() > destination_profile_bound {
        return Err(ProcessingError::ResourceLimit(
            "ICC destination profile exceeds admitted parse memory".into(),
        ));
    }
    let destination_channels = icc_channels(&destination_profile)?;
    let expected_source = if source_channels == 1 {
        IccColorSpace::Gray
    } else {
        IccColorSpace::Rgb
    };
    if source_profile.color_space() != expected_source {
        return Err(ProcessingError::Unsupported(
            "source frame channels do not match the source ICC profile".into(),
        ));
    }

    // The output inventory is independent of transform compilation once the
    // destination profile's channel shape is known.  Admit/check it before
    // compiling the potentially large CMS route so a rejected output cannot
    // cause transform allocation first.
    let width = source.descriptor().width();
    let height = source.descriptor().height();
    let pixels = checked_pixels(width, height)?;
    let has_alpha = source.descriptor().alpha() != AlphaAssociation::None;
    let output_planes = destination_channels
        .checked_add(usize::from(has_alpha))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output plane count overflows".into()))?;
    let output_plan = OutputOwnershipPlan::inspect_with_icc_profiles(
        source,
        output_planes,
        pixels,
        options.source_profile.len(),
        options.destination_profile.len(),
        true,
        limits,
    )?;
    let output_total = output_plan.planned_output_bytes()?;
    let output_metadata = output_plan.planned_metadata_bytes();
    let combined_metadata = source_metadata
        .checked_add(output_metadata)
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC metadata budget overflows".into()))?;
    if combined_metadata > limits.max_metadata_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC source and destination metadata exceed the configured limit".into(),
        ));
    }

    let profile_memory = source_profile
        .memory_usage()
        .and_then(|memory| {
            destination_profile
                .memory_usage()
                .and_then(|other| memory.checked_add(other))
        })
        .map_err(map_icc_error)?;
    // Profiles and the borrowed option slices remain live while the
    // transform is built and while the output is materialized.  Reserve the
    // complete live-only aggregate up front; output frame ownership remains a
    // separate ledger class and is added by `OutputOwnershipPlan::admit`.
    let compile_budget = limits
        .max_total_live_decoded_bytes
        .checked_sub(source_owned)
        .and_then(|bytes| bytes.checked_sub(borrowed_profiles))
        .and_then(|bytes| bytes.checked_sub(output_total))
        .and_then(|bytes| bytes.checked_sub(profile_memory.profile_bytes()))
        .ok_or_else(|| {
            ProcessingError::ResourceLimit(
                "ICC source, borrowed profiles and output exceed live construction budget".into(),
            )
        })?;
    let max_clut_entries = limits.max_clut_bytes / size_of::<f32>();
    if max_clut_entries == 0 {
        return Err(ProcessingError::ResourceLimit(
            "ICC CLUT byte limit cannot admit one f32 entry".into(),
        ));
    }
    let transform_limits = TransformLimits::builder()
        .max_compiled_bytes(compile_budget)
        .max_curve_entries(limits.max_parser_entries)
        .max_clut_entries(max_clut_entries)
        .build()
        .map_err(|message| ProcessingError::ResourceLimit(message.into()))?;
    let transform = Transform::new_with_limits(
        &source_profile,
        &destination_profile,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            black_point_compensation: false,
            clamp: false,
        },
        transform_limits,
    )
    .map_err(map_icc_error)?;
    if transform.input_channels() != source_channels
        || transform.output_channels() != destination_channels
    {
        return Err(ProcessingError::Unsupported(
            "compiled ICC route channel count does not match the frame".into(),
        ));
    }
    let transform_memory = transform.memory_usage().map_err(map_icc_error)?;
    let live_seed = source_owned
        .checked_add(borrowed_profiles)
        .and_then(|bytes| bytes.checked_add(transform_memory.build_peak_bytes()))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC live ownership overflows".into()))?;
    if live_seed
        .checked_add(output_total)
        .is_none_or(|total| total > limits.max_total_live_decoded_bytes)
    {
        return Err(ProcessingError::ResourceLimit(
            "ICC transform and output exceed live construction limits".into(),
        ));
    }
    let ledger = ConstructionLedger::new_with_ownership(0, 0, live_seed, limits)?;
    let mut admitted = output_plan.admit(ledger, source.metadata())?;
    let checkpoint = admitted.checkpoint();
    let mut maker = IccCandidateMaker;
    let result = materialize_icc_frame(
        source,
        options,
        &transform,
        source_channels,
        destination_channels,
        &mut admitted,
        &mut maker,
        width,
        height,
        pixels,
        has_alpha,
    );
    match result {
        Ok(frame) => match admitted.complete_for(&frame) {
            Ok(()) => Ok(frame),
            Err(error) => {
                drop(frame);
                admitted.restore(checkpoint);
                Err(error)
            }
        },
        Err(error) => {
            admitted.restore(checkpoint);
            Err(error)
        }
    }
}

fn validate_icc_inputs(
    options: IccConversionOptions<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<(), ProcessingError> {
    if options.source_profile.is_empty() || options.destination_profile.is_empty() {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidMetadata("ICC profile is empty".into()),
        ));
    }
    if options.source_profile.len() > limits.max_icc_bytes
        || options.destination_profile.len() > limits.max_icc_bytes
    {
        return Err(ProcessingError::ResourceLimit(
            "ICC profile exceeds the configured byte limit".into(),
        ));
    }
    if limits.max_total_live_decoded_bytes == 0 {
        return Err(ProcessingError::ResourceLimit(
            "ICC conversion requires non-zero live memory".into(),
        ));
    }
    Ok(())
}

fn icc_parse_limits(limits: &ResourceLimits) -> Result<ParseLimits, ProcessingError> {
    let mut result = ParseLimits::default();
    result.max_profile_size = result.max_profile_size.min(limits.max_icc_bytes);
    result.max_tag_count = result.max_tag_count.min(limits.max_parser_entries);
    result.max_tag_size = result.max_tag_size.min(limits.max_icc_bytes);
    result.max_curve_entries = result.max_curve_entries.min(limits.max_parser_entries);
    if result.max_tag_count == 0 || result.max_tag_size == 0 || result.max_curve_entries < 2 {
        return Err(ProcessingError::ResourceLimit(
            "ICC parser limits cannot admit a profile".into(),
        ));
    }
    Ok(result)
}

fn icc_channels(profile: &Profile) -> Result<usize, ProcessingError> {
    match profile.color_space() {
        IccColorSpace::Gray => Ok(1),
        IccColorSpace::Rgb => Ok(3),
        IccColorSpace::Cmyk | IccColorSpace::NColor(_) => Err(ProcessingError::Unsupported(
            "ICC CMYK/N-color destinations are unsupported by this adapter".into(),
        )),
    }
}

#[allow(dead_code)]
fn convert_frame_with_icc_legacy(
    source: &ImageFrame,
    options: IccConversionOptions<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<ImageFrame, ProcessingError> {
    source.validate_with_limits(limits)?;
    let source_channels = validate_source_frame(source)?;

    if options.source_profile.is_empty() || options.destination_profile.is_empty() {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidMetadata("ICC profile is empty".into()),
        ));
    }
    if options.source_profile.len() > limits.max_icc_bytes
        || options.destination_profile.len() > limits.max_icc_bytes
    {
        return Err(ProcessingError::ResourceLimit(
            "ICC profile exceeds the configured byte limit".into(),
        ));
    }
    if options.destination_profile.len() > limits.max_metadata_bytes {
        return Err(ProcessingError::ResourceLimit(
            "destination ICC metadata exceeds the configured metadata limit".into(),
        ));
    }
    if limits.max_total_live_decoded_bytes == 0 {
        return Err(ProcessingError::ResourceLimit(
            "ICC conversion requires non-zero live memory".into(),
        ));
    }
    let source_metadata_bytes = source
        .metadata()
        .fresh_clone_retained_bytes()
        .map_err(ProcessingError::from)?;
    // The source remains live for the complete conversion.  Account for its
    // actual owned descriptor/pixel/metadata graph before admitting either
    // the parser's profile copies or the output candidate.
    let source_owned_bytes = source_owned_bytes(source)?;
    let mut parse_limits = ParseLimits::default();
    parse_limits.max_profile_size = parse_limits.max_profile_size.min(limits.max_icc_bytes);
    parse_limits.max_tag_count = parse_limits.max_tag_count.min(limits.max_parser_entries);
    parse_limits.max_tag_size = parse_limits.max_tag_size.min(limits.max_icc_bytes);
    parse_limits.max_curve_entries = parse_limits
        .max_curve_entries
        .min(limits.max_parser_entries);
    if parse_limits.max_tag_count == 0
        || parse_limits.max_tag_size == 0
        || parse_limits.max_curve_entries < 2
    {
        return Err(ProcessingError::ResourceLimit(
            "ICC parser limits cannot admit a profile".into(),
        ));
    }
    let metadata_plan = source_metadata_bytes
        .checked_add(options.source_profile.len())
        .and_then(|value| value.checked_add(options.destination_profile.len()))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC metadata plan overflows".into()))?;
    if metadata_plan > limits.max_metadata_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC source and destination metadata exceed the configured limit".into(),
        ));
    }
    let preflight_output = estimated_output_bytes(
        source,
        options.destination_profile.len(),
        options.source_profile.len(),
        if source.descriptor().alpha() == AlphaAssociation::None {
            3
        } else {
            4
        },
        3,
        checked_pixels(source.descriptor().width(), source.descriptor().height())?,
    )?;
    // The caller may pass two views into the same owned ICC blob.  Charge
    // the borrowed input interval once for the construction peak, while the
    // ICC parser is still allowed to make its two independent immutable
    // profile owners below.
    let borrowed_profile_bytes =
        borrowed_profile_union_bytes(options.source_profile, options.destination_profile)?;
    let source_profile_bound = profile_parse_bound(options.source_profile, parse_limits)?;
    let destination_profile_bound = profile_parse_bound(options.destination_profile, parse_limits)?;
    let parser_profile_bytes = borrowed_profile_bytes
        .checked_add(source_profile_bound)
        .and_then(|value| value.checked_add(destination_profile_bound))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC profile plan overflows".into()))?;
    let preflight_bytes = source_owned_bytes
        .checked_add(parser_profile_bytes)
        .and_then(|value| value.checked_add(preflight_output))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC conversion plan overflows".into()))?;
    let compile_budget = limits
        .max_total_live_decoded_bytes
        .checked_sub(preflight_bytes)
        .ok_or_else(|| {
            ProcessingError::ResourceLimit(
                "ICC profiles and output exceed live memory before compilation".into(),
            )
        })?;
    if compile_budget == 0 {
        return Err(ProcessingError::ResourceLimit(
            "ICC transform has no admitted compilation memory".into(),
        ));
    }

    let source_profile =
        Profile::parse_with_limits(options.source_profile, parse_limits).map_err(map_icc_error)?;
    let source_profile_memory = source_profile.memory_usage().map_err(map_icc_error)?;
    if source_profile_memory.profile_bytes() > source_profile_bound {
        return Err(ProcessingError::ResourceLimit(
            "ICC source profile exceeds its admitted parse memory".into(),
        ));
    }
    // Recheck the second parse boundary with the first profile's actual
    // allocation. This keeps malformed/tag-rich destinations from being
    // parsed after the aggregate construction budget is already exhausted.
    let second_parse_live = source_owned_bytes
        .checked_add(borrowed_profile_bytes)
        .and_then(|value| value.checked_add(source_profile_memory.profile_bytes()))
        .and_then(|value| value.checked_add(destination_profile_bound))
        .and_then(|value| value.checked_add(preflight_output))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC parse plan overflows".into()))?;
    if second_parse_live > limits.max_total_live_decoded_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC destination profile exceeds the remaining parse budget".into(),
        ));
    }
    let destination_profile = Profile::parse_with_limits(options.destination_profile, parse_limits)
        .map_err(map_icc_error)?;
    let destination_profile_memory = destination_profile.memory_usage().map_err(map_icc_error)?;
    if destination_profile_memory.profile_bytes() > destination_profile_bound {
        return Err(ProcessingError::ResourceLimit(
            "ICC destination profile exceeds its admitted parse memory".into(),
        ));
    }
    let destination_channels = match destination_profile.color_space() {
        IccColorSpace::Gray => 1,
        IccColorSpace::Rgb => 3,
        IccColorSpace::Cmyk | IccColorSpace::NColor(_) => {
            return Err(ProcessingError::Unsupported(
                "ICC CMYK/N-color destinations are unsupported by this adapter".into(),
            ));
        }
    };
    if source_profile.color_space()
        != if source_channels == 1 {
            IccColorSpace::Gray
        } else {
            IccColorSpace::Rgb
        }
    {
        return Err(ProcessingError::Unsupported(
            "source frame channels do not match the source ICC profile".into(),
        ));
    }

    let max_clut_entries = limits.max_clut_bytes / size_of::<f32>();
    if max_clut_entries == 0 {
        return Err(ProcessingError::ResourceLimit(
            "ICC CLUT byte limit cannot admit one f32 entry".into(),
        ));
    }
    let transform_limits = TransformLimits::builder()
        .max_compiled_bytes(compile_budget)
        .max_curve_entries(limits.max_parser_entries)
        .max_clut_entries(max_clut_entries)
        .build()
        .map_err(|message| ProcessingError::ResourceLimit(message.into()))?;
    let transform = Transform::new_with_limits(
        &source_profile,
        &destination_profile,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            black_point_compensation: false,
            clamp: false,
        },
        transform_limits,
    )
    .map_err(map_icc_error)?;
    if transform.input_channels() != source_channels
        || transform.output_channels() != destination_channels
    {
        return Err(ProcessingError::Unsupported(
            "compiled ICC route channel count does not match the frame".into(),
        ));
    }

    let output_alpha_index = source
        .descriptor()
        .planes()
        .iter()
        .position(|plane| plane.roles().contains(&ChannelRole::Alpha));
    let width = source.descriptor().width();
    let height = source.descriptor().height();
    let pixels = checked_pixels(width, height)?;
    let output_planes = if output_alpha_index.is_some() {
        destination_channels + 1
    } else {
        destination_channels
    };
    let transform_memory = transform.memory_usage().map_err(map_icc_error)?;
    let output_bytes = estimated_output_bytes(
        source,
        options.destination_profile.len(),
        options.source_profile.len(),
        output_planes,
        destination_channels,
        pixels,
    )?;
    let required = transform_memory
        .build_peak_bytes()
        .checked_add(output_bytes)
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC conversion memory overflows".into()))?;
    let required_live = source_owned_bytes
        .checked_add(required)
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC live memory overflows".into()))?;
    if required > limits.max_frame_bytes || required_live > limits.max_total_live_decoded_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC transform and output exceed live memory limits".into(),
        ));
    }

    let metadata_plan = source_metadata_bytes
        .checked_add(options.source_profile.len())
        .and_then(|value| value.checked_add(options.destination_profile.len()))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC metadata plan overflows".into()))?;
    let source_metadata_owned = source
        .metadata()
        .metadata_bytes()
        .map_err(ProcessingError::from)?;
    let mut allocation = ConstructionLedger::new_with_ownership(
        transform_memory.build_peak_bytes(),
        source_metadata_owned,
        source_owned_bytes,
        limits,
    )?;
    allocation.reserve_output(output_bytes, metadata_plan)?;
    let mut output_samples = allocate_samples(destination_channels, pixels, &mut allocation)?;
    let mut alpha_samples = output_alpha_index
        .is_some()
        .then(|| allocate_one_samples(pixels, &mut allocation))
        .transpose()?;
    transform_pixels(
        source,
        source_channels,
        destination_channels,
        &transform,
        &mut output_samples,
    )?;
    if let (Some(alpha_index), Some(destination)) = (output_alpha_index, alpha_samples.as_mut()) {
        let alpha = source
            .pixels()
            .f32_planes()
            .and_then(|planes| planes.get(alpha_index))
            .ok_or_else(|| {
                ProcessingError::Invalid(super::HighresError::InvalidLayout(
                    "ICC alpha plane storage is missing".into(),
                ))
            })?;
        copy_alpha_samples(alpha, destination)?;
    }

    let mut metadata_sink = LedgerMetadataSink {
        allocation: &mut allocation,
    };
    let mut metadata = source
        .metadata()
        .try_clone_for_conversion_with_sink(&mut metadata_sink)?;
    // Keep the explicit source route distinguishable from the destination
    // active colour descriptor.  This replaces only the source ICC payload;
    // all unrelated source metadata remains cloned above.
    let mut source_profile_copy = allocation.try_new_metadata_vec_with_limit(
        options.source_profile.len(),
        Some(limits.max_icc_bytes),
        |count| {
            let mut values = Vec::new();
            values.try_reserve_exact(count).map_err(|error| {
                ProcessingError::Allocation(format!(
                    "ICC source profile allocation failed: {error}"
                ))
            })?;
            Ok(values)
        },
    )?;
    source_profile_copy.extend_from_slice(options.source_profile);
    metadata
        .source_color_mut()
        .set_icc_profile(source_profile_copy)
        .map_err(ProcessingError::from)?;
    metadata
        .source_color_mut()
        .set_icc_color_type(options.source_color_type);
    metadata
        .source_color_mut()
        .push_admitted_provenance(super::metadata::ColorProvenance::Explicit)?;
    let destination_color = destination_color_information(
        options.destination_profile,
        options.destination_color_type,
        &mut allocation,
    )?;
    // ICC conversion history is deliberately independent of stale CICP or
    // linear-route metadata copied from the source frame.  Keep the actual
    // profiles in source/active colour information and retain only these
    // copyable authority descriptors in the history record.
    metadata.set_last_conversion(LastConversion::from_parts(LastConversionParts {
        source: ConversionSource::Icc,
        source_cicp: None,
        source_icc: Some(IccConversionAuthority::from_color_type(
            options.source_color_type,
        )),
        source_primaries: RgbPrimaries::srgb(),
        source_primaries_authority: ConversionPrimaries::IccDefined,
        source_domain: SampleDomain::Encoded,
        destination: ConversionDestination::Icc,
        destination_icc: Some(IccConversionAuthority::from_color_type(
            options.destination_color_type,
        )),
        destination_domain: SampleDomain::Encoded,
        destination_primaries: RgbPrimaries::srgb(),
        destination_primaries_authority: ConversionPrimaries::IccDefined,
        intent: ConversionIntent::RelativeColorimetric,
        native: None,
        white_adaptation: ConversionWhiteAdaptation::IccD50Pcs,
        alpha: ConversionAlpha::PreserveAssociation,
    }));
    let descriptor = output_descriptor(
        width,
        height,
        destination_channels,
        output_alpha_index.is_some(),
        destination_color,
    )?;
    let pixels = output_pixel_buffer(
        width,
        height,
        destination_channels,
        output_samples,
        alpha_samples,
    )?;
    let frame = ImageFrame::from_parts(descriptor, pixels, metadata, source.timing())
        .map_err(ProcessingError::from)?;
    let output_metadata_bytes = frame
        .metadata()
        .metadata_bytes()
        .map_err(ProcessingError::from)?;
    let actual_output = frame
        .descriptor()
        .owned_bytes()
        .map_err(ProcessingError::from)?
        .checked_add(
            frame
                .pixels()
                .owned_bytes()
                .map_err(ProcessingError::from)?,
        )
        .and_then(|value| value.checked_add(output_metadata_bytes))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output ownership overflows".into()))?;
    let resident = transform_memory
        .build_peak_bytes()
        .checked_add(source_owned_bytes)
        .and_then(|value| value.checked_add(actual_output))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC resident memory overflows".into()))?;
    if resident > limits.max_frame_bytes || resident > limits.max_total_live_decoded_bytes {
        return Err(ProcessingError::ResourceLimit(
            "ICC transform and actual output exceed live memory limits".into(),
        ));
    }
    Ok(frame)
}

fn materialize_icc_frame(
    source: &ImageFrame,
    options: IccConversionOptions<'_>,
    transform: &Transform,
    source_channels: usize,
    destination_channels: usize,
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut IccCandidateMaker,
    width: u32,
    height: u32,
    pixels: usize,
    has_alpha: bool,
) -> std::result::Result<ImageFrame, ProcessingError> {
    let plane_limit = pixels
        .checked_mul(size_of::<f32>())
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output plane overflows".into()))?;
    // Keep the short list of colour planes on the stack.  In particular, do
    // not create an image-sized (or even fallible-but-unaccounted) outer Vec
    // before the ownership plan has admitted the actual plane owners.
    let mut output_samples: [Vec<f32>; 3] = std::array::from_fn(|_| Vec::new());
    for channel in 0..destination_channels {
        let mut values =
            admitted.fresh_with_limit(OwnerKey::Sample(channel), pixels, plane_limit, maker)?;
        values.resize(pixels, 0.0);
        output_samples[channel] = values;
    }
    let mut alpha_samples = if has_alpha {
        let mut values = admitted.fresh_with_limit(
            OwnerKey::Sample(destination_channels),
            pixels,
            plane_limit,
            maker,
        )?;
        values.resize(pixels, 0.0);
        Some(values)
    } else {
        None
    };
    transform_pixels(
        source,
        source_channels,
        destination_channels,
        transform,
        &mut output_samples[..destination_channels],
    )?;
    if let Some(destination) = alpha_samples.as_mut() {
        let alpha_index = source
            .descriptor()
            .planes()
            .iter()
            .position(|plane| plane.roles().contains(&ChannelRole::Alpha))
            .ok_or_else(|| {
                ProcessingError::Invalid(super::HighresError::InvalidLayout(
                    "ICC alpha plane storage is missing".into(),
                ))
            })?;
        let alpha = source
            .pixels()
            .f32_planes()
            .and_then(|planes| planes.get(alpha_index))
            .ok_or_else(|| {
                ProcessingError::Invalid(super::HighresError::InvalidLayout(
                    "ICC alpha plane storage is missing".into(),
                ))
            })?;
        copy_alpha_samples(alpha, destination)?;
    }

    // The ownership plan admits descriptor/pixel containers before metadata
    // owners. Build these containers with an empty colour descriptor; the
    // destination ICC object is attached after its keyed owners are admitted.
    let (mut descriptor, pixels_buffer) = output_parts_planned(
        width,
        height,
        destination_channels,
        has_alpha,
        output_samples,
        alpha_samples,
        admitted,
        maker,
    )?;

    let mut metadata_sink = IccOutputSink { admitted, maker };
    let mut metadata = source
        .metadata()
        .try_clone_for_icc_conversion_with_sink(&mut metadata_sink)?;
    let mut source_profile = admitted.fresh_metadata(
        OwnerKey::IccSourceProfile,
        options.source_profile.len(),
        maker,
    )?;
    source_profile.extend_from_slice(options.source_profile);
    metadata
        .source_color_mut()
        .set_icc_profile_admitted(source_profile)?;
    metadata
        .source_color_mut()
        .set_icc_color_type(options.source_color_type);
    metadata
        .source_color_mut()
        .push_admitted_provenance(super::metadata::ColorProvenance::Explicit)?;
    let mut destination_profile = admitted.fresh_metadata(
        OwnerKey::IccDestinationProfile,
        options.destination_profile.len(),
        maker,
    )?;
    destination_profile.extend_from_slice(options.destination_profile);
    let mut destination_provenance =
        admitted.fresh_metadata(OwnerKey::IccDestinationProvenance, 1, maker)?;
    destination_provenance.push(super::metadata::ColorProvenance::Explicit);
    let destination_color = ColorInformationSet::from_owned_parts(
        destination_provenance,
        Some(destination_profile),
        Some(options.destination_color_type),
        None,
        None,
        Vec::new(),
    );
    descriptor = descriptor.with_color_information(destination_color);
    metadata.set_last_conversion(LastConversion::from_parts(LastConversionParts {
        source: ConversionSource::Icc,
        source_cicp: None,
        source_icc: Some(IccConversionAuthority::from_color_type(
            options.source_color_type,
        )),
        source_primaries: source
            .descriptor()
            .primaries()
            .unwrap_or_else(RgbPrimaries::srgb),
        source_primaries_authority: ConversionPrimaries::IccDefined,
        source_domain: SampleDomain::Encoded,
        destination: ConversionDestination::Icc,
        destination_icc: Some(IccConversionAuthority::from_color_type(
            options.destination_color_type,
        )),
        destination_domain: SampleDomain::Encoded,
        destination_primaries: RgbPrimaries::srgb(),
        destination_primaries_authority: ConversionPrimaries::IccDefined,
        intent: ConversionIntent::RelativeColorimetric,
        native: None,
        white_adaptation: ConversionWhiteAdaptation::IccD50Pcs,
        alpha: ConversionAlpha::PreserveAssociation,
    }));
    ImageFrame::from_parts(descriptor, pixels_buffer, metadata, source.timing())
        .map_err(ProcessingError::from)
}

fn output_parts_planned(
    width: u32,
    height: u32,
    channels: usize,
    has_alpha: bool,
    samples: [Vec<f32>; 3],
    alpha: Option<Vec<f32>>,
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut IccCandidateMaker,
) -> std::result::Result<(ImageDescriptor, PixelBuffer), ProcessingError> {
    let plane_count = channels
        .checked_add(usize::from(has_alpha))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output plane count overflows".into()))?;
    let mut descriptors = admitted.fresh_with(OwnerKey::DescriptorOuter, plane_count, maker)?;
    let mut planes = admitted.fresh_with(OwnerKey::PixelOuter, plane_count, maker)?;
    let roles = if channels == 1 {
        [
            ChannelRole::Gray,
            ChannelRole::Alpha,
            ChannelRole::Alpha,
            ChannelRole::Alpha,
        ]
    } else {
        [
            ChannelRole::Red,
            ChannelRole::Green,
            ChannelRole::Blue,
            ChannelRole::Alpha,
        ]
    };
    for (plane, values) in samples.into_iter().take(channels).chain(alpha).enumerate() {
        let descriptor_layout = planned_layout(
            width,
            height,
            OwnerKey::DescriptorLayout { plane },
            admitted,
            maker,
        )?;
        let mut role_values = admitted.fresh_with(OwnerKey::Role { plane }, 1, maker)?;
        role_values.push(roles[plane]);
        descriptors.push(
            PlaneDescriptor::new(descriptor_layout, role_values, 32)
                .map_err(ProcessingError::from)?,
        );
        let pixel_layout = planned_layout(
            width,
            height,
            OwnerKey::PixelLayout { plane },
            admitted,
            maker,
        )?;
        planes.push(Plane::new(pixel_layout, values).map_err(ProcessingError::from)?);
    }
    let model = if channels == 1 {
        ChannelModel::Gray
    } else {
        ChannelModel::RGB
    };
    let descriptor = ImageDescriptor::new(width, height, model, descriptors)
        .map_err(ProcessingError::from)?
        .with_alpha(if has_alpha {
            AlphaAssociation::Straight
        } else {
            AlphaAssociation::None
        })
        .map_err(ProcessingError::from)?
        .with_color_information(ColorInformationSet::new())
        .with_domain(SampleDomain::Encoded);
    let pixels = PixelBuffer::f32(planes).map_err(ProcessingError::from)?;
    Ok((descriptor, pixels))
}

fn planned_layout(
    width: u32,
    height: u32,
    key: OwnerKey,
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut IccCandidateMaker,
) -> std::result::Result<PlaneLayout, ProcessingError> {
    let row_stride = usize::try_from(width).map_err(|_| {
        ProcessingError::ResourceLimit("ICC output width exceeds platform limits".into())
    })?;
    let mut offsets = admitted.fresh_with(key, 1, maker)?;
    offsets.push(0);
    PlaneLayout::new(width, height, row_stride, 1, offsets, Subsampling::FULL)
        .map_err(ProcessingError::from)
}

fn validate_source_frame(source: &ImageFrame) -> std::result::Result<usize, ProcessingError> {
    if source.descriptor().domain() != SampleDomain::Encoded {
        return Err(ProcessingError::Unsupported(
            "ICC adapter requires an explicitly encoded source domain".into(),
        ));
    }
    if source.descriptor().alpha() == AlphaAssociation::Premultiplied {
        return Err(ProcessingError::Unsupported(
            "premultiplied alpha is unsupported by the ICC adapter".into(),
        ));
    }
    let channels = match source.descriptor().model() {
        ChannelModel::Gray => 1,
        ChannelModel::RGB => 3,
        ChannelModel::YCbCr => {
            return Err(ProcessingError::Unsupported(
                "YCbCr input requires an explicit RGB conversion before ICC".into(),
            ));
        }
    };
    let planes = source.pixels().f32_planes().ok_or_else(|| {
        ProcessingError::Unsupported("ICC adapter accepts encoded F32 input only".into())
    })?;
    let expected_roles: &[ChannelRole] = if channels == 1 {
        &[ChannelRole::Gray]
    } else {
        &[ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
    };
    let width = source.descriptor().width();
    let height = source.descriptor().height();
    if planes.len() != source.descriptor().planes().len() {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidLayout(
                "ICC source plane count does not match its descriptor".into(),
            ),
        ));
    }
    let mut found_colors = 0usize;
    let mut found_alpha = 0usize;
    for descriptor in source.descriptor().planes() {
        let roles = descriptor.roles();
        if roles.len() != 1 {
            return Err(ProcessingError::Unsupported(
                "ICC adapter requires one role per source plane".into(),
            ));
        }
        let role = roles[0];
        let is_color = expected_roles.contains(&role);
        let is_alpha = role == ChannelRole::Alpha;
        if !is_color && !is_alpha {
            return Err(ProcessingError::Unsupported(
                "ICC adapter does not preserve extra source channels".into(),
            ));
        }
        if descriptor.layout().width() != width
            || descriptor.layout().height() != height
            || descriptor.meaningful_bits() != 32
        {
            return Err(ProcessingError::Unsupported(
                "ICC adapter requires full-resolution 32-bit source planes".into(),
            ));
        }
        if is_color {
            found_colors = found_colors.checked_add(1).ok_or_else(|| {
                ProcessingError::ResourceLimit("ICC source channel count overflows".into())
            })?;
        } else if is_alpha {
            found_alpha = found_alpha.checked_add(1).ok_or_else(|| {
                ProcessingError::ResourceLimit("ICC source alpha count overflows".into())
            })?;
        }
    }
    if found_colors != channels {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidLayout(
                "ICC source color plane count does not match its model".into(),
            ),
        ));
    }
    if (source.descriptor().alpha() == AlphaAssociation::None) != (found_alpha == 0) {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidLayout(
                "ICC source alpha association does not match its planes".into(),
            ),
        ));
    }
    Ok(channels)
}

fn checked_pixels(width: u32, height: u32) -> std::result::Result<usize, ProcessingError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC pixel count overflows".into()))
}

fn source_owned_bytes(source: &ImageFrame) -> std::result::Result<usize, ProcessingError> {
    let descriptor = source
        .descriptor()
        .owned_bytes()
        .map_err(ProcessingError::from)?;
    let pixels = source
        .pixels()
        .owned_bytes()
        .map_err(ProcessingError::from)?;
    let metadata = source
        .metadata()
        .metadata_bytes()
        .map_err(ProcessingError::from)?;
    descriptor
        .checked_add(pixels)
        .and_then(|value| value.checked_add(metadata))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC source ownership overflows".into()))
}

fn profile_parse_bound(
    profile: &[u8],
    limits: ParseLimits,
) -> std::result::Result<usize, ProcessingError> {
    if profile.len() < 132 {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidMetadata("ICC profile header is truncated".into()),
        ));
    }
    let declared_length =
        u32::from_be_bytes([profile[0], profile[1], profile[2], profile[3]]) as usize;
    if declared_length < 132 || declared_length > profile.len() {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidMetadata("ICC profile length is invalid".into()),
        ));
    }
    let tag_count =
        u32::from_be_bytes([profile[128], profile[129], profile[130], profile[131]]) as usize;
    if tag_count > limits.max_tag_count {
        return Err(ProcessingError::ResourceLimit(
            "ICC tag count exceeds the configured parse limit".into(),
        ));
    }
    // `Profile::memory_usage` also accounts for the immutable ProfileInner
    // header (including parsed header/options and its Vec headers).  Keep a
    // fixed conservative allowance here because ProfileInner is private to
    // the ICC crate; the post-parse memory_usage check remains authoritative.
    const PROFILE_INNER_OVERHEAD: usize = 512;
    size_of::<Profile>()
        .checked_add(PROFILE_INNER_OVERHEAD)
        .and_then(|value| value.checked_add(declared_length))
        .and_then(|value| {
            value.checked_add(tag_count.checked_mul(size_of::<(u32, usize, usize)>())?)
        })
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC parse memory overflows".into()))
}

/// Return the number of bytes occupied by the union of two borrowed slices.
///
/// ICC parsing still creates separate owned profile objects, so this is only
/// the input-side peak charge.  Pointer arithmetic is performed in the
/// address domain with checked ends and never dereferences either slice.
fn borrowed_profile_union_bytes(
    source: &[u8],
    destination: &[u8],
) -> std::result::Result<usize, ProcessingError> {
    let source_start = source.as_ptr() as usize;
    let destination_start = destination.as_ptr() as usize;
    let source_end = source_start
        .checked_add(source.len())
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC source interval overflows".into()))?;
    let destination_end = destination_start
        .checked_add(destination.len())
        .ok_or_else(|| {
            ProcessingError::ResourceLimit("ICC destination interval overflows".into())
        })?;
    let (first_start, first_end, second_start, second_end) = if source_start <= destination_start {
        (source_start, source_end, destination_start, destination_end)
    } else {
        (destination_start, destination_end, source_start, source_end)
    };
    if second_start <= first_end {
        second_end
            .checked_sub(first_start)
            .ok_or_else(|| ProcessingError::ResourceLimit("ICC interval size overflows".into()))
    } else {
        source
            .len()
            .checked_add(destination.len())
            .ok_or_else(|| ProcessingError::ResourceLimit("ICC interval size overflows".into()))
    }
}

/// Return borrowed profile bytes that are not already covered by ICC buffers
/// owned by the source frame.  This keeps an option borrowed from either the
/// source metadata or the active descriptor from being charged a second time
/// during the parser peak.  The fixed endpoint walk handles arbitrary overlap
/// without allocating an interval table.
fn borrowed_profile_union_bytes_excluding_owned(
    source: &[u8],
    destination: &[u8],
    owned: [Option<(&[u8], usize)>; 2],
) -> std::result::Result<usize, ProcessingError> {
    let mut inputs = [None; 2];
    inputs[0] = Some(ByteInterval::from_slice(source)?);
    inputs[1] = Some(ByteInterval::from_slice(destination)?);
    let mut owned_intervals = [None; 2];
    for (index, value) in owned.into_iter().enumerate() {
        if let Some((profile, capacity)) = value {
            owned_intervals[index] = ByteInterval::from_raw(profile.as_ptr(), capacity)?;
        }
    }
    let mut endpoints = [0usize; 8];
    let mut endpoint_count = 0usize;
    for interval in inputs.into_iter().flatten() {
        endpoints[endpoint_count] = interval.start;
        endpoint_count += 1;
        endpoints[endpoint_count] = interval.end;
        endpoint_count += 1;
    }
    for interval in owned_intervals.into_iter().flatten() {
        endpoints[endpoint_count] = interval.start;
        endpoint_count += 1;
        endpoints[endpoint_count] = interval.end;
        endpoint_count += 1;
    }
    endpoints[..endpoint_count].sort_unstable();
    let mut total = 0usize;
    for pair in endpoints[..endpoint_count].windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start == end {
            continue;
        }
        let input_covered = inputs
            .into_iter()
            .flatten()
            .any(|interval| interval.contains(start, end));
        let owned_covered = owned_intervals
            .into_iter()
            .flatten()
            .any(|interval| interval.contains(start, end));
        if input_covered && !owned_covered {
            total = total.checked_add(end - start).ok_or_else(|| {
                ProcessingError::ResourceLimit("ICC interval union overflows".into())
            })?;
        }
    }
    Ok(total)
}

#[derive(Clone, Copy)]
struct ByteInterval {
    start: usize,
    end: usize,
}

impl ByteInterval {
    fn from_slice(slice: &[u8]) -> Result<Self, ProcessingError> {
        Self::from_raw(slice.as_ptr(), slice.len())?
            .ok_or_else(|| ProcessingError::ResourceLimit("ICC profile interval is empty".into()))
    }

    fn from_raw(pointer: *const u8, length: usize) -> Result<Option<Self>, ProcessingError> {
        if length == 0 {
            return Ok(None);
        }
        let start = pointer as usize;
        let end = start.checked_add(length).ok_or_else(|| {
            ProcessingError::ResourceLimit("ICC profile interval overflows".into())
        })?;
        Ok(Some(Self { start, end }))
    }

    fn contains(self, start: usize, end: usize) -> bool {
        self.start <= start && end <= self.end
    }
}

fn allocate_samples(
    channels: usize,
    pixels: usize,
    allocation: &mut ConstructionLedger,
) -> std::result::Result<Vec<Vec<f32>>, ProcessingError> {
    let mut result = allocation.try_new_reserved_vec_with(channels, |count| {
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|error| {
            ProcessingError::Allocation(format!("ICC output allocation failed: {error}"))
        })?;
        Ok(values)
    })?;
    result.resize_with(channels, Vec::new);
    for slot in &mut result {
        *slot = allocate_one_samples(pixels, allocation)?;
    }
    Ok(result)
}

fn allocate_one_samples(
    pixels: usize,
    allocation: &mut ConstructionLedger,
) -> std::result::Result<Vec<f32>, ProcessingError> {
    let mut values = allocation.try_new_reserved_vec_with(pixels, |count| {
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|error| {
            ProcessingError::Allocation(format!("ICC output allocation failed: {error}"))
        })?;
        Ok(values)
    })?;
    values.resize(pixels, 0.0);
    Ok(values)
}

fn transform_pixels(
    source: &ImageFrame,
    source_channels: usize,
    destination_channels: usize,
    transform: &Transform,
    output: &mut [Vec<f32>],
) -> std::result::Result<(), ProcessingError> {
    let planes = source
        .pixels()
        .f32_planes()
        .expect("validated source F32 planes");
    let mut input = [0.0f32; 3];
    let mut converted = [0.0f32; 3];
    for y in 0..source.descriptor().height() as usize {
        for x in 0..source.descriptor().width() as usize {
            for (channel, role) in color_roles(source_channels).iter().enumerate() {
                let index = source
                    .descriptor()
                    .planes()
                    .iter()
                    .position(|plane| plane.roles() == [*role])
                    .expect("validated source color role");
                input[channel] = sample_at(&planes[index], x, y)?;
            }
            transform
                .transform_f32(
                    &input[..source_channels],
                    &mut converted[..destination_channels],
                )
                .map_err(map_icc_error)?;
            let pixel = y
                .checked_mul(source.descriptor().width() as usize)
                .and_then(|offset| offset.checked_add(x))
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("ICC pixel offset overflows".into())
                })?;
            for channel in 0..destination_channels {
                output[channel][pixel] = converted[channel];
            }
        }
    }
    Ok(())
}

fn copy_alpha_samples(
    source: &Plane<f32>,
    destination: &mut Vec<f32>,
) -> std::result::Result<(), ProcessingError> {
    let width = source.layout().width() as usize;
    let height = source.layout().height() as usize;
    if destination.len()
        != width
            .checked_mul(height)
            .ok_or_else(|| ProcessingError::ResourceLimit("alpha pixel count overflows".into()))?
    {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidLayout("alpha output shape is inconsistent".into()),
        ));
    }
    for y in 0..height {
        for x in 0..width {
            destination[y * width + x] = sample_at(source, x, y)?;
        }
    }
    Ok(())
}

fn sample_at(plane: &Plane<f32>, x: usize, y: usize) -> std::result::Result<f32, ProcessingError> {
    let layout = plane.layout();
    let base = y
        .checked_mul(layout.row_stride())
        .and_then(|offset| offset.checked_add(x.checked_mul(layout.pixel_stride())?))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC sample offset overflows".into()))?;
    let offset = *layout.channel_offsets().first().ok_or_else(|| {
        ProcessingError::Invalid(super::HighresError::InvalidLayout(
            "ICC plane has no channel offset".into(),
        ))
    })?;
    let value =
        *plane
            .samples()
            .get(base.checked_add(offset).ok_or_else(|| {
                ProcessingError::ResourceLimit("ICC sample offset overflows".into())
            })?)
            .ok_or_else(|| {
                ProcessingError::Invalid(super::HighresError::InvalidLayout(
                    "ICC plane sample storage is truncated".into(),
                ))
            })?;
    if !value.is_finite() {
        return Err(ProcessingError::Invalid(
            super::HighresError::InvalidSamples("ICC source sample is not finite".into()),
        ));
    }
    Ok(value)
}

const fn color_roles(channels: usize) -> &'static [ChannelRole] {
    if channels == 1 {
        &[ChannelRole::Gray]
    } else {
        &[ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
    }
}

fn destination_color_information(
    profile: &[u8],
    color_type: IccColorType,
    allocation: &mut ConstructionLedger,
) -> std::result::Result<ColorInformationSet, ProcessingError> {
    let max_icc_bytes = allocation.max_icc_bytes();
    let mut owned = allocation.try_new_metadata_vec_with_limit(
        profile.len(),
        Some(max_icc_bytes),
        |count| {
            let mut values = Vec::new();
            values.try_reserve_exact(count).map_err(|error| {
                ProcessingError::Allocation(format!(
                    "ICC destination profile allocation failed: {error}"
                ))
            })?;
            Ok(values)
        },
    )?;
    owned.extend_from_slice(profile);
    ColorInformationSet::new()
        .with_icc_profile(owned)
        .map(|color| color.with_icc_color_type(color_type))
        .map_err(ProcessingError::from)
}

fn output_descriptor(
    width: u32,
    height: u32,
    channels: usize,
    has_alpha: bool,
    color: ColorInformationSet,
) -> std::result::Result<ImageDescriptor, ProcessingError> {
    let mut descriptor = if channels == 1 {
        ImageDescriptor::gray(width, height, 32)
    } else {
        ImageDescriptor::rgb(width, height, 32)
    }
    .map_err(ProcessingError::from)?;
    if has_alpha {
        let layout =
            PlaneLayout::planar(width, height, Subsampling::FULL).map_err(ProcessingError::from)?;
        let alpha = PlaneDescriptor::planar(layout, ChannelRole::Alpha, 32)
            .map_err(ProcessingError::from)?;
        let mut planes = descriptor.planes().to_vec();
        planes.try_reserve_exact(1).map_err(|error| {
            ProcessingError::Allocation(format!("ICC descriptor allocation failed: {error}"))
        })?;
        planes.push(alpha);
        descriptor = ImageDescriptor::new(width, height, descriptor.model(), planes)
            .map_err(ProcessingError::from)?;
    }
    descriptor = descriptor
        .with_alpha(if has_alpha {
            AlphaAssociation::Straight
        } else {
            AlphaAssociation::None
        })
        .map_err(ProcessingError::from)?
        .with_color_information(color)
        .with_domain(SampleDomain::Encoded);
    Ok(descriptor)
}

fn output_pixel_buffer(
    width: u32,
    height: u32,
    channels: usize,
    samples: Vec<Vec<f32>>,
    alpha: Option<Vec<f32>>,
) -> std::result::Result<PixelBuffer, ProcessingError> {
    let mut planes = Vec::new();
    planes
        .try_reserve_exact(channels + usize::from(alpha.is_some()))
        .map_err(|error| {
            ProcessingError::Allocation(format!("ICC output plane allocation failed: {error}"))
        })?;
    for values in samples {
        let layout =
            PlaneLayout::planar(width, height, Subsampling::FULL).map_err(ProcessingError::from)?;
        planes.push(Plane::new(layout, values).map_err(ProcessingError::from)?);
    }
    if let Some(values) = alpha {
        let layout =
            PlaneLayout::planar(width, height, Subsampling::FULL).map_err(ProcessingError::from)?;
        planes.push(Plane::new(layout, values).map_err(ProcessingError::from)?);
    }
    PixelBuffer::f32(planes).map_err(ProcessingError::from)
}

fn estimated_output_bytes(
    source: &ImageFrame,
    destination_profile_bytes: usize,
    source_profile_bytes: usize,
    plane_count: usize,
    channels: usize,
    pixels: usize,
) -> std::result::Result<usize, ProcessingError> {
    let metadata_bytes = source
        .metadata()
        .fresh_clone_retained_bytes()
        .map_err(ProcessingError::from)?;
    let sample_planes = channels
        .checked_add(usize::from(plane_count > channels))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output plane count overflows".into()))?;
    let samples = pixels
        .checked_mul(sample_planes)
        .and_then(|count| count.checked_mul(size_of::<f32>()))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output samples overflow".into()))?;
    let mut total = size_of::<ImageFrame>()
        .checked_add(size_of::<ImageDescriptor>())
        .and_then(|value| value.checked_add(size_of::<PixelBuffer>()))
        .and_then(|value| value.checked_add(size_of::<ColorInformationSet>()))
        .and_then(|value| value.checked_add(source_profile_bytes))
        .and_then(|value| value.checked_add(destination_profile_bytes))
        .and_then(|value| value.checked_add(metadata_bytes))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output metadata overflows".into()))?;
    let descriptor_bytes = plane_count
        .checked_mul(
            size_of::<PlaneDescriptor>()
                .checked_add(size_of::<PlaneLayout>())
                .and_then(|value| value.checked_add(size_of::<ChannelRole>()))
                .and_then(|value| value.checked_add(size_of::<usize>()))
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("ICC descriptor bytes overflow".into())
                })?,
        )
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC descriptor bytes overflow".into()))?;
    let pixel_bytes = plane_count
        .checked_mul(size_of::<Plane<f32>>() + size_of::<PlaneLayout>() + size_of::<usize>())
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC pixel headers overflow".into()))?;
    total = total
        .checked_add(descriptor_bytes)
        .and_then(|value| value.checked_add(pixel_bytes))
        .and_then(|value| value.checked_add(samples))
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC output bytes overflow".into()))?;
    Ok(total)
}

struct LedgerMetadataSink<'a> {
    allocation: &'a mut ConstructionLedger,
}

impl OwnerSink for LedgerMetadataSink<'_> {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> std::result::Result<Vec<T>, ProcessingError> {
        let limit = matches!(
            key,
            OwnerKey::IccProfile | OwnerKey::IccSourceProfile | OwnerKey::IccDestinationProfile
        )
        .then(|| self.allocation.max_icc_bytes());
        self.allocation
            .try_new_metadata_vec_with_limit(count, limit, |count| {
                let mut values = Vec::new();
                values.try_reserve_exact(count).map_err(|error| {
                    ProcessingError::Allocation(format!("ICC metadata allocation failed: {error}"))
                })?;
                Ok(values)
            })
    }

    fn commit_inline(
        &mut self,
        _key: OwnerKey,
        bytes: usize,
    ) -> std::result::Result<(), ProcessingError> {
        self.allocation.commit_pending_inline(bytes, bytes)
    }
}

fn map_icc_error(error: icc_profile::TransformError) -> ProcessingError {
    match error {
        icc_profile::TransformError::ResourceLimit(message) => {
            ProcessingError::ResourceLimit(message.into())
        }
        icc_profile::TransformError::UnsupportedProfileFeature(message) => {
            ProcessingError::Unsupported(message.into())
        }
        other => ProcessingError::Conversion(other.to_string()),
    }
}
