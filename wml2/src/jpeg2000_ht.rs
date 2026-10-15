//! Independent T.814 (06/2019), clauses 7 and B, HT block reconstruction.
//! The byte readers follow the four different stuffing/padding rules; no
//! OpenJPEG/OpenJPH implementation is linked or copied.
use super::{Error, bad, tier1::Segment};
#[path = "jpeg2000_ht_tables.rs"]
mod tables;

struct Forward<'a> {
    data: &'a [u8],
    pos: usize,
    left: u8,
    last: u8,
    value: u8,
    limit: usize,
    mode: u8,
}
impl Forward<'_> {
    fn bit(&mut self) -> Result<u32, Error> {
        if self.left == 0 {
            self.left = if self.last == 255 { 7 } else { 8 };
            let mut value = if self.pos < self.limit {
                self.data[self.pos]
            } else if self.mode == 0 && self.pos == self.limit || self.mode == 1 {
                255
            } else if self.mode == 2 {
                0
            } else {
                return Err(bad("HT MagSgn extent"));
            };
            if self.mode == 1 && self.pos < self.data.len() {
                if self.pos + 1 == self.data.len() {
                    value = 255;
                } else if self.pos + 2 == self.data.len() {
                    value |= 15;
                }
            }
            if self.left == 7 && value & 128 != 0 && self.pos < self.limit {
                return Err(bad("HT bit stuffing"));
            }
            self.last = value;
            self.value = value;
            self.pos += 1;
        }
        self.left -= 1;
        let bit = if self.mode == 1 {
            u32::from((self.value >> self.left) & 1)
        } else {
            let v = u32::from(self.value & 1);
            self.value >>= 1;
            v
        };
        Ok(bit)
    }
    fn number(&mut self, n: u32) -> Result<u32, Error> {
        if n > 30 {
            return Err(bad("HT magnitude precision"));
        }
        let mut v = 0;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Ok(v)
    }
}
struct Reverse<'a> {
    data: &'a [u8],
    pos: isize,
    left: u8,
    value: u8,
    last: u8,
    lower: usize,
    padding: bool,
}
impl Reverse<'_> {
    fn bit(&mut self) -> Result<u32, Error> {
        if self.left == 0 {
            let value = if self.pos >= self.lower as isize {
                *self
                    .data
                    .get(self.pos as usize)
                    .ok_or_else(|| bad("HT reverse extent"))?
            } else if self.padding {
                0
            } else {
                return Err(bad("HT VLC extent"));
            };
            self.pos -= 1;
            self.left = if self.last > 143 && value & 127 == 127 {
                7
            } else {
                8
            };
            self.last = value;
            self.value = value;
        }
        let v = u32::from(self.value & 1);
        self.value >>= 1;
        self.left -= 1;
        Ok(v)
    }
    fn number(&mut self, n: u32) -> Result<u32, Error> {
        let mut v = 0;
        for i in 0..n {
            v |= self.bit()? << i;
        }
        Ok(v)
    }
    fn prefix(&mut self) -> Result<u32, Error> {
        if self.bit()? != 0 {
            Ok(1)
        } else if self.bit()? != 0 {
            Ok(2)
        } else if self.bit()? != 0 {
            Ok(3)
        } else {
            Ok(5)
        }
    }
    fn suffix(&mut self, p: u32) -> Result<u32, Error> {
        self.number(if p < 3 {
            0
        } else if p == 3 {
            1
        } else {
            5
        })
    }
}
struct Mel<'a> {
    reader: Forward<'a>,
    k: usize,
    run: u32,
    one: bool,
}
impl Mel<'_> {
    fn symbol(&mut self) -> Result<bool, Error> {
        const E: [u8; 13] = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];
        if self.run == 0 && !self.one {
            let e = E[self.k];
            if self.reader.bit()? != 0 {
                self.run = 1 << e;
                self.k = (self.k + 1).min(12);
            } else {
                for _ in 0..e {
                    self.run = self.run * 2 + self.reader.bit()?;
                }
                self.k = self.k.saturating_sub(1);
                self.one = true;
            }
        }
        if self.run > 0 {
            self.run -= 1;
            Ok(false)
        } else {
            self.one = false;
            Ok(true)
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Quad {
    rho: u8,
    offset: bool,
    k: u8,
    ones: u8,
}
fn decode_quad(
    vlc: &mut Reverse<'_>,
    mel: &mut Mel<'_>,
    initial: bool,
    context: u8,
) -> Result<Quad, Error> {
    if context == 0 && !mel.symbol()? {
        return Ok(Quad::default());
    }
    let table = if initial {
        &tables::VLC_0
    } else {
        &tables::VLC_1
    };
    let mut word = 0;
    for len in 1..=7 {
        word |= vlc.bit()? << (len - 1);
        let v = table[usize::from(context) * 128 + word as usize];
        if v >> 13 == len {
            return Ok(Quad {
                rho: (v & 15) as u8,
                offset: v & 16 != 0,
                k: ((v >> 5) & 15) as u8,
                ones: ((v >> 9) & 15) as u8,
            });
        }
    }
    Err(bad("HT CxtVLC code"))
}
fn residuals(
    vlc: &mut Reverse<'_>,
    mel: &mut Mel<'_>,
    q: &[Quad],
    initial: bool,
) -> Result<[u32; 2], Error> {
    let joint = initial && q.len() == 2 && q[0].offset && q[1].offset;
    let extra = if joint && mel.symbol()? { 2 } else { 0 };
    let mut prefix = [0; 2];
    let mut suffix = [0; 2];
    for i in 0..q.len() {
        if q[i].offset {
            prefix[i] = if i == 1 && joint && extra == 0 && prefix[0] > 2 {
                vlc.bit()? + 1
            } else {
                vlc.prefix()?
            };
        }
    }
    for i in 0..q.len() {
        suffix[i] = vlc.suffix(prefix[i])?;
    }
    let mut out = [0; 2];
    for i in 0..q.len() {
        if q[i].offset {
            out[i] = extra
                + prefix[i]
                + suffix[i]
                + if suffix[i] >= 28 {
                    4 * vlc.number(4)?
                } else {
                    0
                };
        }
    }
    Ok(out)
}

pub(super) fn decode(
    segments: &[Segment],
    w: usize,
    h: usize,
    planes: u8,
    style: u8,
) -> Result<Vec<i32>, Error> {
    if w == 0 || h == 0 || w > 1024 || h > 1024 || w * h > 4096 || planes == 0 || planes > 28 {
        return Err(bad("HT block geometry"));
    }
    let cleanup = segments
        .iter()
        .enumerate()
        .filter(|(i, s)| i % 2 == 0 && !s.data.is_empty())
        .next_back();
    let Some((segment_index, segment)) = cleanup else {
        return Ok(vec![0; w * h]);
    };
    let shift = usize::from(planes)
        .checked_sub(1 + segment_index / 2)
        .ok_or_else(|| bad("HT skipped bit planes"))?;
    let data = &segment.data;
    let n = data.len();
    if !(2..65535).contains(&n)
        || data[n - 1] == 255
        || data.windows(2).any(|b| b[0] == 255 && b[1] > 143)
    {
        return Err(bad("HT cleanup segment"));
    }
    let suffix = usize::from(data[n - 1]) * 16 + usize::from(data[n - 2] & 15);
    if suffix < 2 || suffix > n.min(4079) {
        return Err(bad("HT suffix length"));
    }
    let prefix = n - suffix;
    if prefix > 0 && data[prefix - 1] == 255 {
        return Err(bad("HT prefix termination"));
    }
    let mut ms = Forward {
        data,
        pos: 0,
        left: 0,
        last: 0,
        value: 0,
        limit: prefix,
        mode: 0,
    };
    let mut mel = Mel {
        reader: Forward {
            data,
            pos: prefix,
            left: 0,
            last: 0,
            value: 0,
            limit: n,
            mode: 1,
        },
        k: 0,
        run: 0,
        one: false,
    };
    let last = data[n - 2] | 15;
    let tmp = last >> 4;
    let mut vlc = Reverse {
        data,
        pos: n as isize - 3,
        left: if tmp & 7 < 7 { 4 } else { 3 },
        value: tmp,
        last,
        lower: prefix,
        padding: false,
    };
    let qw = w.div_ceil(2);
    let qh = h.div_ceil(2);
    let mut quads = vec![Quad::default(); qw * qh];
    let mut magnitude = vec![0u32; qw * qh * 4];
    let mut sign = vec![false; qw * qh * 4];
    let mut exponent = vec![0u8; qw * qh * 4];
    for y in 0..qh {
        for x in (0..qw).step_by(2) {
            let size = (qw - x).min(2);
            let start = y * qw + x;
            for j in 0..size {
                let q = start + j;
                let left = if x + j == 0 { 0 } else { quads[q - 1].rho };
                let context = if y == 0 {
                    u8::from(left & 3 != 0) + ((left >> 2) & 1) * 2 + ((left >> 3) & 1) * 4
                } else {
                    let above = quads[q - qw].rho;
                    let nw = if x + j == 0 {
                        0
                    } else {
                        (quads[q - qw - 1].rho >> 3) & 1
                    };
                    let nf = if x + j + 1 == qw {
                        0
                    } else {
                        (quads[q - qw + 1].rho >> 1) & 1
                    };
                    (nw | ((above >> 1) & 1))
                        + u8::from(left & 12 != 0) * 2
                        + (((above >> 3) & 1) | nf) * 4
                };
                quads[q] = decode_quad(&mut vlc, &mut mel, y == 0, context)?;
            }
            let residual = residuals(&mut vlc, &mut mel, &quads[start..start + size], y == 0)?;
            for j in 0..size {
                let q = start + j;
                let quad = quads[q];
                let predictor = if y == 0 || quad.rho.count_ones() < 2 {
                    1
                } else {
                    let above = q - qw;
                    let mut max = exponent[above * 4 + 1].max(exponent[above * 4 + 3]);
                    if x + j != 0 {
                        max = max.max(exponent[above * 4 - 1]);
                    }
                    if x + j + 1 < qw {
                        max = max.max(exponent[above * 4 + 5]);
                    }
                    u32::from(max.saturating_sub(1)).max(1)
                };
                let bound = predictor + residual[j];
                if bound > 29 {
                    return Err(bad("HT exponent bound"));
                }
                for k in 0..4 {
                    if quad.rho & (1 << k) == 0 {
                        continue;
                    }
                    let bits = bound - u32::from((quad.k >> k) & 1);
                    if bits == 0 {
                        return Err(bad("HT significant zero bit count"));
                    }
                    let v = ms.number(bits)? | (u32::from((quad.ones >> k) & 1) << bits);
                    let mu = (v >> 1) + 1;
                    let i = q * 4 + k;
                    magnitude[i] = mu;
                    sign[i] = v & 1 != 0;
                    exponent[i] = (32 - (2 * mu - 1).leading_zeros()) as u8;
                    if (x + j) * 2 + k / 2 >= w || y * 2 + k % 2 >= h {
                        return Err(bad("HT padded sample is significant"));
                    }
                }
            }
        }
    }
    // Convert quad indices to raster order for the stripe-oriented refinement.
    let mut mu = vec![0u32; w * h];
    let mut negative = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let q = (y / 2) * qw + x / 2;
            let i = q * 4 + (x % 2) * 2 + y % 2;
            mu[y * w + x] = magnitude[i];
            negative[y * w + x] = sign[i];
        }
    }
    let mut refined = vec![false; w * h];
    let mut r = vec![false; w * h];
    let refinement = segments
        .get(segment_index + 1)
        .filter(|s| !s.data.is_empty());
    if let Some(segment) = refinement {
        if shift == 0
            || segment.passes > 2
            || segment.data.len() >= 2047
            || segment.data.last() == Some(&255)
            || segment.data.windows(2).any(|b| b[0] == 255 && b[1] > 143)
        {
            return Err(bad("HT refinement segment"));
        }
        let d = &segment.data;
        let mut sp = Forward {
            data: d,
            pos: 0,
            left: 0,
            last: 0,
            value: 0,
            limit: d.len(),
            mode: 2,
        };
        for stripe in (0..h).step_by(4) {
            for group in (0..w).step_by(4) {
                for x in group..(group + 4).min(w) {
                    for y in stripe..(stripe + 4).min(h) {
                        let i = y * w + x;
                        if mu[i] != 0 {
                            continue;
                        }
                        let mut neighbor = false;
                        for dy in -1isize..=1 {
                            for dx in -1isize..=1 {
                                if dx == 0 && dy == 0 || style & 8 != 0 && y % 4 == 3 && dy > 0 {
                                    continue;
                                }
                                let Some(nx) = x.checked_add_signed(dx).filter(|&n| n < w) else {
                                    continue;
                                };
                                let Some(ny) = y.checked_add_signed(dy).filter(|&n| n < h) else {
                                    continue;
                                };
                                let ni = ny * w + nx;
                                let causal = ny / 4 < y / 4
                                    || ny / 4 == y / 4 && (nx < x || nx == x && ny < y);
                                neighbor |= mu[ni] != 0 || causal && r[ni];
                            }
                        }
                        if neighbor {
                            refined[i] = true;
                            r[i] = sp.bit()? != 0;
                        }
                    }
                }
                for x in group..(group + 4).min(w) {
                    for y in stripe..(stripe + 4).min(h) {
                        let i = y * w + x;
                        if r[i] {
                            negative[i] = sp.bit()? != 0;
                        }
                    }
                }
            }
        }
        if segment.passes == 2 {
            let mut mr = Reverse {
                data: d,
                pos: d.len() as isize - 1,
                left: 0,
                value: 0,
                last: 255,
                lower: 0,
                padding: true,
            };
            for stripe in (0..h).step_by(4) {
                for x in 0..w {
                    for y in stripe..(stripe + 4).min(h) {
                        let i = y * w + x;
                        if mu[i] != 0 {
                            refined[i] = true;
                            r[i] = mr.bit()? != 0;
                        }
                    }
                }
            }
        }
    }
    let mut out = Vec::with_capacity(w * h);
    for i in 0..w * h {
        let value = (u64::from(mu[i]) << (shift + 1))
            + if refined[i] {
                (u64::from(r[i]) << shift) + if shift > 0 { 1u64 << (shift - 1) } else { 0 }
            } else if mu[i] != 0 {
                1u64 << shift
            } else {
                0
            };
        let v = i32::try_from(value).map_err(|_| bad("HT coefficient overflow"))?;
        out.push(if negative[i] { -v } else { v });
    }
    Ok(out)
}
