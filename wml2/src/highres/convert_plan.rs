use super::super::domain::{RgbPrimaries, SampleDomain};
use super::super::metadata::{Av1ColorInformation, IccColorType, NclxColorInformation};
use super::super::processing::ProcessingError;
use super::super::types::{AlphaAssociation, ChannelModel, ImageFrame, PixelFormat, Subsampling};
use super::super::{HighresError, ResourceLimits};
use super::options::{
    AlphaPolicy, ChromaLocation, ColorConvertOptions, Destination, MatrixCoefficients,
    NativeSampleEncoding, RenderingIntent, SampleRange, SourceInterpretation, WhiteAdaptation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceRoute<'a> {
    Icc {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    Cicp {
        color: NclxColorInformation,
    },
    Linear {
        domain: SampleDomain,
        primaries: RgbPrimaries,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DestinationRoute<'a> {
    IccGray {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    IccRgb {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    CicpRgb {
        color: NclxColorInformation,
    },
    LinearRgb {
        domain: SampleDomain,
        primaries: RgbPrimaries,
    },
}

/// Borrowed conversion facts; no output buffers or CMS state are compiled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConversionPlan<'a> {
    source_route: SourceRoute<'a>,
    destination_route: DestinationRoute<'a>,
    native_sample_encoding: Option<NativeSampleEncoding>,
    estimated_live_bytes: usize,
}

impl<'a> ConversionPlan<'a> {
    pub(crate) const fn native_sample_encoding(&self) -> Option<NativeSampleEncoding> {
        self.native_sample_encoding
    }

    pub(crate) const fn source_route(&self) -> SourceRoute<'a> {
        self.source_route
    }

    pub(crate) const fn destination_route(&self) -> DestinationRoute<'a> {
        self.destination_route
    }

    pub(crate) const fn estimated_live_bytes(&self) -> usize {
        self.estimated_live_bytes
    }

    pub(crate) fn inspect(
        frame: &'a ImageFrame,
        options: &ColorConvertOptions<'a>,
        limits: &ResourceLimits,
    ) -> std::result::Result<Self, ProcessingError> {
        frame.validate_with_limits(limits)?;
        frame.descriptor().domain().require_explicit()?;
        validate_selected_source_profile_limit(frame, options.source(), limits)?;
        validate_destination_profile_size(options.destination(), limits)?;
        let source_route = resolve_source(
            frame,
            options.source(),
            options.native_sample_encoding().is_some(),
        )?;
        validate_source_transfer_domain(frame, source_route)?;
        validate_rendering_intent(
            options.rendering_intent(),
            source_route,
            options.destination(),
        )?;
        let native_sample_encoding = resolve_native_encoding(
            frame,
            options.native_sample_encoding(),
            frame.descriptor().color_information().nclx(),
            frame.descriptor().color_information().av1(),
            frame.metadata().av1_description(),
        )?;
        let destination_route = validate_destination(options.destination())?;
        validate_alpha_policy(frame, options.alpha_policy(), destination_route)?;
        validate_white_adaptation(options.white_adaptation(), source_route, destination_route)?;
        validate_domain_route(frame, source_route, destination_route)?;

        let descriptor_bytes = frame
            .descriptor()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let pixel_bytes = frame
            .pixels()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let metadata_bytes = frame
            .metadata()
            .metadata_bytes()
            .map_err(ProcessingError::from)?;
        let source_bytes = descriptor_bytes
            .checked_add(pixel_bytes)
            .and_then(|bytes| bytes.checked_add(metadata_bytes))
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("conversion ownership overflows".into())
            })?;
        // Count visible borrowed ICC ranges once; an active profile is already
        // included in the frame ownership measured above.
        let profile_bytes = borrowed_profile_bytes(frame, source_route, destination_route)?;
        let estimated_live_bytes = source_bytes.checked_add(profile_bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("conversion ownership overflows".into())
        })?;
        if estimated_live_bytes > limits.max_total_live_decoded_bytes {
            return Err(ProcessingError::ResourceLimit(
                "conversion ownership exceeds live-byte limit".into(),
            ));
        }
        Ok(Self {
            source_route,
            destination_route,
            native_sample_encoding,
            estimated_live_bytes,
        })
    }
}

fn resolve_source<'a>(
    frame: &'a ImageFrame,
    source: SourceInterpretation<'a>,
    explicit_native_encoding: bool,
) -> std::result::Result<SourceRoute<'a>, ProcessingError> {
    match source {
        SourceInterpretation::Icc {
            profile,
            color_type,
        } => {
            validate_profile(profile, expected_source_space(frame))?;
            Ok(SourceRoute::Icc {
                profile,
                color_type,
            })
        }
        SourceInterpretation::Cicp(color) => {
            validate_source_cicp(
                color,
                explicit_native_encoding || frame.descriptor().model() == ChannelModel::Gray,
            )?;
            Ok(SourceRoute::Cicp { color })
        }
        SourceInterpretation::ActiveMetadata => {
            let active = frame.descriptor().color_information();
            match (active.icc_profile(), active.icc_color_type()) {
                (Some(profile), Some(color_type)) => {
                    validate_profile(profile, expected_source_space(frame))?;
                    Ok(SourceRoute::Icc {
                        profile,
                        color_type,
                    })
                }
                (Some(_), None) | (None, Some(_)) => {
                    Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                        "active ICC profile and colour type must be supplied together".into(),
                    )))
                }
                (None, None) => {
                    if let Some(color) = active.nclx() {
                        validate_source_cicp(
                            color,
                            explicit_native_encoding
                                || frame.descriptor().model() == ChannelModel::Gray,
                        )?;
                        Ok(SourceRoute::Cicp { color })
                    } else if let Some(color) = active.av1() {
                        let color = nclx_from_av1(color);
                        validate_source_cicp(
                            color,
                            explicit_native_encoding
                                || frame.descriptor().model() == ChannelModel::Gray,
                        )?;
                        Ok(SourceRoute::Cicp { color })
                    } else if let (
                        domain @ (SampleDomain::LinearRelative
                        | SampleDomain::LinearAbsoluteNits
                        | SampleDomain::HlgSceneLinear
                        | SampleDomain::HlgDisplayLinear(_)),
                        Some(primaries),
                    ) = (frame.descriptor().domain(), frame.descriptor().primaries())
                    {
                        Ok(SourceRoute::Linear { domain, primaries })
                    } else {
                        Err(ProcessingError::Unsupported(
                            "active metadata has no ICC, CICP, or explicit linear interpretation"
                                .into(),
                        ))
                    }
                }
            }
        }
    }
}

fn validate_source_transfer_domain(
    frame: &ImageFrame,
    source: SourceRoute<'_>,
) -> std::result::Result<(), ProcessingError> {
    let domain = frame.descriptor().domain();
    if matches!(source, SourceRoute::Icc { .. })
        && matches!(
            domain,
            SampleDomain::LinearRelative
                | SampleDomain::LinearAbsoluteNits
                | SampleDomain::HlgSceneLinear
                | SampleDomain::HlgDisplayLinear(_)
        )
    {
        return Err(ProcessingError::Unsupported(
            "an ICC transfer must not be applied to already-linear samples".into(),
        ));
    }
    if let SourceRoute::Cicp { color } = source
        && matches!(
            domain,
            SampleDomain::LinearRelative
                | SampleDomain::LinearAbsoluteNits
                | SampleDomain::HlgSceneLinear
                | SampleDomain::HlgDisplayLinear(_)
        )
        && color.transfer() != 8
    {
        return Err(ProcessingError::Unsupported(
            "a non-linear CICP transfer must not be applied to linear samples".into(),
        ));
    }
    Ok(())
}

fn validate_profile(
    profile: &[u8],
    expected_space: &[u8; 4],
) -> std::result::Result<(), ProcessingError> {
    if profile.len() < 132 {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC profile header is truncated".into(),
        )));
    }
    let declared_size = u32::from_be_bytes(profile[0..4].try_into().unwrap()) as usize;
    if declared_size < 132 || declared_size > profile.len() {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC profile declared size is invalid".into(),
        )));
    }
    if &profile[36..40] != b"acsp" {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC profile signature is invalid".into(),
        )));
    }
    if !matches!(profile[8], 2 | 4) {
        return Err(ProcessingError::Unsupported(
            "ICC profile major version is unsupported".into(),
        ));
    }
    if &profile[16..20] != expected_space {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC profile channel space does not match the frame route".into(),
        )));
    }
    let tag_count = u32::from_be_bytes(profile[128..132].try_into().unwrap()) as usize;
    let table_bytes = tag_count.checked_mul(12).ok_or_else(|| {
        ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC tag table size overflows".into(),
        ))
    })?;
    let table_end = 132usize.checked_add(table_bytes).ok_or_else(|| {
        ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC tag table offset overflows".into(),
        ))
    })?;
    if table_end > declared_size {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "ICC tag table exceeds declared profile size".into(),
        )));
    }
    for record in profile[132..table_end].chunks_exact(12) {
        let offset = u32::from_be_bytes(record[4..8].try_into().unwrap()) as usize;
        let size = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
        let end = offset.checked_add(size).ok_or_else(|| {
            ProcessingError::Invalid(HighresError::InvalidMetadata(
                "ICC tag range overflows".into(),
            ))
        })?;
        if offset < table_end || end > declared_size {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "ICC tag range exceeds declared profile size".into(),
            )));
        }
    }
    Ok(())
}

fn expected_source_space(frame: &ImageFrame) -> &'static [u8; 4] {
    match frame.descriptor().model() {
        ChannelModel::Gray => b"GRAY",
        ChannelModel::RGB | ChannelModel::YCbCr => b"RGB ",
    }
}

fn validate_selected_source_profile_limit(
    frame: &ImageFrame,
    source: SourceInterpretation<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<(), ProcessingError> {
    let profile = match source {
        SourceInterpretation::Icc { profile, .. } => Some(profile),
        SourceInterpretation::Cicp(_) => None,
        SourceInterpretation::ActiveMetadata => {
            frame.descriptor().color_information().icc_profile()
        }
    };
    if let Some(profile) = profile
        && profile.len() > limits.max_icc_bytes
    {
        return Err(ProcessingError::ResourceLimit(
            "source ICC profile exceeds the configured limit".into(),
        ));
    }
    Ok(())
}

fn validate_destination(
    destination: Destination<'_>,
) -> std::result::Result<DestinationRoute<'_>, ProcessingError> {
    match destination {
        Destination::IccGray {
            profile,
            color_type,
        } => {
            validate_profile(profile, b"GRAY")?;
            Ok(DestinationRoute::IccGray {
                profile,
                color_type,
            })
        }
        Destination::IccRgb {
            profile,
            color_type,
        } => {
            validate_profile(profile, b"RGB ")?;
            Ok(DestinationRoute::IccRgb {
                profile,
                color_type,
            })
        }
        Destination::EncodedCicpRgb { color } => {
            validate_cicp(color)?;
            Ok(DestinationRoute::CicpRgb { color })
        }
        Destination::LinearRgb { domain, primaries } => {
            validate_linear_domain(domain)?;
            Ok(DestinationRoute::LinearRgb { domain, primaries })
        }
    }
}

fn validate_linear_domain(domain: SampleDomain) -> std::result::Result<(), ProcessingError> {
    match domain {
        SampleDomain::LinearRelative
        | SampleDomain::LinearAbsoluteNits
        | SampleDomain::HlgSceneLinear
        | SampleDomain::HlgDisplayLinear(_) => Ok(()),
        SampleDomain::Unknown => Err(ProcessingError::Unsupported(
            "linear destination requires an explicit sample domain".into(),
        )),
        SampleDomain::Encoded => Err(ProcessingError::Unsupported(
            "encoded destination needs ICC or CICP colour information".into(),
        )),
    }
}

fn validate_rendering_intent(
    intent: RenderingIntent,
    source: SourceRoute<'_>,
    destination: Destination<'_>,
) -> std::result::Result<(), ProcessingError> {
    if !matches!(intent, RenderingIntent::RelativeColorimetric)
        && (matches!(source, SourceRoute::Cicp { .. })
            || matches!(destination, Destination::EncodedCicpRgb { .. }))
    {
        return Err(ProcessingError::Unsupported(
            "only relative colorimetric CICP conversion is supported in this slice".into(),
        ));
    }
    Ok(())
}

fn validate_white_adaptation(
    adaptation: WhiteAdaptation,
    source: SourceRoute<'_>,
    destination: DestinationRoute<'_>,
) -> std::result::Result<(), ProcessingError> {
    if matches!(adaptation, WhiteAdaptation::Bradford)
        && matches!(source, SourceRoute::Cicp { .. })
        && matches!(destination, DestinationRoute::LinearRgb { .. })
    {
        return Err(ProcessingError::Unsupported(
            "CICP white adaptation is not implemented in this slice".into(),
        ));
    }
    if matches!(adaptation, WhiteAdaptation::RequireSameWhite)
        && let (Some(source), Some(destination)) =
            (source_white(source), destination_white(destination))
        && source != destination
    {
        return Err(ProcessingError::Unsupported(
            "source and destination white points differ and no adaptation was selected".into(),
        ));
    }
    Ok(())
}

fn source_white(source: SourceRoute<'_>) -> Option<(f32, f32)> {
    match source {
        SourceRoute::Cicp { color } => cicp_white(color.primaries()),
        SourceRoute::Linear { primaries, .. } => Some(primaries.white()),
        SourceRoute::Icc { .. } => None,
    }
}

fn destination_white(destination: DestinationRoute<'_>) -> Option<(f32, f32)> {
    match destination {
        DestinationRoute::LinearRgb { primaries, .. } => Some(primaries.white()),
        DestinationRoute::CicpRgb { color } => cicp_white(color.primaries()),
        DestinationRoute::IccGray { .. } | DestinationRoute::IccRgb { .. } => None,
    }
}

fn cicp_white(primaries: u16) -> Option<(f32, f32)> {
    match primaries {
        // The supported CICP RGB primary sets in this slice use D65.
        1 | 9 | 12 => Some((0.3127, 0.3290)),
        _ => None,
    }
}

fn validate_alpha_policy(
    frame: &ImageFrame,
    policy: AlphaPolicy,
    destination: DestinationRoute<'_>,
) -> std::result::Result<(), ProcessingError> {
    if !matches!(policy, AlphaPolicy::PreserveAssociation) {
        return Err(ProcessingError::Unsupported(
            "changing alpha association is deferred to the execution slice".into(),
        ));
    }
    if frame.descriptor().alpha() == AlphaAssociation::Premultiplied
        && (frame.descriptor().domain() == SampleDomain::Encoded
            || (matches!(
                frame.descriptor().domain(),
                SampleDomain::LinearRelative
                    | SampleDomain::LinearAbsoluteNits
                    | SampleDomain::HlgSceneLinear
                    | SampleDomain::HlgDisplayLinear(_)
            ) && matches!(
                destination,
                DestinationRoute::IccGray { .. }
                    | DestinationRoute::IccRgb { .. }
                    | DestinationRoute::CicpRgb { .. }
            )))
    {
        return Err(ProcessingError::Unsupported(
            "premultiplied non-linear alpha conversion is unsupported".into(),
        ));
    }
    Ok(())
}

fn validate_domain_route(
    frame: &ImageFrame,
    source: SourceRoute<'_>,
    destination: DestinationRoute<'_>,
) -> std::result::Result<(), ProcessingError> {
    let source_domain = effective_source_domain(frame, source)?;
    let destination_domain = effective_destination_domain(destination)?;
    if domains_compatible(source_domain, destination_domain) {
        Ok(())
    } else {
        Err(ProcessingError::Unsupported(
            "source and destination sample domains need an explicit policy".into(),
        ))
    }
}

fn effective_source_domain(
    frame: &ImageFrame,
    source: SourceRoute<'_>,
) -> std::result::Result<SampleDomain, ProcessingError> {
    match source {
        // ICC colourants in this planning slice describe an encoded,
        // relative-light route. HDR ICC semantics need a later explicit CMS
        // policy and must not be inferred here.
        SourceRoute::Icc { .. } => Ok(SampleDomain::LinearRelative),
        SourceRoute::Cicp { color }
            if color.transfer() == 8
                && matches!(
                    frame.descriptor().domain(),
                    SampleDomain::LinearRelative
                        | SampleDomain::LinearAbsoluteNits
                        | SampleDomain::HlgSceneLinear
                        | SampleDomain::HlgDisplayLinear(_)
                ) =>
        {
            // A linear-sample descriptor is authoritative for the unit when
            // an explicit CICP route carries the identity transfer (TC=8).
            Ok(frame.descriptor().domain())
        }
        SourceRoute::Cicp { color } => cicp_domain(color.transfer()),
        SourceRoute::Linear { domain, .. } => Ok(domain),
    }
}

fn effective_destination_domain(
    destination: DestinationRoute<'_>,
) -> std::result::Result<SampleDomain, ProcessingError> {
    match destination {
        DestinationRoute::IccGray { .. } | DestinationRoute::IccRgb { .. } => {
            Ok(SampleDomain::LinearRelative)
        }
        DestinationRoute::CicpRgb { color } => cicp_domain(color.transfer()),
        DestinationRoute::LinearRgb { domain, .. } => Ok(domain),
    }
}

fn domains_compatible(source: SampleDomain, destination: SampleDomain) -> bool {
    match (source, destination) {
        (SampleDomain::LinearRelative, SampleDomain::LinearRelative) => true,
        (SampleDomain::LinearAbsoluteNits, SampleDomain::LinearAbsoluteNits) => true,
        (
            SampleDomain::HlgSceneLinear,
            SampleDomain::HlgSceneLinear | SampleDomain::HlgDisplayLinear(_),
        ) => true,
        (SampleDomain::HlgDisplayLinear(source), SampleDomain::HlgDisplayLinear(destination)) => {
            source == destination
        }
        _ => false,
    }
}

fn cicp_domain(transfer: u16) -> std::result::Result<SampleDomain, ProcessingError> {
    match transfer {
        1 | 6 | 13 | 14 | 15 => Ok(SampleDomain::LinearRelative),
        8 => Ok(SampleDomain::LinearRelative),
        16 => Ok(SampleDomain::LinearAbsoluteNits),
        18 => Ok(SampleDomain::HlgSceneLinear),
        _ => Err(ProcessingError::Unsupported(
            "CICP transfer has no supported sample-domain route".into(),
        )),
    }
}

fn resolve_native_encoding(
    frame: &ImageFrame,
    explicit: Option<NativeSampleEncoding>,
    active_nclx: Option<NclxColorInformation>,
    active_av1: Option<super::super::metadata::Av1ColorInformation>,
    av1_description: Option<super::super::metadata::Av1Description>,
) -> std::result::Result<Option<NativeSampleEncoding>, ProcessingError> {
    let model = frame.descriptor().model();
    let integer = matches!(frame.pixels().format(), PixelFormat::U8 | PixelFormat::U16);
    let has_explicit = explicit.is_some();
    let mut encoding = match (explicit, integer) {
        (Some(encoding), _) => encoding,
        (None, true) => {
            let (range, matrix) = active_native_fields(active_nclx, active_av1)?;
            NativeSampleEncoding::new(range, matrix, None)
        }
        (None, false) => return Ok(None),
    };
    if !has_explicit
        && model == ChannelModel::YCbCr
        && frame
            .descriptor()
            .planes()
            .iter()
            .any(|plane| plane.layout().subsampling() != Subsampling::FULL)
        && let Some(description) = av1_description
        && let Some(position) = description.chroma_sample_position()
    {
        encoding = NativeSampleEncoding::new(
            encoding.range(),
            encoding.matrix(),
            Some(
                ChromaLocation::av1_position(position)
                    .map_err(|message| ProcessingError::Unsupported(message.into()))?,
            ),
        );
    }
    match model {
        ChannelModel::YCbCr => match encoding.matrix() {
            MatrixCoefficients::Bt601 | MatrixCoefficients::Bt709 | MatrixCoefficients::Bt2020 => {}
            MatrixCoefficients::Identity => {
                return Err(ProcessingError::Unsupported(
                    "identity matrix is not valid for a YCbCr source".into(),
                ));
            }
            MatrixCoefficients::Unspecified(_) => {
                return Err(ProcessingError::Unsupported(
                    "YCbCr matrix coefficients must be explicit".into(),
                ));
            }
        },
        ChannelModel::Gray => {
            if encoding.chroma_location().is_some() {
                return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                    "Gray native encoding must not carry chroma location".into(),
                )));
            }
        }
        ChannelModel::RGB => {
            if !matches!(encoding.matrix(), MatrixCoefficients::Identity)
                || encoding.chroma_location().is_some()
            {
                return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                    "Gray/RGB native encoding requires identity matrix and no chroma location"
                        .into(),
                )));
            }
        }
    }
    if model == ChannelModel::YCbCr
        && frame
            .descriptor()
            .planes()
            .iter()
            .any(|plane| plane.layout().subsampling() != Subsampling::FULL)
        && encoding.chroma_location().is_none()
    {
        return Err(ProcessingError::Unsupported(
            "subsampled YCbCr requires an explicit chroma location".into(),
        ));
    }
    if let Some(color) = active_nclx {
        if color.full_range() != matches!(encoding.range(), SampleRange::Full) {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "native range conflicts with active CICP range".into(),
            )));
        }
        let matrix_matches =
            native_matrix_matches(encoding.matrix(), color.matrix(), model, has_explicit);
        if !matrix_matches {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "native matrix conflicts with active CICP matrix".into(),
            )));
        }
    }
    if let Some(color) = active_av1 {
        if color.full_range() != matches!(encoding.range(), SampleRange::Full) {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "native range conflicts with active AV1 range".into(),
            )));
        }
        let matrix_matches =
            native_matrix_matches(encoding.matrix(), color.matrix(), model, has_explicit);
        if !matrix_matches {
            return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
                "native matrix conflicts with active AV1 matrix".into(),
            )));
        }
    }
    if let Some(description) = av1_description
        && let Some(full_range) = description.full_range()
        && full_range != matches!(encoding.range(), SampleRange::Full)
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "native range conflicts with AV1 sequence range".into(),
        )));
    }
    Ok(Some(encoding))
}

fn active_native_fields(
    nclx: Option<NclxColorInformation>,
    av1: Option<super::super::metadata::Av1ColorInformation>,
) -> std::result::Result<(SampleRange, MatrixCoefficients), ProcessingError> {
    let nclx_fields = nclx.map(|color| (color.full_range(), color.matrix()));
    let av1_fields = av1.map(|color| (color.full_range(), color.matrix()));
    if let (Some(left), Some(right)) = (nclx_fields, av1_fields)
        && left != right
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "active nclx and AV1 native signalling disagree".into(),
        )));
    }
    let (full_range, matrix) = nclx_fields.or(av1_fields).ok_or_else(|| {
        ProcessingError::Unsupported(
            "native integer samples require known active range and matrix".into(),
        )
    })?;
    let matrix = match matrix {
        0 => MatrixCoefficients::Identity,
        1 => MatrixCoefficients::Bt709,
        5 | 6 => MatrixCoefficients::Bt601,
        9 => MatrixCoefficients::Bt2020,
        2 => MatrixCoefficients::Unspecified(2),
        _ => {
            return Err(ProcessingError::Unsupported(
                "active native matrix is unsupported".into(),
            ));
        }
    };
    Ok((
        if full_range {
            SampleRange::Full
        } else {
            SampleRange::Limited
        },
        matrix,
    ))
}

fn native_matrix_matches(
    encoding: MatrixCoefficients,
    signaling_matrix: u16,
    model: ChannelModel,
    explicit: bool,
) -> bool {
    if signaling_matrix == 2 {
        // Matrix 2 is CICP's unspecified value. It may be completed by an
        // explicit native description, but must not be silently inferred.
        return model == ChannelModel::Gray || explicit;
    }
    match encoding {
        MatrixCoefficients::Identity => signaling_matrix == 0,
        MatrixCoefficients::Bt601 => matches!(signaling_matrix, 5 | 6),
        MatrixCoefficients::Bt709 => signaling_matrix == 1,
        MatrixCoefficients::Bt2020 => signaling_matrix == 9,
        MatrixCoefficients::Unspecified(_) => model == ChannelModel::Gray,
    }
}

fn validate_cicp(color: NclxColorInformation) -> std::result::Result<(), ProcessingError> {
    validate_cicp_primary_transfer(color)?;
    if !matches!(color.matrix(), 0 | 1 | 5 | 6 | 9) {
        return Err(ProcessingError::Unsupported(
            "CICP matrix coefficients are not implemented in this slice".into(),
        ));
    }
    Ok(())
}

fn validate_cicp_primary_transfer(
    color: NclxColorInformation,
) -> std::result::Result<(), ProcessingError> {
    if !matches!(color.primaries(), 1 | 9 | 12) {
        return Err(ProcessingError::Unsupported(
            "CICP primaries are not implemented in this slice".into(),
        ));
    }
    if !matches!(color.transfer(), 1 | 6 | 8 | 13 | 14 | 15 | 16 | 18) {
        return Err(ProcessingError::Unsupported(
            "CICP transfer function is not implemented in this slice".into(),
        ));
    }
    Ok(())
}

fn validate_source_cicp(
    color: NclxColorInformation,
    explicit_native_encoding: bool,
) -> std::result::Result<(), ProcessingError> {
    validate_cicp_primary_transfer(color)?;
    if color.matrix() == 2 && explicit_native_encoding {
        return Ok(());
    }
    if !matches!(color.matrix(), 0 | 1 | 5 | 6 | 9) {
        return Err(ProcessingError::Unsupported(
            "CICP matrix coefficients are not implemented in this slice".into(),
        ));
    }
    Ok(())
}

fn nclx_from_av1(color: Av1ColorInformation) -> NclxColorInformation {
    NclxColorInformation::new(
        color.primaries(),
        color.transfer(),
        color.matrix(),
        color.full_range(),
    )
}

fn validate_destination_profile_size(
    destination: Destination<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<(), ProcessingError> {
    let profile = match destination {
        Destination::IccGray { profile, .. } | Destination::IccRgb { profile, .. } => profile,
        Destination::EncodedCicpRgb { .. } | Destination::LinearRgb { .. } => return Ok(()),
    };
    if profile.len() > limits.max_icc_bytes {
        return Err(ProcessingError::ResourceLimit(
            "destination ICC profile exceeds the configured limit".into(),
        ));
    }
    Ok(())
}

fn borrowed_profile_bytes(
    frame: &ImageFrame,
    source: SourceRoute<'_>,
    destination: DestinationRoute<'_>,
) -> std::result::Result<usize, ProcessingError> {
    let owned = [
        frame.descriptor().color_information().icc_profile(),
        frame.metadata().source_color().icc_profile(),
    ];
    let source = match source {
        SourceRoute::Icc { profile, .. } => Some(profile),
        SourceRoute::Cicp { .. } | SourceRoute::Linear { .. } => None,
    };
    let destination = match destination {
        DestinationRoute::IccGray { profile, .. } | DestinationRoute::IccRgb { profile, .. } => {
            Some(profile)
        }
        DestinationRoute::CicpRgb { .. } | DestinationRoute::LinearRgb { .. } => None,
    };
    let mut total = 0usize;
    let mut covered = [None; 4];
    let mut covered_count = 0usize;
    for profile in owned.into_iter().flatten() {
        covered[covered_count] = Some(byte_range(profile)?);
        covered_count += 1;
    }
    for candidate in [source, destination].into_iter().flatten() {
        let range = byte_range(candidate)?;
        let uncovered = uncovered_bytes(candidate.len(), range, &covered[..covered_count])?;
        total = total.checked_add(uncovered).ok_or_else(|| {
            ProcessingError::ResourceLimit("borrowed ICC byte count overflows".into())
        })?;
        covered[covered_count] = Some(range);
        covered_count += 1;
    }
    Ok(total)
}

#[derive(Clone, Copy)]
struct ByteRange {
    start: usize,
    end: usize,
}

fn byte_range(bytes: &[u8]) -> std::result::Result<ByteRange, ProcessingError> {
    let start = bytes.as_ptr() as usize;
    let end = start
        .checked_add(bytes.len())
        .ok_or_else(|| ProcessingError::ResourceLimit("ICC byte range address overflows".into()))?;
    Ok(ByteRange { start, end })
}

fn uncovered_bytes(
    candidate_len: usize,
    candidate: ByteRange,
    covered: &[Option<ByteRange>],
) -> std::result::Result<usize, ProcessingError> {
    let mut intersections: [Option<ByteRange>; 4] = [None; 4];
    let mut count = 0usize;
    for range in covered.iter().flatten() {
        let start = candidate.start.max(range.start);
        let end = candidate.end.min(range.end);
        if start < end {
            if count >= intersections.len() {
                return Err(ProcessingError::ResourceLimit(
                    "ICC overlap range accounting overflows".into(),
                ));
            }
            let mut index = count;
            while index > 0 {
                let previous = intersections[index - 1];
                if previous.is_none_or(|range| range.start <= start) {
                    break;
                }
                intersections[index] = intersections[index - 1];
                index -= 1;
            }
            intersections[index] = Some(ByteRange { start, end });
            count += 1;
        }
    }
    let mut covered_len = 0usize;
    let mut merged_end = candidate.start;
    for intersection in intersections[..count].iter().flatten() {
        if intersection.end > merged_end {
            let start = intersection.start.max(merged_end);
            covered_len = covered_len
                .checked_add(intersection.end - start)
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("ICC overlap byte count overflows".into())
                })?;
            merged_end = intersection.end;
        }
    }
    candidate_len.checked_sub(covered_len).ok_or_else(|| {
        ProcessingError::ResourceLimit("ICC overlap exceeds candidate length".into())
    })
}
