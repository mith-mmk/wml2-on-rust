//! Bounded T.800 E/F/G inverse quantization, 9/7 synthesis and ICT.
//! The entropy decoder supplies doubled per-coefficient midpoint values.
use super::{
    BlockGrid, Coding, Error, PacketData, bad, packets_planes_grid, subbands, u16be, unsupported,
};

pub(super) fn quant_irreversible(q: &[u8], levels: u8) -> Result<(Vec<u8>, Vec<f64>), Error> {
    let header = *q.first().ok_or_else(|| bad("empty scalar quantization"))?;
    let style = header & 31;
    let bands = 1 + 3 * usize::from(levels);
    let words = match style {
        1 => 1,
        2 => bands,
        _ => {
            return Err(unsupported(
                "9/7 requires scalar derived or expounded quantization",
            ));
        }
    };
    if q.len() != 1 + 2 * words {
        return Err(bad("scalar quantization band count"));
    }
    let mut mb = Vec::with_capacity(bands);
    let mut steps = Vec::with_capacity(bands);
    for band in 0..bands {
        let word = u16be(q, 1 + 2 * if style == 1 { 0 } else { band })?;
        let reduction = if style == 1 && band > 0 {
            (band - 1) / 3
        } else {
            0
        };
        let exponent = (word >> 11)
            .checked_sub(reduction as u16)
            .ok_or_else(|| bad("derived quantization exponent underflow"))?;
        let precision = u16::from(header >> 5) + exponent;
        if !(2..=25).contains(&precision) {
            return Err(unsupported("9/7 coefficient precision must be 1..24"));
        }
        mb.push((precision - 1) as u8);
        let gain = if band == 0 {
            0
        } else if band % 3 == 0 {
            2
        } else {
            1
        };
        // E-3/E-4: R_b = unsigned sample precision 8 + subband gain exponent.
        steps.push(
            (1.0 + f64::from(word & 2047) / 2048.0) * 2.0f64.powi(8 + gain - i32::from(exponent)),
        );
    }
    Ok((mb, steps))
}

// F-7/Table F.4 normalization; do not mix with the alternative 1.1496 scaling.
const K: f64 = 1.230_174_104_914_001;
const LIFTING: [(bool, f64); 4] = [
    (true, 0.443_506_852_043_971),
    (false, 0.882_911_075_530_934),
    (true, -0.052_980_118_572_961),
    (false, -1.586_134_342_059_924),
];
fn inverse_line(packed: &[f64], origin: usize) -> Vec<f64> {
    let n = packed.len();
    // F.3.6 singleton handling precedes scaling/lifting for either filter.
    if n <= 1 {
        return packed
            .iter()
            .map(|&v| if origin & 1 == 0 { v } else { v * 0.5 })
            .collect();
    }
    let odd = origin & 1;
    let low = (n + 1 - odd) / 2;
    let mut out = vec![0.0; n];
    for (i, v) in out.iter_mut().enumerate() {
        let even = (i + odd) & 1 == 0;
        *v = packed[if even { i / 2 } else { low + i / 2 }] * if even { K } else { 1.0 / K };
    }
    // Whole-sample reflection preserves parity. Each lifting phase updates
    // just one parity, so reflected neighbors already have the correct phase.
    for (even, factor) in LIFTING {
        for i in (if even { odd } else { 1 - odd }..n).step_by(2) {
            let left = if i == 0 { 1 } else { i - 1 };
            let right = if i + 1 == n { i - 1 } else { i + 1 };
            out[i] -= factor * (out[left] + out[right]);
        }
    }
    out
}
pub(super) fn synthesize(
    plane: &mut [f64],
    w: usize,
    h: usize,
    levels: u8,
    origin: (usize, usize),
) -> Result<(), Error> {
    let (x0, y0) = origin;
    for depth in (1..=levels).rev() {
        let factor = 1usize << (depth - 1);
        let (rx, ry) = (x0.div_ceil(factor), y0.div_ceil(factor));
        let (rw, rh) = (
            (x0 + w).div_ceil(factor) - rx,
            (y0 + h).div_ceil(factor) - ry,
        );
        if rw == 0 || rh == 0 {
            continue;
        }
        for y in 0..rh {
            let row = inverse_line(&plane[y * w..y * w + rw], rx);
            plane[y * w..y * w + rw].copy_from_slice(&row);
        }
        for x in 0..rw {
            let packed: Vec<_> = (0..rh).map(|y| plane[y * w + x]).collect();
            let column = inverse_line(&packed, ry);
            for y in 0..rh {
                plane[y * w + x] = column[y];
            }
        }
    }
    if !plane.iter().all(|v| v.is_finite()) {
        return Err(bad("nonfinite 9/7 coefficient"));
    }
    Ok(())
}
fn sample(v: f64) -> u8 {
    // Final sample conversion only: nearest-even, inverse DC shift, saturation.
    (v.round_ties_even() + 128.0).clamp(0.0, 255.0) as u8
}
fn ict(y: f64, cb: f64, cr: f64) -> [u8; 3] {
    // G-12..14; ICT retains floating precision until the final sample conversion.
    [
        y + 1.402 * cr,
        y - 0.34413 * cb - 0.71414 * cr,
        y + 1.772 * cb,
    ]
    .map(sample)
}
pub(super) fn tile_pixels_irreversible<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    origin: (usize, usize),
    coding: Coding,
    mb: &[Vec<u8>],
    steps: &[Vec<f64>],
) -> Result<Vec<u8>, Error> {
    let mut coefficients = vec![vec![0; w * h]; mb.len()];
    let mb: Vec<_> = mb.iter().map(Vec::as_slice).collect();
    packets_planes_grid::<true, SPLIT, true>(
        input,
        w,
        h,
        coding.levels,
        &mb,
        &mut coefficients,
        BlockGrid {
            block: coding.block,
            origin,
        },
    )?;
    let bands = subbands::<true>(w, h, coding.levels, origin);
    let mut planes = Vec::with_capacity(coefficients.len());
    for (coefficients, steps) in coefficients.into_iter().zip(steps) {
        let mut plane = vec![0.0; w * h];
        for (&(bx, by, bw, bh, _), &step) in bands.iter().zip(steps) {
            for y in by..by + bh {
                for x in bx..bx + bw {
                    plane[y * w + x] = f64::from(coefficients[y * w + x]) * (step * 0.5);
                }
            }
        }
        synthesize(&mut plane, w, h, coding.levels, origin)?;
        planes.push(plane);
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        let rgb = if planes.len() == 1 {
            [sample(planes[0][i]); 3]
        } else if coding.mct {
            ict(planes[0][i], planes[1][i], planes[2][i])
        } else {
            [
                sample(planes[0][i]),
                sample(planes[1][i]),
                sample(planes[2][i]),
            ]
        };
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derived_exponents_and_precision_bounds() {
        let (mb, steps) = quant_irreversible(&[0x41, 0x84, 0x00], 5).unwrap();
        assert_eq!(
            mb,
            [
                17, 17, 17, 17, 16, 16, 16, 15, 15, 15, 14, 14, 14, 13, 13, 13
            ]
        );
        assert_eq!(steps[0], 1.5 / 256.0);
        assert_eq!(steps[1], 1.5 / 128.0);
        assert_eq!(steps[3], 1.5 / 64.0);
        assert_eq!(steps[13], 1.5 / 8.0);
        assert_eq!(quant_irreversible(&[0x42, 0xb8, 0x00], 0).unwrap().0, [24]);
        for q in [
            &[][..],
            &[0x42][..],
            &[0x42, 0x00][..],
            &[0x43, 0x40, 0x00][..],
            &[0x42, 0xc0, 0x00][..],
            &[0x02, 0x00, 0x00][..],
        ] {
            assert!(quant_irreversible(q, 0).is_err(), "{q:?}");
        }
        assert!(quant_irreversible(&[0x41, 0x08, 0x00], 5).is_err());
    }
    #[test]
    fn singleton_and_final_rounding() {
        assert_eq!(inverse_line(&[], 1), Vec::<f64>::new());
        assert_eq!(inverse_line(&[-3.0], 0), [-3.0]);
        assert_eq!(inverse_line(&[-3.0], 1), [-1.5]);
        assert_eq!(
            [sample(-0.5), sample(0.5), sample(1.5), sample(-1.5)],
            [128, 128, 130, 126]
        );
        assert_eq!(ict(-128.0, 0.0, 0.0), [0; 3]);
        assert_eq!(ict(127.0, 0.0, 0.0), [255; 3]);
    }
}
