use super::super::ResourceLimits;
use super::super::domain::{RgbPrimaries, SampleDomain};
use super::super::metadata::{IccColorType, NclxColorInformation};
use super::super::processing::ProcessingError;
use super::super::types::{AlphaAssociation, ImageFrame};
use super::plan::ConversionPlan;

/// Selects which active colour description is authoritative for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInterpretation<'a> {
    /// Read only the descriptor's active colour metadata.
    ActiveMetadata,
    /// Use an explicitly supplied ICC profile; CMS compilation is later.
    Icc {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    /// Use an explicitly supplied CICP/nclx description.
    Cicp(NclxColorInformation),
}

/// Destination vocabulary for the checked, allocation-free planning slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination<'a> {
    IccGray {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    IccRgb {
        profile: &'a [u8],
        color_type: IccColorType,
    },
    EncodedCicpRgb {
        color: NclxColorInformation,
    },
    LinearRgb {
        domain: SampleDomain,
        primaries: RgbPrimaries,
    },
}

impl<'a> Destination<'a> {
    pub const fn icc_gray(profile: &'a [u8], color_type: IccColorType) -> Self {
        Self::IccGray {
            profile,
            color_type,
        }
    }
    pub const fn icc_rgb(profile: &'a [u8], color_type: IccColorType) -> Self {
        Self::IccRgb {
            profile,
            color_type,
        }
    }
    pub const fn encoded_cicp_rgb(color: NclxColorInformation) -> Self {
        Self::EncodedCicpRgb { color }
    }
    pub const fn linear_rgb(domain: SampleDomain, primaries: RgbPrimaries) -> Self {
        Self::LinearRgb { domain, primaries }
    }
    pub const fn domain(self) -> Option<SampleDomain> {
        match self {
            Self::IccGray { .. } | Self::IccRgb { .. } | Self::EncodedCicpRgb { .. } => None,
            Self::LinearRgb { domain, .. } => Some(domain),
        }
    }
    pub const fn is_gray(self) -> bool {
        matches!(self, Self::IccGray { .. })
    }
    pub const fn primaries(self) -> Option<RgbPrimaries> {
        match self {
            Self::LinearRgb { primaries, .. } => Some(primaries),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRange {
    Full,
    Limited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCoefficients {
    Identity,
    Bt601,
    Bt709,
    Bt2020,
    Unspecified(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaPhase {
    Zero,
    Half,
    One,
}

/// Explicit x/y phase and its H.273 code; AV1 positions 1 and 2 differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromaLocation {
    x_phase: ChromaPhase,
    y_phase: ChromaPhase,
    h273_code: u8,
}

impl ChromaLocation {
    pub const fn new(
        x_phase: ChromaPhase,
        y_phase: ChromaPhase,
        h273_code: u8,
    ) -> std::result::Result<Self, &'static str> {
        let expected = match (x_phase, y_phase) {
            (ChromaPhase::Zero, ChromaPhase::Half) => 0,
            (ChromaPhase::Half, ChromaPhase::Half) => 1,
            (ChromaPhase::Zero, ChromaPhase::Zero) => 2,
            (ChromaPhase::Half, ChromaPhase::Zero) => 3,
            (ChromaPhase::Zero, ChromaPhase::One) => 4,
            (ChromaPhase::Half, ChromaPhase::One) => 5,
            _ => u8::MAX,
        };
        if expected == u8::MAX || h273_code != expected {
            return Err("x/y chroma phases and H.273 code disagree");
        }
        Ok(Self {
            x_phase,
            y_phase,
            h273_code,
        })
    }
    pub const fn av1_position(position: u8) -> std::result::Result<Self, &'static str> {
        match position {
            1 => Self::new(ChromaPhase::Zero, ChromaPhase::Half, 0),
            2 => Self::new(ChromaPhase::Zero, ChromaPhase::Zero, 2),
            _ => Err("AV1 chroma sample position is unsupported"),
        }
    }
    pub const fn from_h273_code(code: u8) -> std::result::Result<Self, &'static str> {
        match code {
            0 => Self::new(ChromaPhase::Zero, ChromaPhase::Half, code),
            1 => Self::new(ChromaPhase::Half, ChromaPhase::Half, code),
            2 => Self::new(ChromaPhase::Zero, ChromaPhase::Zero, code),
            3 => Self::new(ChromaPhase::Half, ChromaPhase::Zero, code),
            4 => Self::new(ChromaPhase::Zero, ChromaPhase::One, code),
            5 => Self::new(ChromaPhase::Half, ChromaPhase::One, code),
            _ => Err("H.273 chroma location code is unsupported"),
        }
    }
    pub const fn x_phase(self) -> ChromaPhase {
        self.x_phase
    }
    pub const fn y_phase(self) -> ChromaPhase {
        self.y_phase
    }
    pub const fn h273_code(self) -> u8 {
        self.h273_code
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSampleEncoding {
    range: SampleRange,
    matrix: MatrixCoefficients,
    chroma_location: Option<ChromaLocation>,
}

impl NativeSampleEncoding {
    pub const fn new(
        range: SampleRange,
        matrix: MatrixCoefficients,
        chroma_location: Option<ChromaLocation>,
    ) -> Self {
        Self {
            range,
            matrix,
            chroma_location,
        }
    }
    pub const fn range(self) -> SampleRange {
        self.range
    }
    pub const fn matrix(self) -> MatrixCoefficients {
        self.matrix
    }
    pub const fn chroma_location(self) -> Option<ChromaLocation> {
        self.chroma_location
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhiteAdaptation {
    RequireSameWhite,
    Bradford,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroAlphaPolicy {
    PreserveColor,
    SetZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaPolicy {
    PreserveAssociation,
    ChangeAssociation {
        input: AlphaAssociation,
        output: AlphaAssociation,
        multiplication_domain: SampleDomain,
        zero_alpha: ZeroAlphaPolicy,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderingIntent {
    RelativeColorimetric,
    AbsoluteColorimetric,
    Perceptual,
    Saturation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorConvertOptions<'a> {
    source: SourceInterpretation<'a>,
    destination: Destination<'a>,
    native_sample_encoding: Option<NativeSampleEncoding>,
    white_adaptation: WhiteAdaptation,
    alpha_policy: AlphaPolicy,
    rendering_intent: RenderingIntent,
}

impl<'a> ColorConvertOptions<'a> {
    pub const fn new(destination: Destination<'a>) -> Self {
        Self {
            source: SourceInterpretation::ActiveMetadata,
            destination,
            native_sample_encoding: None,
            white_adaptation: WhiteAdaptation::RequireSameWhite,
            alpha_policy: AlphaPolicy::PreserveAssociation,
            rendering_intent: RenderingIntent::RelativeColorimetric,
        }
    }
    pub const fn source(self) -> SourceInterpretation<'a> {
        self.source
    }
    pub const fn destination(self) -> Destination<'a> {
        self.destination
    }
    pub const fn native_sample_encoding(self) -> Option<NativeSampleEncoding> {
        self.native_sample_encoding
    }
    pub const fn white_adaptation(self) -> WhiteAdaptation {
        self.white_adaptation
    }
    pub const fn alpha_policy(self) -> AlphaPolicy {
        self.alpha_policy
    }
    pub const fn rendering_intent(self) -> RenderingIntent {
        self.rendering_intent
    }
    pub const fn with_source(mut self, source: SourceInterpretation<'a>) -> Self {
        self.source = source;
        self
    }
    pub const fn with_native_sample_encoding(mut self, encoding: NativeSampleEncoding) -> Self {
        self.native_sample_encoding = Some(encoding);
        self
    }
    pub const fn with_white_adaptation(mut self, adaptation: WhiteAdaptation) -> Self {
        self.white_adaptation = adaptation;
        self
    }
    pub const fn with_alpha_policy(mut self, policy: AlphaPolicy) -> Self {
        self.alpha_policy = policy;
        self
    }
    pub const fn with_rendering_intent(mut self, intent: RenderingIntent) -> Self {
        self.rendering_intent = intent;
        self
    }

    /// Validate a borrowed route without output allocation or CMS compilation.
    ///
    /// This checks the frame/resource bounds, ICC header and channel-space
    /// shape, known CICP/native signalling, sample-domain route, and known
    /// white-point compatibility used by this planning slice. A successful
    /// result does not prove ICC execution is possible: PCS/media-white
    /// bridging, profile adaptation, and the selected ICC transform remain
    /// additional compile/execute gates.
    pub fn validate_for(
        &self,
        frame: &ImageFrame,
        limits: &ResourceLimits,
    ) -> std::result::Result<(), ProcessingError> {
        ConversionPlan::inspect(frame, self, limits).map(|_| ())
    }
}
