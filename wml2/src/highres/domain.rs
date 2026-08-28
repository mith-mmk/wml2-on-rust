//! Explicit sample domains and RGB primaries.

use super::{HighresError, HlgDisplayConditions, ProcessingError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleDomain {
    Unknown,
    Encoded,
    LinearRelative,
    LinearAbsoluteNits,
    HlgSceneLinear,
    HlgDisplayLinear(HlgDisplayConditions),
}
impl Eq for SampleDomain {}

impl SampleDomain {
    /// Reject an unspecified domain at APIs that need to interpret samples.
    pub fn require_explicit(self) -> Result<(), ProcessingError> {
        if matches!(self, Self::Unknown) {
            Err(ProcessingError::Unsupported(
                "sample domain must be explicitly specified".into(),
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbPrimaries {
    red: (f32, f32),
    green: (f32, f32),
    blue: (f32, f32),
    white: (f32, f32),
}
// All instances pass the finite/range checks in `new`; no NaN value can enter
// this private-field type, so the ordinary mathematical equality is total.
impl Eq for RgbPrimaries {}

impl RgbPrimaries {
    pub fn new(
        red: (f32, f32),
        green: (f32, f32),
        blue: (f32, f32),
        white: (f32, f32),
    ) -> Result<Self, HighresError> {
        for (name, (x, y)) in [
            ("red", red),
            ("green", green),
            ("blue", blue),
            ("white", white),
        ] {
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..=1.0).contains(&x)
                || !(0.0..=1.0).contains(&y)
                || x + y <= 0.0
                || (name == "white" && y <= 0.0)
            {
                return Err(HighresError::InvalidMetadata(format!(
                    "{name} primary is invalid"
                )));
            }
        }
        Ok(Self {
            red,
            green,
            blue,
            white,
        })
    }
    pub const fn red(self) -> (f32, f32) {
        self.red
    }
    pub const fn green(self) -> (f32, f32) {
        self.green
    }
    pub const fn blue(self) -> (f32, f32) {
        self.blue
    }
    pub const fn white(self) -> (f32, f32) {
        self.white
    }
    pub fn srgb() -> Self {
        Self::new((0.64, 0.33), (0.30, 0.60), (0.15, 0.06), (0.3127, 0.3290))
            .expect("sRGB primaries are valid")
    }
}
