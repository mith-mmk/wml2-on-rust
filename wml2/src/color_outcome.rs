//! Machine-readable observations of color processing for decoded outputs.
//!
//! Records describe completed pixel processing, not the presence of profiles.
//! They are valid only with a successfully completed, non-cancelled decode.
//! Streaming consumers must discard records together with pixels on error/Abort.
//! Missing, malformed and future-version records are always unknown.
use crate::metadata::DataMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
/// Color coordinates of the returned pixels, independent of conversion activity.
pub enum OutputColorSpace {
    #[default]
    Unknown,
    Srgb,
    Source,
    Reconstruction,
    DeviceApproximation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// Why a device approximation was selected instead of an ICC transform.
pub enum ApproximationReason {
    MissingProfile,
    InvalidProfile,
    UnsupportedProfile,
    ColorManagementDisabled,
    /// Reserved for an explicit future fallback; current CMYK runtime failures are errors.
    TransformFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// The actual legacy device-CMYK approximation used for the pixels.
pub enum ApproximationMethod {
    AdditiveCmyk,
    MultiplicativeCmyk,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// Why no additional color transform was performed.
pub enum NotAppliedReason {
    /// Reserved for a producer that establishes no conversion is necessary.
    AlreadySrgb,
    Disabled,
    Unsupported,
    InvalidProfile,
    SourcePreserved,
    ReconstructionPreserved,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
/// Observed processing, never inferred from profile presence.
pub enum Conversion {
    #[default]
    Unknown,
    /// An ICC conversion to sRGB completed successfully.
    AppliedToSrgb,
    Approximation(ApproximationReason, ApproximationMethod),
    NotApplied(NotAppliedReason),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Outcome for one completed decoded output.
pub struct ColorOutcome {
    /// Whether a transform or explicit approximation actually ran.
    pub conversion: Conversion,
    /// Coordinates of the delivered pixels.
    pub output: OutputColorSpace,
}

/// Zero-based output index. TIFF uses display-page order; PSD uses composite
/// followed by emitted layers. Never fall back to another output's record.
pub fn metadata_key(output_index: usize) -> String {
    format!("WML2 Color Outcome {output_index}")
}

impl ColorOutcome {
    /// Stable version-one wire record carried by the existing Raw variant.
    /// No existing metadata enum or image buffer layout changes are required.
    pub fn to_metadata(self) -> DataMap {
        let (state, reason, method) = match self.conversion {
            Conversion::Unknown => (0, 0, 0),
            Conversion::AppliedToSrgb => (1, 0, 0),
            Conversion::Approximation(reason, method) => (
                2,
                match reason {
                    ApproximationReason::MissingProfile => 1,
                    ApproximationReason::InvalidProfile => 2,
                    ApproximationReason::UnsupportedProfile => 3,
                    ApproximationReason::ColorManagementDisabled => 4,
                    ApproximationReason::TransformFailed => 5,
                },
                match method {
                    ApproximationMethod::AdditiveCmyk => 1,
                    ApproximationMethod::MultiplicativeCmyk => 2,
                },
            ),
            Conversion::NotApplied(reason) => (
                3,
                match reason {
                    NotAppliedReason::AlreadySrgb => 1,
                    NotAppliedReason::Disabled => 2,
                    NotAppliedReason::Unsupported => 3,
                    NotAppliedReason::InvalidProfile => 4,
                    NotAppliedReason::SourcePreserved => 5,
                    NotAppliedReason::ReconstructionPreserved => 6,
                },
                0,
            ),
        };
        let output = match self.output {
            OutputColorSpace::Unknown => 0,
            OutputColorSpace::Srgb => 1,
            OutputColorSpace::Source => 2,
            OutputColorSpace::Reconstruction => 3,
            OutputColorSpace::DeviceApproximation => 4,
        };
        DataMap::Raw(vec![b'W', b'C', b'O', 1, state, reason, method, output])
    }
    /// Reads a protocol record, never a human warning or an ICC profile.
    pub fn from_metadata(value: Option<&DataMap>) -> Self {
        let Some(DataMap::Raw(bytes)) = value else {
            return Self::default();
        };
        if bytes.len() != 8 || bytes[..4] != [b'W', b'C', b'O', 1] {
            return Self::default();
        }
        let output = match bytes[7] {
            0 => OutputColorSpace::Unknown,
            1 => OutputColorSpace::Srgb,
            2 => OutputColorSpace::Source,
            3 => OutputColorSpace::Reconstruction,
            4 => OutputColorSpace::DeviceApproximation,
            _ => return Self::default(),
        };
        let conversion = match (bytes[4], bytes[5], bytes[6]) {
            (0, 0, 0) if output == OutputColorSpace::Unknown => Conversion::Unknown,
            (1, 0, 0) if output == OutputColorSpace::Srgb => Conversion::AppliedToSrgb,
            (2, r, m) if output == OutputColorSpace::DeviceApproximation => {
                let reason = match r {
                    1 => ApproximationReason::MissingProfile,
                    2 => ApproximationReason::InvalidProfile,
                    3 => ApproximationReason::UnsupportedProfile,
                    4 => ApproximationReason::ColorManagementDisabled,
                    5 => ApproximationReason::TransformFailed,
                    _ => return Self::default(),
                };
                let method = match m {
                    1 => ApproximationMethod::AdditiveCmyk,
                    2 => ApproximationMethod::MultiplicativeCmyk,
                    _ => return Self::default(),
                };
                Conversion::Approximation(reason, method)
            }
            (3, r, 0) => Conversion::NotApplied(match r {
                1 if output == OutputColorSpace::Srgb => NotAppliedReason::AlreadySrgb,
                2 => NotAppliedReason::Disabled,
                3 => NotAppliedReason::Unsupported,
                4 => NotAppliedReason::InvalidProfile,
                5 if output == OutputColorSpace::Source => NotAppliedReason::SourcePreserved,
                6 if output == OutputColorSpace::Reconstruction => {
                    NotAppliedReason::ReconstructionPreserved
                }
                _ => return Self::default(),
            }),
            _ => return Self::default(),
        };
        Self { conversion, output }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_reason_round_trips_and_invalid_lengths_are_unknown() {
        for reason in [
            ApproximationReason::MissingProfile,
            ApproximationReason::InvalidProfile,
            ApproximationReason::UnsupportedProfile,
            ApproximationReason::ColorManagementDisabled,
            ApproximationReason::TransformFailed,
        ] {
            for method in [
                ApproximationMethod::AdditiveCmyk,
                ApproximationMethod::MultiplicativeCmyk,
            ] {
                let record = ColorOutcome {
                    conversion: Conversion::Approximation(reason, method),
                    output: OutputColorSpace::DeviceApproximation,
                };
                assert_eq!(
                    ColorOutcome::from_metadata(Some(&record.to_metadata())),
                    record
                );
            }
        }
        for (reason, output) in [
            (NotAppliedReason::AlreadySrgb, OutputColorSpace::Srgb),
            (NotAppliedReason::Disabled, OutputColorSpace::Unknown),
            (NotAppliedReason::Unsupported, OutputColorSpace::Unknown),
            (NotAppliedReason::InvalidProfile, OutputColorSpace::Unknown),
            (NotAppliedReason::SourcePreserved, OutputColorSpace::Source),
            (
                NotAppliedReason::ReconstructionPreserved,
                OutputColorSpace::Reconstruction,
            ),
        ] {
            let record = ColorOutcome {
                conversion: Conversion::NotApplied(reason),
                output,
            };
            assert_eq!(
                ColorOutcome::from_metadata(Some(&record.to_metadata())),
                record
            );
        }
        for length in 0..16 {
            if length != 8 {
                assert_eq!(
                    ColorOutcome::from_metadata(Some(&DataMap::Raw(vec![0; length]))),
                    ColorOutcome::default()
                );
            }
        }
        for bytes in [
            [b'W', b'C', b'O', 1, 1, 0, 0, 0],
            [b'W', b'C', b'O', 1, 2, 1, 1, 1],
            [b'W', b'C', b'O', 1, 0, 0, 0, 1],
            [b'W', b'C', b'O', 1, 3, 5, 0, 1],
        ] {
            assert_eq!(
                ColorOutcome::from_metadata(Some(&DataMap::Raw(bytes.to_vec()))),
                ColorOutcome::default()
            );
        }
    }
    #[test]
    fn round_trip_and_reject_future_or_corrupt() {
        let records = [
            ColorOutcome::default(),
            ColorOutcome {
                conversion: Conversion::AppliedToSrgb,
                output: OutputColorSpace::Srgb,
            },
            ColorOutcome {
                conversion: Conversion::Approximation(
                    ApproximationReason::InvalidProfile,
                    ApproximationMethod::AdditiveCmyk,
                ),
                output: OutputColorSpace::DeviceApproximation,
            },
            ColorOutcome {
                conversion: Conversion::NotApplied(NotAppliedReason::ReconstructionPreserved),
                output: OutputColorSpace::Reconstruction,
            },
        ];
        for record in records {
            assert_eq!(
                ColorOutcome::from_metadata(Some(&record.to_metadata())),
                record
            );
        }
        assert_eq!(ColorOutcome::from_metadata(None), ColorOutcome::default());
        assert_eq!(
            ColorOutcome::from_metadata(Some(&DataMap::Ascii("ICC applied".into()))),
            ColorOutcome::default()
        );
        for byte in 0..8 {
            let DataMap::Raw(mut data) = records[1].to_metadata() else {
                unreachable!()
            };
            data[byte] = 255;
            assert_eq!(
                ColorOutcome::from_metadata(Some(&DataMap::Raw(data))),
                ColorOutcome::default()
            );
        }
        assert_ne!(metadata_key(0), metadata_key(1));
    }
}
