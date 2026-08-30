//! The first executable high-resolution conversion slice.
//!
//! This module intentionally contains no CMS or encoded-output path.  It
//! consumes the borrowed plan once and materializes only a full-resolution
//! planar linear RGB(A) frame.

use super::super::allocation::ConstructionLedger;
use super::super::domain::{RgbPrimaries, SampleDomain};
use super::super::metadata::{
    ConversionAlpha, ConversionDestination, ConversionIntent, ConversionPrimaries, ConversionSource,
    ConversionWhiteAdaptation, LastConversion, LastConversionParts, NativeInterpretation,
};
use super::super::output_plan::{
    AdmittedOutputPlan, CandidateMaker, OutputOwnershipPlan, OwnerElement, OwnerKey, OwnerSink,
};
use super::super::processing::ProcessingError;
use super::super::types::{
    AlphaAssociation, ChannelModel, ChannelRole, ImageDescriptor, ImageFrame, PixelBuffer, Plane,
    PlaneDescriptor, PlaneLayout,
};
use super::super::{HighresError, ResourceLimits};
use super::native::{NativeColor, NativePixel, NativePixelReader};
use super::options::{
    AlphaPolicy, ColorConvertOptions, RenderingIntent, SourceInterpretation, WhiteAdaptation,
};
use super::plan::{ConversionPlan, DestinationRoute, SourceRoute};

/// Convert one frame to full-resolution planar F32 linear RGB(A).
///
/// The route is deliberately small: CICP transfers 1, 6, 8, 13, 14 and 15,
/// or an already declared linear source, may be converted to a known D65
/// linear RGB destination. Supported HDR destinations are explicitly
/// `LinearAbsoluteNits` for PQ and `HlgSceneLinear` for HLG. ICC, encoded
/// destinations, quantization and association changes are rejected before
/// output buffers are allocated. Geometry and source metadata remain
/// unchanged.
pub fn convert_frame(
    source: &ImageFrame,
    options: &ColorConvertOptions<'_>,
    limits: &ResourceLimits,
) -> std::result::Result<ImageFrame, ProcessingError> {
    let mut prepared = PreparedConversion::new(source, options, limits)?;
    let mut maker = OrdinaryMaker;
    prepared.materialize_with(&mut maker)
}

struct OrdinaryMaker;

impl CandidateMaker for OrdinaryMaker {
    fn make<T: super::super::output_plan::OwnerElement>(
        &mut self,
        _key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|error| {
            ProcessingError::Allocation(format!("output owner allocation failed: {error}"))
        })?;
        Ok(values)
    }
}

struct ConversionSink<'p, 'a, M: CandidateMaker> {
    admitted: &'p mut AdmittedOutputPlan<'a>,
    maker: &'p mut M,
}

impl<M: CandidateMaker> OwnerSink for ConversionSink<'_, '_, M> {
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

struct PreparedConversion<'a> {
    context: MaterializeContext<'a>,
    admitted: AdmittedOutputPlan<'a>,
    published: bool,
}

impl<'a> PreparedConversion<'a> {
    fn new(
        source: &'a ImageFrame,
        options: &'a ColorConvertOptions<'a>,
        limits: &ResourceLimits,
    ) -> std::result::Result<Self, ProcessingError> {
        let plan = ConversionPlan::inspect(source, options, limits)?;
        validate_execution_route(source, options, &plan)?;
        if source.metadata().tags().capacity() != 0 {
            return Err(ProcessingError::Unsupported(
                "metadata tags with retained capacity are unsupported by this conversion slice"
                    .into(),
            ));
        }

        let reader = NativePixelReader::from_plan(source, plan)?;
        let transform = RelativeTransform::from_plan(source, plan)?;
        let last_conversion = last_conversion_record(options, plan, &transform);
        let width = source.descriptor().width();
        let height = source.descriptor().height();
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or_else(|| ProcessingError::ResourceLimit("output pixel count overflows".into()))?;
        let has_alpha = source.descriptor().alpha() != AlphaAssociation::None;
        let plane_count = if has_alpha { 4usize } else { 3usize };
        let output_plan = OutputOwnershipPlan::inspect(source, plane_count, pixel_count, limits)?;
        let ledger =
            ConstructionLedger::new_with_ownership(0, 0, plan.estimated_live_bytes(), limits)?;
        let admitted = output_plan.admit(ledger, source.metadata())?;
        Ok(Self {
            context: MaterializeContext {
                source,
                _options: options,
                _plan: plan,
                reader,
                width,
                height,
                pixel_count,
                has_alpha,
                plane_limit: limits.max_plane_bytes,
                destination_domain: transform.destination_domain,
                transform,
                last_conversion,
            },
            admitted,
            published: false,
        })
    }

    fn materialize_with<M: CandidateMaker>(
        &mut self,
        maker: &mut M,
    ) -> std::result::Result<ImageFrame, ProcessingError> {
        if self.published {
            return Err(ProcessingError::Unsupported(
                "a prepared conversion cannot be materialized twice after success".into(),
            ));
        }
        let checkpoint = self.admitted.checkpoint();
        match materialize_frame(&self.context, &mut self.admitted, maker) {
            Ok(frame) => match self.admitted.complete_for(&frame) {
                Ok(()) => {
                    self.published = true;
                    Ok(frame)
                }
                Err(error) => {
                    drop(frame);
                    #[cfg(test)]
                    super::super::allocation::observe_restore_boundary();
                    self.admitted.restore(checkpoint);
                    Err(error)
                }
            },
            Err(error) => {
                // `materialize_frame` owns all partial vectors in its locals; by
                // the time this branch runs they have been dropped. Restore the
                // admitted checkpoint only after that ownership has ended.
                #[cfg(test)]
                super::super::allocation::observe_restore_boundary();
                self.admitted.restore(checkpoint);
                Err(error)
            }
        }
    }
}

struct MaterializeContext<'a> {
    source: &'a ImageFrame,
    _options: &'a ColorConvertOptions<'a>,
    _plan: ConversionPlan<'a>,
    reader: NativePixelReader<'a>,
    width: u32,
    height: u32,
    pixel_count: usize,
    has_alpha: bool,
    plane_limit: usize,
    destination_domain: SampleDomain,
    transform: RelativeTransform,
    last_conversion: LastConversion,
}

fn materialize_frame(
    context: &MaterializeContext<'_>,
    admitted: &mut AdmittedOutputPlan,
    maker: &mut impl CandidateMaker,
) -> std::result::Result<ImageFrame, ProcessingError> {
    let MaterializeContext {
        source,
        reader,
        width,
        height,
        pixel_count,
        has_alpha,
        plane_limit,
        destination_domain,
        transform,
        last_conversion,
        ..
    } = context;
    let transform = *transform;
    let mut red =
        admitted.fresh_with_limit(OwnerKey::Sample(0), *pixel_count, *plane_limit, maker)?;
    let mut green =
        admitted.fresh_with_limit(OwnerKey::Sample(1), *pixel_count, *plane_limit, maker)?;
    let mut blue =
        admitted.fresh_with_limit(OwnerKey::Sample(2), *pixel_count, *plane_limit, maker)?;
    let mut alpha = has_alpha
        .then(|| admitted.fresh_with_limit(OwnerKey::Sample(3), *pixel_count, *plane_limit, maker))
        .transpose()?;

    for y in 0..*height {
        for x in 0..*width {
            let pixel = reader.pixel(x, y).map_err(ProcessingError::from)?;
            let converted = transform.convert(pixel)?;
            red.push(converted[0]);
            green.push(converted[1]);
            blue.push(converted[2]);
            if let (Some(samples), Some(value)) = (alpha.as_mut(), pixel.alpha) {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
                        "alpha sample is not finite or is outside [0, 1]".into(),
                    )));
                }
                samples.push(value);
            } else if *has_alpha {
                return Err(ProcessingError::Invalid(HighresError::InvalidLayout(
                    "alpha descriptor has no readable alpha sample".into(),
                )));
            }
        }
    }

    let plane_count = if *has_alpha { 4usize } else { 3usize };
    let mut descriptors = admitted.fresh_with(OwnerKey::DescriptorOuter, plane_count, maker)?;
    let mut pixel_planes = admitted.fresh_with(OwnerKey::PixelOuter, plane_count, maker)?;
    for (plane, (role, samples)) in [
        (ChannelRole::Red, red),
        (ChannelRole::Green, green),
        (ChannelRole::Blue, blue),
    ]
    .into_iter()
    .enumerate()
    {
        let (descriptor, pixel_plane) =
            output_plane(admitted, *width, *height, plane, role, samples, maker)?;
        descriptors.push(descriptor);
        pixel_planes.push(pixel_plane);
    }
    if let Some(samples) = alpha {
        let (descriptor, pixel_plane) = output_plane(
            admitted,
            *width,
            *height,
            3,
            ChannelRole::Alpha,
            samples,
            maker,
        )?;
        descriptors.push(descriptor);
        pixel_planes.push(pixel_plane);
    }
    let descriptor = ImageDescriptor::new(*width, *height, ChannelModel::RGB, descriptors)
        .map_err(ProcessingError::from)?
        .with_alpha(source.descriptor().alpha())
        .map_err(ProcessingError::from)?
        .with_domain(*destination_domain)
        .with_primaries(transform.destination_primaries);
    let pixels = PixelBuffer::f32(pixel_planes).map_err(ProcessingError::from)?;
    let mut metadata = {
        let mut sink = ConversionSink { admitted, maker };
        source
            .metadata()
            .try_clone_for_conversion_with_sink(&mut sink)?
    };
    metadata.set_last_conversion(*last_conversion);
    ImageFrame::from_parts(descriptor, pixels, metadata, source.timing())
        .map_err(ProcessingError::from)
}

fn output_plane(
    admitted: &mut AdmittedOutputPlan,
    width: u32,
    height: u32,
    plane: usize,
    role: ChannelRole,
    samples: Vec<f32>,
    maker: &mut impl CandidateMaker,
) -> std::result::Result<(PlaneDescriptor, Plane<f32>), ProcessingError> {
    let mut descriptor_offsets =
        admitted.fresh_with(OwnerKey::DescriptorLayout { plane }, 1, maker)?;
    descriptor_offsets.push(0);
    let descriptor_layout = PlaneLayout::new(
        width,
        height,
        usize::try_from(width).map_err(|_| {
            ProcessingError::ResourceLimit("output width exceeds platform limits".into())
        })?,
        1,
        descriptor_offsets,
        super::super::Subsampling::FULL,
    )
    .map_err(ProcessingError::from)?;
    let mut roles = admitted.fresh_with(OwnerKey::Role { plane }, 1, maker)?;
    roles.push(role);
    let descriptor =
        PlaneDescriptor::new(descriptor_layout, roles, 32).map_err(ProcessingError::from)?;
    let mut sample_offsets = admitted.fresh_with(OwnerKey::PixelLayout { plane }, 1, maker)?;
    sample_offsets.push(0);
    let sample_layout = PlaneLayout::new(
        width,
        height,
        usize::try_from(width).map_err(|_| {
            ProcessingError::ResourceLimit("output width exceeds platform limits".into())
        })?,
        1,
        sample_offsets,
        super::super::Subsampling::FULL,
    )
    .map_err(ProcessingError::from)?;
    let plane = Plane::new(sample_layout, samples).map_err(ProcessingError::from)?;
    Ok((descriptor, plane))
}

fn validate_execution_route(
    source: &ImageFrame,
    options: &ColorConvertOptions<'_>,
    plan: &ConversionPlan<'_>,
) -> std::result::Result<(), ProcessingError> {
    if options.white_adaptation() != WhiteAdaptation::RequireSameWhite {
        return Err(ProcessingError::Unsupported(
            "Bradford adaptation is deferred to the CMS execution slice".into(),
        ));
    }
    if options.rendering_intent() != RenderingIntent::RelativeColorimetric {
        return Err(ProcessingError::Unsupported(
            "non-relative rendering intents are deferred to the CMS execution slice".into(),
        ));
    }
    if !matches!(options.alpha_policy(), AlphaPolicy::PreserveAssociation) {
        return Err(ProcessingError::Unsupported(
            "alpha association changes are deferred to the execution slice".into(),
        ));
    }
    let destination_domain = match plan.destination_route() {
        DestinationRoute::LinearRgb { domain, primaries } => {
            validate_known_d65_primaries(primaries)?;
            domain
        }
        _ => {
            return Err(ProcessingError::Unsupported(
                "this conversion slice only emits linear RGB F32".into(),
            ));
        }
    };
    match plan.source_route() {
        SourceRoute::Icc { .. } => Err(ProcessingError::Unsupported(
            "ICC execution is deferred to the CMS conversion slice".into(),
        )),
        SourceRoute::Cicp { color } => {
            let compatible = match (color.transfer(), destination_domain) {
                (1 | 6 | 8 | 13 | 14 | 15, SampleDomain::LinearRelative)
                | (16, SampleDomain::LinearAbsoluteNits)
                | (18, SampleDomain::HlgSceneLinear) => true,
                _ => false,
            };
            if !compatible {
                return Err(ProcessingError::Unsupported(
                    "CICP transfer and linear destination domain are incompatible".into(),
                ));
            }
            if source.descriptor().alpha() == AlphaAssociation::Premultiplied
                && color.transfer() != 8
            {
                return Err(ProcessingError::Unsupported(
                    "nonlinear premultiplied alpha cannot be preserved in this slice".into(),
                ));
            }
            Ok(())
        }
        SourceRoute::Linear {
            domain: SampleDomain::LinearRelative,
            primaries,
        } if destination_domain == SampleDomain::LinearRelative => {
            validate_known_d65_primaries(primaries)
        }
        SourceRoute::Linear { .. } => Err(ProcessingError::Unsupported(
            "only relative-linear source samples are executable in this slice".into(),
        )),
    }
}

fn validate_known_d65_primaries(
    primaries: RgbPrimaries,
) -> std::result::Result<(), ProcessingError> {
    if [1u16, 9, 12]
        .into_iter()
        .filter_map(|code| cicp_primaries(code).ok())
        .any(|known| known == primaries)
    {
        Ok(())
    } else {
        Err(ProcessingError::Unsupported(
            "only known D65 709/2020/P3-D65 primaries are executable".into(),
        ))
    }
}

fn last_conversion_record(
    options: &ColorConvertOptions<'_>,
    plan: ConversionPlan<'_>,
    transform: &RelativeTransform,
) -> LastConversion {
    let source_kind = match (options.source(), plan.source_route()) {
        (SourceInterpretation::Cicp(_), SourceRoute::Cicp { .. }) => ConversionSource::ExplicitCicp,
        (SourceInterpretation::ActiveMetadata, SourceRoute::Cicp { .. }) => {
            ConversionSource::ActiveCicp
        }
        (SourceInterpretation::Icc { .. }, SourceRoute::Cicp { .. }) => {
            unreachable!("ICC source cannot resolve to CICP")
        }
        (_, SourceRoute::Icc { .. }) => unreachable!("ICC route rejected before execution"),
        (_, SourceRoute::Linear { .. }) => ConversionSource::LinearRelative,
    };
    let native = plan.native_sample_encoding().map(|encoding| {
        NativeInterpretation::from_parts(
            matches!(encoding.range(), super::super::SampleRange::Full),
            native_matrix_code(encoding.matrix()),
            encoding
                .chroma_location()
                .map(|location| location.h273_code()),
        )
    });
    let source_cicp = match plan.source_route() {
        SourceRoute::Cicp { color } => Some(color),
        SourceRoute::Linear { .. } => None,
        SourceRoute::Icc { .. } => unreachable!("ICC route rejected before execution"),
    };
    LastConversion::from_parts(LastConversionParts {
        source: source_kind,
        source_cicp,
        source_icc: None,
        source_primaries_authority: ConversionPrimaries::Explicit(transform.source_primaries),
        source_primaries: transform.source_primaries,
        source_domain: transform.source_domain,
        destination: match transform.destination_domain {
            SampleDomain::LinearRelative => ConversionDestination::LinearRelativeRgb,
            SampleDomain::LinearAbsoluteNits => ConversionDestination::LinearAbsoluteNitsRgb,
            SampleDomain::HlgSceneLinear => ConversionDestination::HlgSceneLinearRgb,
            SampleDomain::HlgDisplayLinear(_) | SampleDomain::Encoded | SampleDomain::Unknown => {
                unreachable!("validated execution route has an unsupported output domain")
            }
        },
        destination_icc: None,
        destination_primaries_authority: ConversionPrimaries::Explicit(
            transform.destination_primaries,
        ),
        destination_domain: transform.destination_domain,
        destination_primaries: transform.destination_primaries,
        intent: match options.rendering_intent() {
            RenderingIntent::RelativeColorimetric => ConversionIntent::RelativeColorimetric,
            _ => unreachable!("non-relative intent rejected before execution"),
        },
        native,
        white_adaptation: match options.white_adaptation() {
            WhiteAdaptation::RequireSameWhite => ConversionWhiteAdaptation::RequireSameWhite,
            WhiteAdaptation::Bradford => unreachable!("Bradford rejected before execution"),
        },
        alpha: match options.alpha_policy() {
            AlphaPolicy::PreserveAssociation => ConversionAlpha::PreserveAssociation,
            AlphaPolicy::ChangeAssociation { .. } => {
                unreachable!("alpha changes rejected before execution")
            }
        },
    })
}

const fn native_matrix_code(matrix: super::super::MatrixCoefficients) -> u16 {
    match matrix {
        super::super::MatrixCoefficients::Identity => 0,
        super::super::MatrixCoefficients::Bt709 => 1,
        super::super::MatrixCoefficients::Bt601 => 6,
        super::super::MatrixCoefficients::Bt2020 => 9,
        super::super::MatrixCoefficients::Unspecified(code) => code,
    }
}

#[derive(Debug, Clone, Copy)]
struct RelativeTransform {
    source_domain: SampleDomain,
    source_primaries: RgbPrimaries,
    destination_primaries: RgbPrimaries,
    destination_domain: SampleDomain,
    matrix: [[f64; 3]; 3],
    transfer: Option<(u16, bool)>,
}

impl RelativeTransform {
    fn from_plan(
        source: &ImageFrame,
        plan: ConversionPlan<'_>,
    ) -> std::result::Result<Self, ProcessingError> {
        let destination_primaries = match plan.destination_route() {
            DestinationRoute::LinearRgb { primaries, .. } => primaries,
            _ => unreachable!("validated execution route has a linear destination"),
        };
        let destination_domain = match plan.destination_route() {
            DestinationRoute::LinearRgb { domain, .. } => domain,
            _ => unreachable!("validated execution route has a linear destination"),
        };
        let (source_domain, source_primaries, transfer) = match plan.source_route() {
            SourceRoute::Cicp { color } => (
                source.descriptor().domain(),
                cicp_primaries(color.primaries())?,
                Some((
                    color.transfer(),
                    color.matrix() == 0 || !matches!(color.transfer(), 13),
                )),
            ),
            SourceRoute::Linear { domain, primaries } => (domain, primaries, None),
            SourceRoute::Icc { .. } => unreachable!("ICC route rejected before execution"),
        };
        let matrix = rgb_conversion_matrix(source_primaries, destination_primaries)?;
        Ok(Self {
            source_domain,
            source_primaries,
            destination_primaries,
            destination_domain,
            matrix,
            transfer,
        })
    }

    fn convert(&self, pixel: NativePixel) -> std::result::Result<[f32; 3], ProcessingError> {
        let mut rgb = match pixel.color {
            NativeColor::Gray(value) => [f64::from(value); 3],
            NativeColor::Rgb(values) => values.map(f64::from),
        };
        if let Some((transfer, normalized_bounds)) = self.transfer {
            if !(self.source_domain == SampleDomain::LinearRelative && transfer == 8) {
                for value in &mut rgb {
                    *value = inverse_cicp_transfer(*value, transfer, normalized_bounds)?;
                }
            } else if rgb.iter().any(|value| !value.is_finite()) {
                return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
                    "linear CICP samples must be finite".into(),
                )));
            }
        }
        let converted = multiply(self.matrix, rgb);
        let output = converted.map(|value| value as f32);
        if output.iter().all(|value| value.is_finite()) {
            Ok(output)
        } else {
            Err(ProcessingError::Conversion(
                "relative RGB conversion produced a non-finite sample".into(),
            ))
        }
    }
}

fn cicp_primaries(code: u16) -> std::result::Result<RgbPrimaries, ProcessingError> {
    let primaries = match code {
        1 => RgbPrimaries::srgb(),
        9 => RgbPrimaries::new(
            (0.708, 0.292),
            (0.170, 0.797),
            (0.131, 0.046),
            (0.3127, 0.3290),
        )
        .map_err(ProcessingError::from)?,
        12 => RgbPrimaries::new(
            (0.680, 0.320),
            (0.265, 0.690),
            (0.150, 0.060),
            (0.3127, 0.3290),
        )
        .map_err(ProcessingError::from)?,
        _ => {
            return Err(ProcessingError::Unsupported(
                "CICP primaries are not in the supported D65 set".into(),
            ));
        }
    };
    Ok(primaries)
}

fn inverse_cicp_transfer(
    value: f64,
    transfer: u16,
    normalized_bounds: bool,
) -> std::result::Result<f64, ProcessingError> {
    if !value.is_finite() || (normalized_bounds && !(0.0..=1.0).contains(&value)) {
        return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
            "normalized CICP sample is outside [0, 1]".into(),
        )));
    }
    let result = match transfer {
        8 => value,
        13 => {
            const ALPHA: f64 = 1.055_010_718_947_586_6;
            const BETA: f64 = 0.003_041_282_560_127_518_3;
            const JUNCTION: f64 = 0.039_293_370_676_847_54;
            let sign = if value < 0.0 { -1.0 } else { 1.0 };
            let magnitude = value.abs();
            let linear = if magnitude <= JUNCTION {
                magnitude / (JUNCTION / BETA)
            } else {
                ((magnitude + ALPHA - 1.0) / ALPHA).powf(2.4)
            };
            sign * linear
        }
        1 | 6 | 14 | 15 => {
            const ALPHA: f64 = 1.099_296_826_809_442;
            const BETA: f64 = 0.018_053_968_510_807;
            if value < 4.5 * BETA {
                value / 4.5
            } else {
                ((value + ALPHA - 1.0) / ALPHA).powf(1.0 / 0.45)
            }
        }
        16 => pq_eotf_f64(value, normalized_bounds)?,
        18 => hlg_scene_from_signal_f64(value, normalized_bounds)?,
        _ => {
            return Err(ProcessingError::Unsupported(
                "CICP transfer is not executable in this slice".into(),
            ));
        }
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ProcessingError::Conversion(
            "CICP transfer produced a non-finite sample".into(),
        ))
    }
}

fn pq_eotf_f64(value: f64, normalized_bounds: bool) -> std::result::Result<f64, ProcessingError> {
    if !value.is_finite() || (normalized_bounds && !(0.0..=1.0).contains(&value)) {
        return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
            "PQ signal is outside [0, 1]".into(),
        )));
    }
    if value == 0.0 {
        return Ok(0.0);
    }
    if value == 1.0 {
        return Ok(10_000.0);
    }
    const M1: f64 = 2610.0 / 16384.0;
    const M2: f64 = 2523.0 / 32.0;
    const C1: f64 = 3424.0 / 4096.0;
    const C2: f64 = 2413.0 / 128.0;
    const C3: f64 = 2392.0 / 128.0;
    let powered = value.powf(1.0 / M2);
    let denominator = C2 - C3 * powered;
    if denominator <= 0.0 {
        return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
            "PQ denominator is non-positive".into(),
        )));
    }
    let result = 10_000.0 * ((powered - C1).max(0.0) / denominator).powf(1.0 / M1);
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ProcessingError::Conversion(
            "PQ luminance is not finite".into(),
        ))
    }
}

fn hlg_scene_from_signal_f64(
    value: f64,
    normalized_bounds: bool,
) -> std::result::Result<f64, ProcessingError> {
    if !value.is_finite() || (normalized_bounds && !(0.0..=1.0).contains(&value)) {
        return Err(ProcessingError::Invalid(HighresError::InvalidSamples(
            "HLG signal is outside [0, 1]".into(),
        )));
    }
    if value == 0.0 || value == 1.0 {
        return Ok(value);
    }
    const A: f64 = 0.178_832_77;
    const B: f64 = 1.0 - 4.0 * A;
    let c = 0.5 - A * (4.0 * A).ln();
    let result = if value <= 0.5 {
        value * value / 3.0
    } else {
        (((value - c) / A).exp() + B) / 12.0
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ProcessingError::Conversion(
            "HLG scene light is not finite".into(),
        ))
    }
}

fn rgb_conversion_matrix(
    source: RgbPrimaries,
    destination: RgbPrimaries,
) -> std::result::Result<[[f64; 3]; 3], ProcessingError> {
    let source_xyz = primaries_matrix(source)?;
    let destination_inverse = inverse_matrix(primaries_matrix(destination)?)?;
    Ok(multiply_matrix(destination_inverse, source_xyz))
}

fn primaries_matrix(
    primaries: RgbPrimaries,
) -> std::result::Result<[[f64; 3]; 3], ProcessingError> {
    let to_xyz = |xy: (f32, f32)| {
        let x = f64::from(xy.0);
        let y = f64::from(xy.1);
        [x / y, 1.0, (1.0 - x - y) / y]
    };
    let red = to_xyz(primaries.red());
    let green = to_xyz(primaries.green());
    let blue = to_xyz(primaries.blue());
    let white = to_xyz(primaries.white());
    let base = [
        [red[0], green[0], blue[0]],
        [red[1], green[1], blue[1]],
        [red[2], green[2], blue[2]],
    ];
    let inverse = inverse_matrix(base)?;
    let scale = [
        inverse[0][0] * white[0] + inverse[0][1] * white[1] + inverse[0][2] * white[2],
        inverse[1][0] * white[0] + inverse[1][1] * white[1] + inverse[1][2] * white[2],
        inverse[2][0] * white[0] + inverse[2][1] * white[1] + inverse[2][2] * white[2],
    ];
    if scale
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "RGB primary white scaling is invalid".into(),
        )));
    }
    Ok([
        [
            base[0][0] * scale[0],
            base[0][1] * scale[1],
            base[0][2] * scale[2],
        ],
        [
            base[1][0] * scale[0],
            base[1][1] * scale[1],
            base[1][2] * scale[2],
        ],
        [
            base[2][0] * scale[0],
            base[2][1] * scale[1],
            base[2][2] * scale[2],
        ],
    ])
}

fn inverse_matrix(matrix: [[f64; 3]; 3]) -> std::result::Result<[[f64; 3]; 3], ProcessingError> {
    let determinant = matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    if !determinant.is_finite() || determinant.abs() <= 1e-12 {
        return Err(ProcessingError::Invalid(HighresError::InvalidMetadata(
            "RGB primary matrix is singular".into(),
        )));
    }
    let inverse = [
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) / determinant,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) / determinant,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) / determinant,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) / determinant,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) / determinant,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) / determinant,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) / determinant,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) / determinant,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) / determinant,
        ],
    ];
    Ok(inverse)
}

fn multiply_matrix(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut output = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            output[row][column] = (0..3)
                .map(|index| left[row][index] * right[index][column])
                .sum();
        }
    }
    output
}

fn multiply(matrix: [[f64; 3]; 3], value: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * value[0] + matrix[0][1] * value[1] + matrix[0][2] * value[2],
        matrix[1][0] * value[0] + matrix[1][1] * value[1] + matrix[1][2] * value[2],
        matrix[2][0] * value[0] + matrix[2][1] * value[1] + matrix[2][2] * value[2],
    ]
}

#[cfg(test)]
#[path = "convert_execute_tests.rs"]
mod tests;
