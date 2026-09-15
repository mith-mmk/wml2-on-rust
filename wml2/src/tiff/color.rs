//! TIFF sample-to-display color conversion.

use crate::color::RGBA;

pub(crate) fn quantize(value: u32, maximum: u32) -> u8 {
    if maximum == 0 {
        return 0;
    }
    (((u64::from(value).min(u64::from(maximum)) * 255) + u64::from(maximum) / 2)
        / u64::from(maximum)) as u8
}

/// Device-CMYK approximation required by TIFF's baseline display behavior.
pub(crate) fn cmyk_to_rgba8(c: u32, m: u32, y: u32, k: u32, maximum: u32, alpha: u32) -> RGBA {
    let c = u128::from(c.min(maximum));
    let m = u128::from(m.min(maximum));
    let y = u128::from(y.min(maximum));
    let k = u128::from(k.min(maximum));
    let max = u128::from(maximum.max(1));
    let r = ((max - c) * (max - k) + max / 2) / max;
    let g = ((max - m) * (max - k) + max / 2) / max;
    let b = ((max - y) * (max - k) + max / 2) / max;
    RGBA {
        red: quantize(r.min(u128::from(u32::MAX)) as u32, maximum),
        green: quantize(g.min(u128::from(u32::MAX)) as u32, maximum),
        blue: quantize(b.min(u128::from(u32::MAX)) as u32, maximum),
        alpha: quantize(alpha, maximum),
    }
}

/// Convert associated (premultiplied) 8-bit color to straight alpha for the
/// legacy RGBA8 drawing API. Zero alpha has no recoverable color and is black.
pub(crate) fn unassociate_alpha(r: u8, g: u8, b: u8, alpha: u8) -> (u8, u8, u8) {
    unassociate_alpha_precision(
        u32::from(r),
        u32::from(g),
        u32::from(b),
        u32::from(alpha),
        u32::from(u8::MAX),
    )
}

/// Convert premultiplied integer samples to straight RGBA8 without first
/// truncating the color or alpha to eight bits.  This matters for 16/32-bit
/// TIFF where a low alpha can leave all three high bytes at zero while the
/// ratio still contains visible color.
pub(crate) fn unassociate_alpha_precision(
    r: u32,
    g: u32,
    b: u32,
    alpha: u32,
    maximum: u32,
) -> (u8, u8, u8) {
    if alpha == 0 || maximum == 0 {
        return (0, 0, 0);
    }
    let restore = |value: u32| {
        let restored =
            (u128::from(value) * u128::from(maximum) + u128::from(alpha) / 2) / u128::from(alpha);
        quantize(restored.min(u128::from(maximum)) as u32, maximum)
    };
    (restore(r), restore(g), restore(b))
}

pub(crate) fn gray_to_rgba8(
    sample: u32,
    maximum: u32,
    white_is_zero: bool,
    alpha: u32,
    associated: bool,
) -> RGBA {
    let sample = if associated {
        if alpha == 0 || maximum == 0 {
            0
        } else {
            ((u128::from(sample) * u128::from(maximum) + u128::from(alpha) / 2) / u128::from(alpha))
                .min(u128::from(maximum)) as u32
        }
    } else {
        sample
    };
    let mut value = quantize(sample, maximum);
    if white_is_zero {
        value = 255 - value;
    }
    let alpha = quantize(alpha, maximum);
    let (red, green, blue) = (value, value, value);
    RGBA {
        red,
        green,
        blue,
        alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cmyk_black_uses_key_channel() {
        let black = cmyk_to_rgba8(0, 0, 0, 255, 255, 255);
        assert_eq!((black.red, black.green, black.blue), (0, 0, 0));
    }

    #[test]
    fn associated_alpha_is_unpremultiplied() {
        // 64 * 255 / 128 = 127.5, rounded to the nearest output value.
        assert_eq!(unassociate_alpha(64, 32, 0, 128), (128, 64, 0));
        assert_eq!(unassociate_alpha(255, 255, 255, 0), (0, 0, 0));
    }

    #[test]
    fn grayscale_associated_alpha_is_converted() {
        let gray = gray_to_rgba8(10, 255, false, 85, true);
        assert_eq!(
            (gray.red, gray.green, gray.blue, gray.alpha),
            (30, 30, 30, 85)
        );
    }
}
