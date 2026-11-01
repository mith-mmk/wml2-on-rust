//! Transfer-only conversion of already decoded, oriented Rec.2020/D65 coordinates.
use super::*;

/// Explicit operator and output units. These are different reference domains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JxlHdrLinearization {
    /// ST 2084 EOTF, absolute display-linear RGB components in cd/m² (nits).
    PqDisplayNits,
    /// BT.2100 inverse reference OETF, normalized scene-relative RGB.
    /// No display OOTF, system gamma or peak/black luminance is selected.
    HlgSceneRelative,
}
/// Linear Rec.2020/D65 RGB in `domain` units; straight alpha remains in0..=1.
/// `source_signal` describes the encoded source, not a linear output ICC.
#[derive(Debug)]
pub struct JxlLinearHdrImage {
    pub dimensions: crate::image_info::ImageDimensions,
    pub source_orientation: u8,
    pub rgba_linear: Vec<f32>,
    pub domain: JxlHdrLinearization,
    pub source_signal: JxlHdrSignal,
}
impl JxlEncodedHdrImage {
    /// Consume encoded storage and linearize RGB in place, preserving alpha bits.
    /// Requires a matching explicit operator. No gamut conversion, exposure,
    /// tone mapping or SDR rendering is performed. Source luminance hints do not
    /// change either normative operator. Failure consumes/drops the input; no
    /// partly converted image is returned. Checks occur at most256 pixels apart.
    ///
    /// Admission counts the vector's capacity, the larger result/input inline
    /// object and128 bytes of scalar working allowance. This is an owned-storage
    /// contract, not a global heap/stack/RSS guarantee. No codec remains live;
    /// decode's input, TOC and internal-frame limits do not describe this step.
    pub fn into_linear_with_resource_limits_and_cancel(
        mut self,
        domain: JxlHdrLinearization,
        limits: DecodeLimits,
        policy: JxlResourceLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<JxlLinearHdrImage, Error> {
        let cancel = Cancellation {
            probe: Some(cancelled),
            cancelled: std::cell::Cell::new(false),
        };
        cancel.check()?;
        policy.validate()?;
        let limits = policy.tighten(limits);
        let s = self.signal;
        if !matches!(
            (s.transfer, domain),
            (JxlHdrTransfer::Pq, JxlHdrLinearization::PqDisplayNits)
                | (JxlHdrTransfer::Hlg, JxlHdrLinearization::HlgSceneRelative)
        ) {
            return Err(unsupported("JPEG XL HDR transfer/operator mismatch"));
        }
        if s.primaries_xy != [[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]]
            || s.white_xy != [0.3127, 0.3290]
            || s.rendering_intent > 3
            || ![s.intensity_target, s.min_nits, s.linear_below]
                .iter()
                .all(|v| v.is_finite())
            || s.intensity_target <= 0.0
            || s.min_nits < 0.0
            || s.min_nits > s.intensity_target
            || s.linear_below < 0.0
            || (s.relative_to_max_display && s.linear_below > 1.0)
        {
            return Err(invalid("invalid JPEG XL HDR source signal"));
        }
        let coded = self.dimensions.coded;
        let output = self
            .dimensions
            .output
            .ok_or_else(|| invalid("missing HDR output size"))?;
        let expected = if self.source_orientation >= 5 {
            (coded.height, coded.width)
        } else {
            (coded.width, coded.height)
        };
        if !(1..=8).contains(&self.source_orientation)
            || coded.width == 0
            || coded.height == 0
            || (output.width, output.height) != expected
            || self
                .dimensions
                .intrinsic_display
                .is_some_and(|v| v.width == 0 || v.height == 0)
        {
            return Err(invalid("invalid JPEG XL HDR geometry"));
        }
        let pixels = mul(output.width as usize, output.height as usize)?;
        limits::check(pixels, limits.pixels.min(MAX_PIXELS), "HDR linear pixels")?;
        limits::check(1, limits.frames, "HDR linear frames")?;
        let samples = mul(pixels, 4)?;
        if self.rgba_encoded.len() != samples {
            return Err(invalid("invalid HDR sample count"));
        }
        limits::check(
            mul(samples, 4)?,
            limits.animation_bytes,
            "HDR linear output",
        )?;
        limits::check(
            std::mem::size_of::<JxlHdrSignal>(),
            limits.metadata_bytes,
            "HDR source signal",
        )?;
        let inline = std::mem::size_of::<Self>().max(std::mem::size_of::<JxlLinearHdrImage>());
        let mut workspace = resources::Workspace::new(
            add(mul(self.rgba_encoded.capacity(), 4)?, inline)?,
            limits.expanded_bytes,
        )?;
        workspace.claim(128, "HDR linear scalar allowance")?;
        cancel.check()?;
        for batch in self.rgba_encoded.chunks_mut(256 * 4) {
            cancel.check()?;
            for pixel in batch.chunks_exact_mut(4) {
                if !pixel
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                {
                    return Err(invalid("HDR encoded samples must be finite and normalized"));
                }
                for v in &mut pixel[..3] {
                    let e = f64::from(*v);
                    let linear = match domain {
                        JxlHdrLinearization::PqDisplayNits => {
                            let p = e.powf(32.0 / 2523.0);
                            10000.0
                                * ((p - 3424.0 / 4096.0).max(0.0)
                                    / (2413.0 / 128.0 - 2392.0 / 128.0 * p))
                                    .powf(16384.0 / 2610.0)
                        }
                        JxlHdrLinearization::HlgSceneRelative => {
                            if e <= 0.5 {
                                e * e / 3.0
                            } else {
                                (((e - 0.55991073) / 0.17883277).exp() + 0.28466892) / 12.0
                            }
                        }
                    };
                    *v = linear as f32;
                }
            }
        }
        cancel.check()?;
        Ok(JxlLinearHdrImage {
            dimensions: self.dimensions,
            source_orientation: self.source_orientation,
            rgba_linear: self.rgba_encoded,
            domain,
            source_signal: s,
        })
    }
}
