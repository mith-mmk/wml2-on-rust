//! Small, bounded implementation of T.800 Annexes C and D, style zero only.
use super::{Error, bad};
// Qe, next MPS state, next LPS state, exchange MPS on LPS.
const STATES: [(u32, usize, usize, bool); 47] = [
    (0x5601, 1, 1, true),
    (0x3401, 2, 6, false),
    (0x1801, 3, 9, false),
    (0x0ac1, 4, 12, false),
    (0x0521, 5, 29, false),
    (0x0221, 38, 33, false),
    (0x5601, 7, 6, true),
    (0x5401, 8, 14, false),
    (0x4801, 9, 14, false),
    (0x3801, 10, 14, false),
    (0x3001, 11, 17, false),
    (0x2401, 12, 18, false),
    (0x1c01, 13, 20, false),
    (0x1601, 29, 21, false),
    (0x5601, 15, 14, true),
    (0x5401, 16, 14, false),
    (0x5101, 17, 15, false),
    (0x4801, 18, 16, false),
    (0x3801, 19, 17, false),
    (0x3401, 20, 18, false),
    (0x3001, 21, 19, false),
    (0x2801, 22, 19, false),
    (0x2401, 23, 20, false),
    (0x2201, 24, 21, false),
    (0x1c01, 25, 22, false),
    (0x1801, 26, 23, false),
    (0x1601, 27, 24, false),
    (0x1401, 28, 25, false),
    (0x1201, 29, 26, false),
    (0x1101, 30, 27, false),
    (0x0ac1, 31, 28, false),
    (0x09c1, 32, 29, false),
    (0x08a1, 33, 30, false),
    (0x0521, 34, 31, false),
    (0x0441, 35, 32, false),
    (0x02a1, 36, 33, false),
    (0x0221, 37, 34, false),
    (0x0141, 38, 35, false),
    (0x0111, 39, 36, false),
    (0x0085, 40, 37, false),
    (0x0049, 41, 38, false),
    (0x0025, 42, 39, false),
    (0x0015, 43, 40, false),
    (0x0009, 44, 41, false),
    (0x0005, 45, 42, false),
    (0x0001, 45, 43, false),
    (0x5601, 46, 46, false),
];
struct Mq<'a> {
    data: &'a [u8],
    pos: usize,
    a: u32,
    c: u32,
    count: u8,
    state: [usize; 19],
    mps: [bool; 19],
}
impl<'a> Mq<'a> {
    fn new(data: &'a [u8]) -> Result<Self, Error> {
        if data.is_empty() || data.windows(2).any(|b| b[0] == 255 && b[1] > 0x8f) {
            return Err(bad("invalid arithmetic segment"));
        }
        let mut m = Self {
            data,
            pos: 0,
            a: 0x8000,
            c: u32::from(data[0]) << 16,
            count: 0,
            state: [0; 19],
            mps: [false; 19],
        };
        m.state[0] = 4;
        m.state[17] = 3;
        m.state[18] = 46;
        m.byte_in();
        m.c = m.c.wrapping_shl(7);
        m.count -= 7;
        Ok(m)
    }
    fn byte_in(&mut self) {
        let b = self.data.get(self.pos).copied().unwrap_or(255);
        let n = self.data.get(self.pos + 1).copied().unwrap_or(255);
        if b == 255 && n > 0x8f {
            self.c = self.c.wrapping_add(0xff00);
            self.count = 8;
        } else {
            self.pos += 1;
            self.count = if b == 255 { 7 } else { 8 };
            self.c = self
                .c
                .wrapping_add(u32::from(n) << if b == 255 { 9 } else { 8 });
        }
    }
    fn bit(&mut self, cx: usize) -> bool {
        let (qe, nm, nl, sw) = STATES[self.state[cx]];
        self.a -= qe;
        let m = self.mps[cx];
        let d;
        if self.c >> 16 < qe {
            if self.a < qe {
                d = m;
                self.state[cx] = nm;
            } else {
                d = !m;
                self.state[cx] = nl;
                self.mps[cx] ^= sw;
            }
            self.a = qe;
        } else {
            self.c = self.c.wrapping_sub(qe << 16);
            if self.a >= 0x8000 {
                return m;
            }
            if self.a < qe {
                d = !m;
                self.state[cx] = nl;
                self.mps[cx] ^= sw;
            } else {
                d = m;
                self.state[cx] = nm;
            }
        }
        while self.a < 0x8000 {
            if self.count == 0 {
                self.byte_in();
            }
            self.a <<= 1;
            self.c = self.c.wrapping_shl(1);
            self.count -= 1;
        }
        d
    }
}
struct Block<const MIDPOINT: bool, const CAUSAL: bool = false> {
    w: usize,
    h: usize,
    orientation: u8,
    value: Vec<i32>,
    sig: Vec<bool>,
    negative: Vec<bool>,
    visited: Vec<bool>,
    refined: Vec<bool>,
    last_plane: Vec<u8>,
}
trait Entropy {
    fn bit(&mut self, cx: usize) -> bool;
    fn raw(&self) -> bool {
        false
    }
}
impl Entropy for Mq<'_> {
    fn bit(&mut self, cx: usize) -> bool {
        Mq::bit(self, cx)
    }
}
struct Raw<'a> {
    data: &'a [u8],
    pos: usize,
    left: u8,
    last: u8,
}
impl Entropy for Raw<'_> {
    fn bit(&mut self, _: usize) -> bool {
        if self.left == 0 {
            self.left = if self.last == 255 { 7 } else { 8 };
            self.last = self.data.get(self.pos).copied().unwrap_or(255);
            self.pos += 1;
        }
        self.left -= 1;
        self.last & (1 << self.left) != 0
    }
    fn raw(&self) -> bool {
        true
    }
}
impl<const MIDPOINT: bool, const CAUSAL: bool> Block<MIDPOINT, CAUSAL> {
    fn neighbor(&self, x: usize, y: usize, dx: isize, dy: isize) -> Option<usize> {
        if CAUSAL && y % 4 == 3 && dy > 0 {
            return None;
        }
        let x = x.checked_add_signed(dx)?;
        let y = y.checked_add_signed(dy)?;
        if x < self.w && y < self.h {
            Some(y * self.w + x)
        } else {
            None
        }
    }
    fn significant(&self, x: usize, y: usize, dx: isize, dy: isize) -> bool {
        self.neighbor(x, y, dx, dy).is_some_and(|i| self.sig[i])
    }
    // Keep coefficient-loop bounds visible when forming neighbor contexts.
    #[inline(always)]
    fn context(&self, x: usize, y: usize) -> usize {
        let mut h =
            usize::from(self.significant(x, y, -1, 0)) + usize::from(self.significant(x, y, 1, 0));
        let mut v =
            usize::from(self.significant(x, y, 0, -1)) + usize::from(self.significant(x, y, 0, 1));
        let d = [(-1, -1), (1, -1), (-1, 1), (1, 1)]
            .into_iter()
            .filter(|&(dx, dy)| self.significant(x, y, dx, dy))
            .count();
        if self.orientation == 1 {
            std::mem::swap(&mut h, &mut v);
        }
        if self.orientation == 3 {
            match (d, h + v) {
                (3.., _) => 8,
                (2, 1..) => 7,
                (2, 0) => 6,
                (1, 2..) => 5,
                (1, 1) => 4,
                (1, 0) => 3,
                (0, 2..) => 2,
                (0, 1) => 1,
                _ => 0,
            }
        } else {
            match (h, v, d) {
                (2, _, _) => 8,
                (1, 1.., _) => 7,
                (1, 0, 1..) => 6,
                (1, 0, 0) => 5,
                (0, 2, _) => 4,
                (0, 1, _) => 3,
                (0, 0, 2..) => 2,
                (0, 0, 1) => 1,
                _ => 0,
            }
        }
    }
    fn signed_neighbor(&self, x: usize, y: usize, dx: isize, dy: isize) -> i32 {
        self.neighbor(x, y, dx, dy)
            .filter(|&i| self.sig[i])
            .map_or(0, |i| if self.negative[i] { -1 } else { 1 })
    }
    // Called in coefficient loops with already-validated coordinates.
    #[inline(always)]
    fn make_significant(&mut self, mq: &mut impl Entropy, x: usize, y: usize, plane: u8) {
        let h = (self.signed_neighbor(x, y, -1, 0) + self.signed_neighbor(x, y, 1, 0)).signum();
        let v = (self.signed_neighbor(x, y, 0, -1) + self.signed_neighbor(x, y, 0, 1)).signum();
        let prediction = h < 0 || (h == 0 && v < 0);
        let v = if h < 0 { -v } else { v };
        let cx = match (h.abs(), v) {
            (0, 0) => 9,
            (0, _) => 10,
            (1, -1) => 11,
            (1, 0) => 12,
            _ => 13,
        };
        let i = y * self.w + x;
        self.negative[i] = mq.bit(cx) ^ (prediction && !mq.raw());
        self.sig[i] = true;
        self.value[i] = 1 << plane;
        if MIDPOINT {
            self.last_plane[i] = plane;
        }
    }
    fn propagation(&mut self, mq: &mut impl Entropy, plane: u8) {
        for stripe in (0..self.h).step_by(4) {
            for x in 0..self.w {
                for y in stripe..(stripe + 4).min(self.h) {
                    let i = y * self.w + x;
                    let cx = self.context(x, y);
                    if !self.sig[i] && cx != 0 {
                        self.visited[i] = true;
                        if mq.bit(cx) {
                            self.make_significant(mq, x, y, plane);
                        }
                    }
                }
            }
        }
    }
    fn refinement(&mut self, mq: &mut impl Entropy, plane: u8) {
        for stripe in (0..self.h).step_by(4) {
            for x in 0..self.w {
                for y in stripe..(stripe + 4).min(self.h) {
                    let i = y * self.w + x;
                    if self.sig[i] && !self.visited[i] {
                        let cx = if self.refined[i] {
                            16
                        } else if self.context(x, y) == 0 {
                            14
                        } else {
                            15
                        };
                        if mq.bit(cx) {
                            self.value[i] |= 1 << plane;
                        }
                        self.refined[i] = true;
                        if MIDPOINT {
                            self.last_plane[i] = plane;
                        }
                    }
                }
            }
        }
    }
    fn cleanup(&mut self, mq: &mut impl Entropy, plane: u8) {
        for stripe in (0..self.h).step_by(4) {
            for x in 0..self.w {
                let end = (stripe + 4).min(self.h);
                let mut start = stripe;
                if end - stripe == 4
                    && (stripe..end).all(|y| {
                        let i = y * self.w + x;
                        !self.sig[i] && !self.visited[i] && self.context(x, y) == 0
                    })
                {
                    if !mq.bit(17) {
                        continue;
                    }
                    let offset = usize::from(mq.bit(18)) * 2 + usize::from(mq.bit(18));
                    self.make_significant(mq, x, stripe + offset, plane);
                    start = stripe + offset + 1;
                }
                for y in start..end {
                    let i = y * self.w + x;
                    if !self.sig[i] && !self.visited[i] && mq.bit(self.context(x, y)) {
                        self.make_significant(mq, x, y, plane);
                    }
                }
            }
        }
        self.visited.fill(false);
    }
}
pub(super) fn decode(
    data: &[u8],
    w: usize,
    h: usize,
    orientation: u8,
    planes: u8,
) -> Result<Vec<i32>, Error> {
    // Keep the proven small-block bounds visible to the optimizer. The wider
    // domain otherwise slows existing 16x16 Gray inputs in release builds.
    if w <= 16 && h <= 16 {
        decode_bounded::<16, false>(data, w, h, orientation, planes, 0)
    } else {
        decode_bounded::<1024, false>(data, w, h, orientation, planes, 0)
    }
}
// Doubled sign-magnitude reconstruction with r=1/2 (T.800 E-6).
// Per-coefficient precision matters when rate control stops at sigprop/refinement.
pub(super) fn decode_irreversible(
    data: &[u8],
    w: usize,
    h: usize,
    orientation: u8,
    planes: u8,
    passes: u32,
) -> Result<Vec<i32>, Error> {
    if w <= 16 && h <= 16 {
        decode_bounded::<16, true>(data, w, h, orientation, planes, passes)
    } else {
        decode_bounded::<1024, true>(data, w, h, orientation, planes, passes)
    }
}
fn decode_bounded<const AXIS: usize, const MIDPOINT: bool>(
    data: &[u8],
    w: usize,
    h: usize,
    orientation: u8,
    planes: u8,
    passes: u32,
) -> Result<Vec<i32>, Error> {
    if w == 0
        || h == 0
        || w > AXIS
        || h > AXIS
        || w * h > 4096
        || planes == 0
        || planes > if MIDPOINT { 24 } else { 16 }
    {
        return Err(bad("invalid code-block geometry or bit depth"));
    }
    if MIDPOINT && (passes == 0 || passes > u32::from(planes) * 3 - 2) {
        return Err(bad("invalid code-block pass count"));
    }
    let n = w * h;
    let mut block: Block<MIDPOINT> = Block {
        w,
        h,
        orientation,
        value: vec![0; n],
        sig: vec![false; n],
        negative: vec![false; n],
        visited: vec![false; n],
        refined: vec![false; n],
        last_plane: if MIDPOINT { vec![0; n] } else { Vec::new() },
    };
    let mut mq = Mq::new(data)?;
    block.cleanup(&mut mq, planes - 1);
    let mut decoded = 1;
    for plane in (0..planes - 1).rev() {
        if MIDPOINT && decoded == passes {
            break;
        }
        block.propagation(&mut mq, plane);
        decoded += 1;
        if MIDPOINT && decoded == passes {
            break;
        }
        block.refinement(&mut mq, plane);
        decoded += 1;
        if MIDPOINT && decoded == passes {
            break;
        }
        block.cleanup(&mut mq, plane);
        decoded += 1;
    }
    for i in 0..n {
        if MIDPOINT && block.sig[i] {
            block.value[i] = 2 * block.value[i] + (1 << block.last_plane[i]);
        }
        if block.negative[i] {
            block.value[i] = -block.value[i];
        }
    }
    Ok(block.value)
}

/// A terminated MQ/RAW segment, assembled across all contributing layers.
pub(super) struct Segment {
    pub data: Vec<u8>,
    pub passes: u32,
}
pub(super) fn segment_capacity(style: u8, pass: u32) -> u32 {
    if style & 64 != 0 {
        if pass % 3 == 0 { 1 } else { 2 }
    } else if style & 4 != 0 {
        1
    } else if style & 1 == 0 {
        u32::MAX
    } else if pass < 10 {
        10 - pass
    } else if pass % 3 == 1 {
        2
    } else {
        1
    }
}
pub(super) fn decode_segments(
    segments: &[Segment],
    w: usize,
    h: usize,
    orientation: u8,
    planes: u8,
    style: u8,
) -> Result<Vec<i32>, Error> {
    if style & 8 != 0 {
        segmented::<true>(segments, w, h, orientation, planes, style)
    } else {
        segmented::<false>(segments, w, h, orientation, planes, style)
    }
}
fn segmented<const CAUSAL: bool>(
    segments: &[Segment],
    w: usize,
    h: usize,
    orientation: u8,
    planes: u8,
    style: u8,
) -> Result<Vec<i32>, Error> {
    if w == 0 || h == 0 || w > 1024 || h > 1024 || w * h > 4096 || planes == 0 || planes > 28 {
        return Err(bad("code-block bounds"));
    }
    let n = w * h;
    let mut block: Block<true, CAUSAL> = Block {
        w,
        h,
        orientation,
        value: vec![0; n],
        sig: vec![false; n],
        negative: vec![false; n],
        visited: vec![false; n],
        refined: vec![false; n],
        last_plane: vec![0; n],
    };
    let mut states = [0; 19];
    states[0] = 4;
    states[17] = 3;
    states[18] = 46;
    let mut mps = [false; 19];
    let mut pass = 0;
    for segment in segments {
        if segment.passes == 0 || pass + segment.passes > u32::from(planes) * 3 - 2 {
            return Err(bad("code-block pass count"));
        }
        let raw = style & 1 != 0 && pass >= 10 && pass % 3 != 0;
        let mut arithmetic = if raw {
            None
        } else {
            let mut m = Mq::new(if segment.data.is_empty() {
                &[255]
            } else {
                &segment.data
            })
            .map_err(|_| {
                bad(&format!(
                    "arithmetic segment at pass {pass}: {} bytes",
                    segment.data.len()
                ))
            })?;
            m.state = states;
            m.mps = mps;
            Some(m)
        };
        let mut bypass = Raw {
            data: &segment.data,
            pos: 0,
            left: 0,
            last: 0,
        };
        for _ in 0..segment.passes {
            let plane = planes - 1 - ((pass + 2) / 3) as u8;
            let coder: &mut dyn Entropy = if let Some(m) = &mut arithmetic {
                m
            } else {
                &mut bypass
            };
            // The small adapter keeps the existing monomorphized legacy paths intact.
            struct Coder<'a>(&'a mut dyn Entropy);
            impl Entropy for Coder<'_> {
                fn bit(&mut self, cx: usize) -> bool {
                    self.0.bit(cx)
                }
                fn raw(&self) -> bool {
                    self.0.raw()
                }
            }
            let mut coder = Coder(coder);
            match pass % 3 {
                0 => {
                    block.cleanup(&mut coder, plane);
                    if style & 32 != 0 {
                        let mut symbol = 0;
                        for _ in 0..4 {
                            symbol = symbol * 2 + u8::from(coder.bit(18));
                        }
                        if symbol != 10 {
                            return Err(bad("segmentation symbol"));
                        }
                    }
                }
                1 => block.propagation(&mut coder, plane),
                _ => block.refinement(&mut coder, plane),
            }
            pass += 1;
            if style & 2 != 0 {
                if let Some(m) = &mut arithmetic {
                    m.state = [0; 19];
                    m.state[0] = 4;
                    m.state[17] = 3;
                    m.state[18] = 46;
                    m.mps = [false; 19];
                }
            }
        }
        if let Some(m) = arithmetic {
            states = m.state;
            mps = m.mps;
        }
    }
    for i in 0..n {
        if block.sig[i] {
            block.value[i] = 2 * block.value[i] + (1 << block.last_plane[i]);
        }
        if block.negative[i] {
            block.value[i] = -block.value[i];
        }
    }
    Ok(block.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_code_block_area() {
        for (w, h) in [(0, 4), (4, 0), (1025, 4), (4, 1025), (64, 65), (1024, 5)] {
            let error = decode(&[], w, h, 0, 1).unwrap_err().to_string();
            assert!(error.contains("geometry"), "{w}x{h}: {error}");
        }
        // Cropped dimensions need not be powers of two, but stay within area.
        for (w, h) in [(64, 64), (1024, 4), (4, 1024), (63, 65)] {
            let error = decode(&[], w, h, 0, 1).unwrap_err().to_string();
            assert!(error.contains("arithmetic"), "{w}x{h}: {error}");
        }
    }
}
