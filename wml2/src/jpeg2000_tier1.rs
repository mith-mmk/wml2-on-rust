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
struct Block {
    w: usize,
    h: usize,
    orientation: u8,
    value: Vec<i32>,
    sig: Vec<bool>,
    negative: Vec<bool>,
    visited: Vec<bool>,
    refined: Vec<bool>,
}
impl Block {
    fn neighbor(&self, x: usize, y: usize, dx: isize, dy: isize) -> Option<usize> {
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
    fn make_significant(&mut self, mq: &mut Mq<'_>, x: usize, y: usize, plane: u8) {
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
        self.negative[i] = mq.bit(cx) ^ prediction;
        self.sig[i] = true;
        self.value[i] = 1 << plane;
    }
    fn propagation(&mut self, mq: &mut Mq<'_>, plane: u8) {
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
    fn refinement(&mut self, mq: &mut Mq<'_>, plane: u8) {
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
                    }
                }
            }
        }
    }
    fn cleanup(&mut self, mq: &mut Mq<'_>, plane: u8) {
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
    if w == 0 || h == 0 || w > 16 || h > 16 || planes == 0 || planes > 16 {
        return Err(bad("invalid code-block geometry or bit depth"));
    }
    let n = w * h;
    let mut block = Block {
        w,
        h,
        orientation,
        value: vec![0; n],
        sig: vec![false; n],
        negative: vec![false; n],
        visited: vec![false; n],
        refined: vec![false; n],
    };
    let mut mq = Mq::new(data)?;
    block.cleanup(&mut mq, planes - 1);
    for plane in (0..planes - 1).rev() {
        block.propagation(&mut mq, plane);
        block.refinement(&mut mq, plane);
        block.cleanup(&mut mq, plane);
    }
    for i in 0..n {
        if block.negative[i] {
            block.value[i] = -block.value[i];
        }
    }
    Ok(block.value)
}
