//! Canonical device-CMYK chunks to borrowed display RGBA.
//!
//! Input is packed C, M, Y, K, with zero meaning no ink. Alpha is a separate
//! span and never enters the ICC transform. JPEG/PSD inversion, YCCK, TIFF
//! planar/byte order and sample interpretation belong to their decoders.
//! Preparation happens before any draw callback. Only malformed/unsupported
//! profiles select a warning plus approximation; resource failures remain
//! errors. Conversion errors never change the route, including after earlier
//! chunks have been drawn. The caller must discard its unpublished output
//! chunk on error: this is not image-wide rollback or atomic streaming.
use crate::warning::ImgWarning;
type Error = Box<dyn std::error::Error>;

/// Internal legacy policies, not new user-facing compatibility switches.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Approximation {
    /// Historical JPEG clipped additive device approximation.
    Additive,
    /// TIFF/PSD subtractive product approximation.
    Multiplicative,
}

#[derive(Debug)]
pub(crate) struct CmykWarning(String);
impl CmykWarning {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl std::fmt::Display for CmykWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CMYK color management: {}", self.0)
    }
}
impl ImgWarning for CmykWarning {}

#[cfg(feature = "color-management")]
const CHUNK_PIXELS: usize = 256;

/// One image/page, no image-owned samples and no heap growth during conversion.
/// Fixed scratch handles arbitrary caller chunks in bounded inner batches.
pub(crate) struct CmykConverter {
    approximation: Approximation,
    #[cfg(feature = "color-management")]
    transform: Option<icc_profile::Transform>,
    #[cfg(feature = "color-management")]
    rgb8: [u8; CHUNK_PIXELS * 3],
    #[cfg(feature = "color-management")]
    rgb16: [u16; CHUNK_PIXELS * 3],
    #[cfg(feature = "color-management")]
    input_float: [f32; CHUNK_PIXELS * 4],
    #[cfg(feature = "color-management")]
    rgb_float: [f32; CHUNK_PIXELS * 3],
}

impl CmykConverter {
    pub(crate) fn prepare(
        profile: Option<&[u8]>,
        approximation: Approximation,
        compiled_budget: usize,
    ) -> Result<(Self, Option<CmykWarning>), Error> {
        let mut warning = None;
        #[cfg(feature = "color-management")]
        let transform = if let Some(profile) = profile {
            use icc_profile::transform::TransformError;
            match prepare_transform(profile, compiled_budget) {
                Ok(transform) => Some(transform),
                Err(
                    error @ (TransformError::InvalidProfile(_)
                    | TransformError::MalformedProfile(_)
                    | TransformError::UnsupportedProfileFeature(_)),
                ) => {
                    warning = Some(CmykWarning::new(format!(
                        "{error}; using the decoder's device-CMYK approximation"
                    )));
                    None
                }
                Err(error) => return Err(Box::new(error)),
            }
        } else {
            None
        };
        #[cfg(not(feature = "color-management"))]
        {
            let _ = compiled_budget;
            if profile.is_some() {
                warning = Some(CmykWarning::new(
                    "embedded profile not applied because color-management is disabled; using the decoder's device-CMYK approximation",
                ));
            }
        }
        Ok((
            Self {
                approximation,
                #[cfg(feature = "color-management")]
                transform,
                #[cfg(feature = "color-management")]
                rgb8: [0; CHUNK_PIXELS * 3],
                #[cfg(feature = "color-management")]
                rgb16: [0; CHUNK_PIXELS * 3],
                #[cfg(feature = "color-management")]
                input_float: [0.0; CHUNK_PIXELS * 4],
                #[cfg(feature = "color-management")]
                rgb_float: [0.0; CHUNK_PIXELS * 3],
            },
            warning,
        ))
    }

    pub(crate) fn has_icc(&self) -> bool {
        #[cfg(feature = "color-management")]
        {
            self.transform.is_some()
        }
        #[cfg(not(feature = "color-management"))]
        {
            false
        }
    }

    pub(crate) fn convert_u8(
        &mut self,
        cmyk: &[u8],
        alpha: Option<&[u8]>,
        rgba: &mut [u8],
    ) -> Result<(), Error> {
        validate_layout(cmyk.len(), alpha.map(<[u8]>::len), rgba.len())?;
        #[cfg(feature = "color-management")]
        if let Some(transform) = &self.transform {
            for (batch, input) in cmyk.chunks(CHUNK_PIXELS * 4).enumerate() {
                let pixels = input.len() / 4;
                transform.transform_u8(input, &mut self.rgb8[..pixels * 3])?;
                let start = batch * CHUNK_PIXELS;
                for (i, output) in rgba[start * 4..(start + pixels) * 4]
                    .chunks_exact_mut(4)
                    .enumerate()
                {
                    output[..3].copy_from_slice(&self.rgb8[i * 3..i * 3 + 3]);
                    output[3] = alpha.map_or(255, |a| a[start + i]);
                }
            }
            return Ok(());
        }
        for (i, (input, output)) in cmyk
            .chunks_exact(4)
            .zip(rgba.chunks_exact_mut(4))
            .enumerate()
        {
            for channel in 0..3 {
                output[channel] = approximate(
                    input[channel].into(),
                    input[3].into(),
                    255,
                    self.approximation,
                ) as u8;
            }
            output[3] = alpha.map_or(255, |a| a[i]);
        }
        Ok(())
    }

    pub(crate) fn convert_u16(
        &mut self,
        cmyk: &[u16],
        alpha: Option<&[u16]>,
        rgba: &mut [u16],
    ) -> Result<(), Error> {
        validate_layout(cmyk.len(), alpha.map(<[u16]>::len), rgba.len())?;
        #[cfg(feature = "color-management")]
        if let Some(transform) = &self.transform {
            for (batch, input) in cmyk.chunks(CHUNK_PIXELS * 4).enumerate() {
                let pixels = input.len() / 4;
                transform.transform_u16(input, &mut self.rgb16[..pixels * 3])?;
                let start = batch * CHUNK_PIXELS;
                for (i, output) in rgba[start * 4..(start + pixels) * 4]
                    .chunks_exact_mut(4)
                    .enumerate()
                {
                    output[..3].copy_from_slice(&self.rgb16[i * 3..i * 3 + 3]);
                    output[3] = alpha.map_or(65535, |a| a[start + i]);
                }
            }
            return Ok(());
        }
        for (i, (input, output)) in cmyk
            .chunks_exact(4)
            .zip(rgba.chunks_exact_mut(4))
            .enumerate()
        {
            for channel in 0..3 {
                output[channel] = approximate(
                    input[channel].into(),
                    input[3].into(),
                    65535,
                    self.approximation,
                ) as u16;
            }
            output[3] = alpha.map_or(65535, |a| a[i]);
        }
        Ok(())
    }

    /// Native TIFF integer samples, including packed and 32-bit channels.
    /// ICC normalizes directly to its f32 domain (not through u8/u16); output
    /// quantization happens last. The no-ICC path preserves TIFF's two rounds.
    pub(crate) fn convert_u32(
        &mut self,
        cmyk: &[u32],
        maximum: u32,
        alpha: Option<&[u32]>,
        rgba: &mut [u8],
    ) -> Result<(), Error> {
        validate_layout(cmyk.len(), alpha.map(<[u32]>::len), rgba.len())?;
        if maximum == 0 {
            return Err(std::io::Error::other("CMYK sample maximum must be nonzero").into());
        }
        #[cfg(feature = "color-management")]
        if let Some(transform) = &self.transform {
            for (batch, input) in cmyk.chunks(CHUNK_PIXELS * 4).enumerate() {
                let pixels = input.len() / 4;
                for (out, &sample) in self.input_float.iter_mut().zip(input) {
                    *out = (f64::from(sample.min(maximum)) / f64::from(maximum)) as f32;
                }
                transform.transform_f32(
                    &self.input_float[..input.len()],
                    &mut self.rgb_float[..pixels * 3],
                )?;
                // Validate the batch before exposing any of it, including f32
                // profile results not representable as finite RGB samples.
                if self.rgb_float[..pixels * 3].iter().any(|v| !v.is_finite()) {
                    return Err(std::io::Error::other(
                        "ICC CMYK transform produced non-finite RGB",
                    )
                    .into());
                }
                let start = batch * CHUNK_PIXELS;
                for (i, output) in rgba[start * 4..(start + pixels) * 4]
                    .chunks_exact_mut(4)
                    .enumerate()
                {
                    for (channel, value) in output[..3].iter_mut().enumerate() {
                        *value =
                            (self.rgb_float[i * 3 + channel].clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                    output[3] = alpha.map_or(255, |a| quantize(a[start + i], maximum));
                }
            }
            return Ok(());
        }
        for (i, (input, output)) in cmyk
            .chunks_exact(4)
            .zip(rgba.chunks_exact_mut(4))
            .enumerate()
        {
            for channel in 0..3 {
                output[channel] = quantize(
                    approximate(input[channel], input[3], maximum, self.approximation),
                    maximum,
                );
            }
            output[3] = alpha.map_or(255, |a| quantize(a[i], maximum));
        }
        Ok(())
    }
}

fn validate_layout(cmyk: usize, alpha: Option<usize>, rgba: usize) -> Result<(), Error> {
    if !cmyk.is_multiple_of(4) || cmyk != rgba || alpha.is_some_and(|n| n != cmyk / 4) {
        return Err(
            std::io::Error::other("CMYK input, alpha or RGBA output layout mismatch").into(),
        );
    }
    Ok(())
}
fn approximate(component: u32, black: u32, maximum: u32, policy: Approximation) -> u32 {
    let component = component.min(maximum);
    let black = black.min(maximum);
    match policy {
        Approximation::Additive => maximum.saturating_sub(component.saturating_add(black)),
        Approximation::Multiplicative => {
            let max = u128::from(maximum);
            ((u128::from(maximum - component) * u128::from(maximum - black) + max / 2) / max) as u32
        }
    }
}
fn quantize(value: u32, maximum: u32) -> u8 {
    ((u64::from(value.min(maximum)) * 255 + u64::from(maximum) / 2) / u64::from(maximum)) as u8
}

#[cfg(feature = "color-management")]
fn prepare_transform(
    profile: &[u8],
    compiled_budget: usize,
) -> Result<icc_profile::Transform, icc_profile::transform::TransformError> {
    use icc_profile::transform::TransformError;
    use icc_profile::{
        ColorSpace, ParseLimits, Profile, Transform, TransformLimits, TransformOptions,
    };
    let budget = compiled_budget.min(64 << 20);
    let scratch = std::mem::size_of::<CmykConverter>();
    let compiled = budget
        .checked_sub(scratch)
        .filter(|n| *n >= 8)
        .ok_or(TransformError::ResourceLimit("CMYK converter workspace"))?;
    let metadata = crate::limits::current().metadata_bytes.min(64 << 20);
    let parse_limits = ParseLimits {
        max_profile_size: metadata,
        max_tag_size: metadata,
        ..Default::default()
    };
    let source = Profile::parse_with_limits(profile, parse_limits)?;
    if source.color_space() != ColorSpace::Cmyk {
        return Err(TransformError::UnsupportedProfileFeature(
            "decoded CMYK samples require a CMYK source profile",
        ));
    }
    let destination = Profile::parse_with_limits(include_bytes!("srgb.icc"), parse_limits)?;
    let transform = Transform::new_with_limits(
        &source,
        &destination,
        TransformOptions::default(),
        TransformLimits::builder()
            .max_compiled_bytes(compiled)
            .max_curve_entries((compiled / 4).clamp(2, 1 << 20))
            .max_clut_entries((compiled / 4).clamp(1, 1 << 24))
            .build()
            .map_err(TransformError::InvalidProfile)?,
    )?;
    let resident = transform
        .memory_usage()?
        .resident_bytes()
        .checked_add(scratch)
        .ok_or(TransformError::ResourceLimit(
            "CMYK converter memory accounting",
        ))?;
    if resident > budget {
        return Err(TransformError::ResourceLimit(
            "CMYK converter retained workspace",
        ));
    }
    Ok(transform)
}

#[cfg(test)]
#[path = "cmyk_tests.rs"]
mod tests;
