//! Transfer functions used by explicit HDR conversion.

use super::HighresError;

const PQ_M1: f64 = 2610.0 / 16384.0;
const PQ_M2: f64 = 2523.0 / 32.0;
const PQ_C1: f64 = 3424.0 / 4096.0;
const PQ_C2: f64 = 2413.0 / 128.0;
const PQ_C3: f64 = 2392.0 / 128.0;
const HLG_A: f64 = 0.178_832_77;
const HLG_B: f64 = 1.0 - 4.0 * HLG_A;
fn hlg_c() -> f64 {
    // `ln` is not a stable const operation on the MSRV used by WML2. Keeping
    // this calculation in one function ensures forward and inverse paths use
    // exactly the same f64 constants.
    0.5 - HLG_A * (4.0 * HLG_A).ln()
}

fn finite_unit(value: f32, name: &'static str) -> Result<f32, HighresError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(HighresError::InvalidSamples(format!(
            "{name} must be finite and in [0, 1]"
        )));
    }
    Ok(value)
}

/// ST 2084 EOTF. The result is absolute linear luminance in nits.
pub fn pq_eotf(signal: f32) -> Result<f32, HighresError> {
    let signal = finite_unit(signal, "PQ signal")?;
    if signal == 0.0 {
        return Ok(0.0);
    }
    if signal == 1.0 {
        return Ok(10_000.0);
    }
    let powered = f64::from(signal).powf(1.0 / PQ_M2);
    let numerator = (powered - PQ_C1).max(0.0);
    let denominator = PQ_C2 - PQ_C3 * powered;
    if denominator <= 0.0 {
        return Err(HighresError::InvalidSamples(
            "PQ denominator is non-positive".into(),
        ));
    }
    let nits = 10_000.0 * (numerator / denominator).powf(1.0 / PQ_M1);
    if !nits.is_finite() {
        return Err(HighresError::InvalidSamples(
            "PQ luminance is not finite".into(),
        ));
    }
    Ok(nits as f32)
}

/// ST 2084 inverse EOTF. Input is absolute luminance in nits.
pub fn pq_oetf(nits: f32) -> Result<f32, HighresError> {
    if !nits.is_finite() || !(0.0..=10_000.0).contains(&nits) {
        return Err(HighresError::InvalidSamples(
            "PQ luminance must be finite and in [0, 10000] nits".into(),
        ));
    }
    if nits == 0.0 {
        return Ok(0.0);
    }
    if nits == 10_000.0 {
        return Ok(1.0);
    }
    let y = (f64::from(nits) / 10_000.0).powf(PQ_M1);
    let signal = ((PQ_C1 + PQ_C2 * y) / (1.0 + PQ_C3 * y)).powf(PQ_M2);
    if !signal.is_finite() {
        return Err(HighresError::InvalidSamples(
            "PQ signal is not finite".into(),
        ));
    }
    Ok(signal as f32)
}

/// BT.2100 HLG inverse OETF (signal to scene-linear relative light).
pub fn hlg_scene_from_signal(signal: f32) -> Result<f32, HighresError> {
    let signal = finite_unit(signal, "HLG signal")?;
    if signal == 0.0 {
        return Ok(0.0);
    }
    if signal == 1.0 {
        return Ok(1.0);
    }
    let c = hlg_c();
    if signal <= 0.5 {
        Ok((f64::from(signal) * f64::from(signal) / 3.0) as f32)
    } else {
        Ok((((f64::from(signal) - c) / HLG_A).exp() + HLG_B) as f32 / 12.0)
    }
}

/// BT.2100 HLG OETF (scene-linear relative light to signal).
pub fn hlg_oetf(scene_linear: f32) -> Result<f32, HighresError> {
    if !scene_linear.is_finite() || !(0.0..=1.0).contains(&scene_linear) {
        return Err(HighresError::InvalidSamples(
            "HLG scene light must be finite and in [0, 1]".into(),
        ));
    }
    if scene_linear == 0.0 || scene_linear == 1.0 {
        return Ok(scene_linear);
    }
    let c = hlg_c();
    let signal = if scene_linear <= 1.0 / 12.0 {
        (3.0_f64 * f64::from(scene_linear)).sqrt() as f32
    } else {
        (HLG_A * (12.0_f64 * f64::from(scene_linear) - HLG_B).ln() + c) as f32
    };
    if !signal.is_finite() || !(0.0..=1.0).contains(&signal) {
        return Err(HighresError::InvalidSamples(
            "HLG signal is not finite and in [0, 1]".into(),
        ));
    }
    Ok(signal)
}

/// Conditions required for an HLG display-referred conversion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HlgDisplayConditions {
    display_peak_nits: f32,
    black_level_nits: f32,
    system_gamma: f32,
}

impl HlgDisplayConditions {
    pub fn new(
        display_peak_nits: f32,
        black_level_nits: f32,
        system_gamma: f32,
    ) -> Result<Self, HighresError> {
        if !display_peak_nits.is_finite()
            || display_peak_nits <= 0.0
            || !black_level_nits.is_finite()
            || black_level_nits < 0.0
            || black_level_nits >= display_peak_nits
            || !system_gamma.is_finite()
            || system_gamma <= 0.0
        {
            return Err(HighresError::InvalidMetadata(
                "HLG display conditions must be finite and in range".into(),
            ));
        }
        Ok(Self {
            display_peak_nits,
            black_level_nits,
            system_gamma,
        })
    }
    pub const fn display_peak_nits(self) -> f32 {
        self.display_peak_nits
    }
    pub const fn black_level_nits(self) -> f32 {
        self.black_level_nits
    }
    pub const fn system_gamma(self) -> f32 {
        self.system_gamma
    }
}
