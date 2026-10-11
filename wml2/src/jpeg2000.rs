//! Experimental JPEG 2000 decoder: unsigned Gray8, <=1024x1024, reversible 5/3,
//! zero to five decompositions, one tile/layer, LRCP, 16x16 blocks, style zero.
//! Unsupported profiles return errors before any image callback.
use crate::draw::{DecodeOptions, InitOptions, ResponseCommand};
use crate::error::{ImgError, ImgErrorKind};
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;
#[path = "jpeg2000_tier1.rs"]
mod tier1;
type Error = Box<dyn std::error::Error>;
fn bad(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::IllegalData,
        format!("JPEG2000: {message}"),
    ))
}
fn unsupported(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::UnsupportedFeature,
        format!("JPEG2000: {message}"),
    ))
}
fn bytes(data: &[u8], p: usize, n: usize) -> Result<&[u8], Error> {
    data.get(p..p.checked_add(n).ok_or_else(|| bad("length overflow"))?)
        .ok_or_else(|| bad("truncated input"))
}
fn u16be(data: &[u8], p: usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(bytes(data, p, 2)?.try_into()?))
}
fn u32be(data: &[u8], p: usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(bytes(data, p, 4)?.try_into()?))
}
const SIGNATURE: &[u8] = b"\x00\x00\x00\x0cjP  \x0d\x0a\x87\x0a";
fn box_at(data: &[u8], p: usize) -> Result<(&[u8], &[u8], usize), Error> {
    let size = u32be(data, p)? as usize;
    let kind = bytes(data, p + 4, 4)?;
    if size == 1 {
        return Err(unsupported("extended box lengths"));
    }
    let size = if size == 0 && kind == b"jp2c" {
        data.len() - p
    } else {
        size
    };
    if size < 8 {
        return Err(bad("invalid box length"));
    }
    let b = bytes(data, p, size)?;
    Ok((kind, &b[8..], p + size))
}
type Codestream<'a> = (&'a [u8], Option<(usize, usize)>);
fn container(data: &[u8]) -> Result<Codestream<'_>, Error> {
    if data.starts_with(&[255, 79]) {
        return Ok((data, None));
    }
    if !data.starts_with(SIGNATURE) {
        return Err(bad("missing signature"));
    }
    let mut p = 12;
    let mut brand = false;
    let mut dimensions = None;
    let mut color = false;
    let mut codestream = None;
    while p < data.len() {
        let (kind, b, next) = box_at(data, p)?;
        match kind {
            b"ftyp" => {
                if brand || p != 12 || b.len() < 12 || (b.len() - 8) % 4 != 0 {
                    return Err(bad("invalid file type box"));
                }
                if &b[..4] != b"jp2 " || !b[8..].chunks_exact(4).any(|b| b == b"jp2 ") {
                    return Err(unsupported("JP2 brand"));
                }
                brand = true;
            }
            b"jp2h" => {
                if !brand || dimensions.is_some() || codestream.is_some() {
                    return Err(bad("invalid JP2 header order"));
                }
                let mut q = 0;
                while q < b.len() {
                    let (k, h, n) = box_at(b, q)?;
                    match k {
                        b"ihdr" => {
                            if dimensions.is_some() || q != 0 || h.len() != 14 {
                                return Err(bad("invalid image header"));
                            }
                            if u16be(h, 8)? != 1
                                || h[10] != 7
                                || h[11] != 7
                                || h[12] != 0
                                || h[13] != 0
                            {
                                return Err(unsupported("JP2 image header profile"));
                            }
                            dimensions = Some((u32be(h, 4)? as usize, u32be(h, 0)? as usize));
                        }
                        b"colr" => {
                            if color || h.len() != 7 {
                                return Err(bad("invalid colour box"));
                            }
                            if h[..3] != [1, 0, 0] || u32be(h, 3)? != 17 {
                                return Err(unsupported("only enumerated grayscale colour"));
                            }
                            color = true;
                        }
                        _ => return Err(unsupported("JP2 header box")),
                    }
                    q = n;
                }
                if dimensions.is_none() || !color {
                    return Err(bad("incomplete JP2 header"));
                }
            }
            b"jp2c" => {
                if !brand || dimensions.is_none() || !color || codestream.is_some() {
                    return Err(bad("invalid codestream box order"));
                }
                codestream = Some(b);
            }
            b"free" => crate::limits::charge_metadata(b.len())?,
            _ => return Err(unsupported("JP2 top-level box")),
        }
        p = next;
    }
    Ok((
        codestream.ok_or_else(|| bad("missing codestream"))?,
        dimensions,
    ))
}
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    left: u8,
    last: u8,
}
impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            left: 0,
            last: 0,
        }
    }
    fn bit(&mut self) -> Result<bool, Error> {
        if self.left == 0 {
            let b = *self
                .data
                .get(self.pos)
                .ok_or_else(|| bad("truncated packet header"))?;
            self.pos += 1;
            self.left = if self.last == 255 {
                if b & 128 != 0 {
                    return Err(bad("packet header stuffing"));
                }
                7
            } else {
                8
            };
            self.last = b;
        }
        self.left -= 1;
        Ok(self.last & (1 << self.left) != 0)
    }
    fn number(&mut self, n: u32) -> Result<u32, Error> {
        if n > 32 {
            return Err(bad("packet field overflow"));
        }
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | u32::from(self.bit()?);
        }
        Ok(v)
    }
    fn align(&mut self) -> Result<(), Error> {
        self.left = 0;
        if self.last == 255 {
            let b = *self
                .data
                .get(self.pos)
                .ok_or_else(|| bad("missing stuffed header byte"))?;
            if b & 128 != 0 {
                return Err(bad("packet header stuffing"));
            }
            self.pos += 1;
        }
        Ok(())
    }
    fn passes(&mut self) -> Result<u32, Error> {
        if !self.bit()? {
            return Ok(1);
        }
        if !self.bit()? {
            return Ok(2);
        }
        let n = self.number(2)?;
        if n != 3 {
            return Ok(3 + n);
        }
        let n = self.number(5)?;
        if n != 31 {
            return Ok(6 + n);
        }
        Ok(37 + self.number(7)?)
    }
}
// Stateful single-layer tag trees. Each node stores the lowest possible value
// and its decoded value; parent information is shared by successive leaves.
#[derive(Clone, Copy)]
struct TagNode {
    low: u8,
    value: u8,
}
struct TagTree {
    levels: Vec<(usize, usize, Vec<TagNode>)>,
}
impl TagTree {
    fn new(mut w: usize, mut h: usize) -> Self {
        let mut levels = Vec::new();
        loop {
            levels.push((
                w,
                h,
                vec![
                    TagNode {
                        low: 0,
                        value: u8::MAX
                    };
                    w * h
                ],
            ));
            if w == 1 && h == 1 {
                break;
            }
            w = w.div_ceil(2);
            h = h.div_ceil(2);
        }
        Self { levels }
    }
    fn below(
        &mut self,
        x: usize,
        y: usize,
        threshold: u8,
        bits: &mut Bits<'_>,
    ) -> Result<bool, Error> {
        let mut low = 0;
        for depth in (0..self.levels.len()).rev() {
            let (w, _, nodes) = &mut self.levels[depth];
            let node = &mut nodes[(y >> depth) * *w + (x >> depth)];
            node.low = node.low.max(low);
            while node.low < threshold && node.low < node.value {
                if bits.bit()? {
                    node.value = node.low;
                } else {
                    node.low += 1;
                }
            }
            low = node.low.min(node.value);
        }
        let (w, _, nodes) = &self.levels[0];
        Ok(nodes[y * *w + x].value < threshold)
    }
}
// Preserve the already-verified small-image parser and its allocation behavior.
// Large-image packets use shared tag trees and independently positioned blocks.
fn packets(data: &[u8], w: usize, h: usize, levels: u8, mb: &[u8]) -> Result<Vec<i32>, Error> {
    if w <= 16 && h <= 16 && levels <= 1 {
        packets_single(data, w, h, levels, mb)
    } else {
        packets_multiple(data, w, h, levels, mb)
    }
}
fn packets_single(
    data: &[u8],
    w: usize,
    h: usize,
    levels: u8,
    mb: &[u8],
) -> Result<Vec<i32>, Error> {
    let mut plane = vec![0; w * h];
    let lw = if levels == 0 { w } else { w.div_ceil(2) };
    let lh = if levels == 0 { h } else { h.div_ceil(2) };
    let bands = [
        (0, 0, lw, lh, 0),
        (lw, 0, w / 2, lh, 1),
        (0, lh, lw, h / 2, 2),
        (lw, lh, w / 2, h / 2, 3),
    ];
    let mut p = 0;
    for resolution in 0..=levels {
        let indices: &[usize] = if resolution == 0 { &[0] } else { &[1, 2, 3] };
        let mut bits = Bits::new(&data[p..]);
        let mut blocks = Vec::new();
        if bits.bit()? {
            for &b in indices {
                let (_, _, bw, bh, _) = bands[b];
                if bw == 0 || bh == 0 {
                    continue;
                }
                if !bits.bit()? {
                    continue;
                }
                let mut missing = 0u8;
                while !bits.bit()? {
                    missing += 1;
                    if missing >= mb[b] {
                        return Err(bad("invalid zero bit-plane tag"));
                    }
                }
                let planes = mb[b] - missing;
                let passes = bits.passes()?;
                if passes != u32::from(planes) * 3 - 2 {
                    return Err(unsupported("partial code-block passes"));
                }
                let mut lblock = 3;
                while bits.bit()? {
                    lblock += 1;
                    if lblock > 32 {
                        return Err(bad("code-block length overflow"));
                    }
                }
                let length = bits.number(lblock + passes.ilog2())? as usize;
                if length == 0 {
                    return Err(bad("empty arithmetic segment"));
                }
                blocks.push((b, planes, length));
            }
        }
        bits.align()?;
        p += bits.pos;
        for (b, planes, length) in blocks {
            let (x, y, bw, bh, orientation) = bands[b];
            let values = tier1::decode(bytes(data, p, length)?, bw, bh, orientation, planes)?;
            p += length;
            for row in 0..bh {
                plane[(y + row) * w + x..(y + row) * w + x + bw]
                    .copy_from_slice(&values[row * bw..(row + 1) * bw]);
            }
        }
    }
    if p != data.len() {
        return Err(bad("trailing tile data"));
    }
    Ok(plane)
}
fn packets_multiple(
    data: &[u8],
    w: usize,
    h: usize,
    levels: u8,
    mb: &[u8],
) -> Result<Vec<i32>, Error> {
    let mut plane = vec![0; w * h];
    // QCD and LRCP order: deepest LL, then HL/LH/HH from coarse to fine.
    // All bands occupy disjoint rectangles in a nested quadrant layout.
    let factor = 1usize << levels;
    let mut bands = Vec::with_capacity(1 + 3 * usize::from(levels));
    bands.push((0, 0, w.div_ceil(factor), h.div_ceil(factor), 0));
    for depth in (1..=levels).rev() {
        let rw = w.div_ceil(1usize << (depth - 1));
        let rh = h.div_ceil(1usize << (depth - 1));
        let lw = rw.div_ceil(2);
        let lh = rh.div_ceil(2);
        bands.extend([
            (lw, 0, rw / 2, lh, 1),
            (0, lh, lw, rh / 2, 2),
            (lw, lh, rw / 2, rh / 2, 3),
        ]);
    }
    let mut p = 0;
    for resolution in 0..=levels {
        let first = if resolution == 0 {
            0
        } else {
            usize::from(resolution) * 3 - 2
        };
        let count = if resolution == 0 { 1 } else { 3 };
        let mut bits = Bits::new(&data[p..]);
        let mut blocks = Vec::new();
        if bits.bit()? {
            for b in first..first + count {
                let (bx, by, bw, bh, orientation) = bands[b];
                if bw == 0 || bh == 0 {
                    continue;
                }
                let nx = bw.div_ceil(16);
                let ny = bh.div_ceil(16);
                // Keep the existing single-block path free of tree allocations.
                let multiple = nx > 1 || ny > 1;
                let mut inclusion = multiple.then(|| TagTree::new(nx, ny));
                let mut zero_planes = multiple.then(|| TagTree::new(nx, ny));
                for cy in 0..ny {
                    for cx in 0..nx {
                        let included = match &mut inclusion {
                            Some(tree) => tree.below(cx, cy, 1, &mut bits)?,
                            None => bits.bit()?,
                        };
                        if !included {
                            continue;
                        }
                        let mut threshold = 1;
                        while !match &mut zero_planes {
                            Some(tree) => tree.below(cx, cy, threshold, &mut bits)?,
                            None => bits.bit()?,
                        } {
                            if threshold >= mb[b] {
                                return Err(bad("invalid zero bit-plane tag"));
                            }
                            threshold += 1;
                        }
                        let planes = mb[b] - (threshold - 1);
                        let passes = bits.passes()?;
                        if passes != u32::from(planes) * 3 - 2 {
                            return Err(unsupported("partial code-block passes"));
                        }
                        let mut lblock = 3;
                        while bits.bit()? {
                            lblock += 1;
                            if lblock > 32 {
                                return Err(bad("code-block length overflow"));
                            }
                        }
                        let length = bits.number(lblock + passes.ilog2())? as usize;
                        if length == 0 {
                            return Err(bad("empty arithmetic segment"));
                        }
                        blocks.push((
                            bx + cx * 16,
                            by + cy * 16,
                            (bw - cx * 16).min(16),
                            (bh - cy * 16).min(16),
                            orientation,
                            planes,
                            length,
                        ));
                    }
                }
            }
        }
        bits.align()?;
        p += bits.pos;
        for (x, y, bw, bh, orientation, planes, length) in blocks {
            let values = tier1::decode(bytes(data, p, length)?, bw, bh, orientation, planes)?;
            p += length;
            for row in 0..bh {
                plane[(y + row) * w + x..(y + row) * w + x + bw]
                    .copy_from_slice(&values[row * bw..(row + 1) * bw]);
            }
        }
    }
    if p != data.len() {
        return Err(bad("trailing tile data"));
    }
    Ok(plane)
}
// Whole-sample symmetric extension. div_euclid implements the required floor
// for negative coefficients, unlike Rust's truncating integer division.
fn inverse_line(packed: &[i32]) -> Vec<i32> {
    let n = packed.len();
    if n < 2 {
        return packed.to_vec();
    }
    let low = n.div_ceil(2);
    let mut out = vec![0; n];
    for i in 0..low {
        out[2 * i] = packed[i];
    }
    for i in 0..n / 2 {
        out[2 * i + 1] = packed[low + i];
    }
    for i in (0..n).step_by(2) {
        let l = if i == 0 { 1 } else { i - 1 };
        let r = if i + 1 == n { i - 1 } else { i + 1 };
        out[i] -= (out[l] + out[r] + 2).div_euclid(4);
    }
    for i in (1..n).step_by(2) {
        let r = if i + 1 == n { i - 1 } else { i + 1 };
        out[i] += (out[i - 1] + out[r]).div_euclid(2);
    }
    out
}
fn inverse53(plane: &mut [i32], w: usize, h: usize) {
    for y in 0..h {
        let row = inverse_line(&plane[y * w..(y + 1) * w]);
        plane[y * w..(y + 1) * w].copy_from_slice(&row);
    }
    for x in 0..w {
        let column: Vec<_> = (0..h).map(|y| plane[y * w + x]).collect();
        let column = inverse_line(&column);
        for y in 0..h {
            plane[y * w + x] = column[y];
        }
    }
}
// New multilevel synthesis uses wide intermediates and checked storage.
// Legacy zero/one-level synthesis is retained above for its proven fast path.
fn inverse_line_checked(packed: &[i32]) -> Result<Vec<i32>, Error> {
    let n = packed.len();
    let low = n.div_ceil(2);
    let mut out = vec![0i64; n];
    for i in 0..low {
        out[2 * i] = i64::from(packed[i]);
    }
    for i in 0..n / 2 {
        out[2 * i + 1] = i64::from(packed[low + i]);
    }
    if n > 1 {
        for i in (0..n).step_by(2) {
            let left = if i == 0 { 1 } else { i - 1 };
            let right = if i + 1 == n { i - 1 } else { i + 1 };
            out[i] -= (out[left] + out[right] + 2).div_euclid(4);
        }
        for i in (1..n).step_by(2) {
            let right = if i + 1 == n { i - 1 } else { i + 1 };
            out[i] += (out[i - 1] + out[right]).div_euclid(2);
        }
    }
    out.into_iter()
        .map(|v| i32::try_from(v).map_err(|_| bad("wavelet coefficient overflow")))
        .collect()
}
// Keep checked multilevel synthesis out of the legacy decode hot path.
#[inline(never)]
fn inverse53_multilevel(
    plane: &mut [i32],
    stride: usize,
    height: usize,
    levels: u8,
) -> Result<(), Error> {
    // Grow the reconstructed LL rectangle from deepest resolution to the canvas.
    // Region row indexing always uses the full canvas stride.
    for depth in (1..=levels).rev() {
        let rw = stride.div_ceil(1usize << (depth - 1));
        let rh = height.div_ceil(1usize << (depth - 1));
        for y in 0..rh {
            let row = inverse_line_checked(&plane[y * stride..y * stride + rw])?;
            plane[y * stride..y * stride + rw].copy_from_slice(&row);
        }
        for x in 0..rw {
            let packed: Vec<_> = (0..rh).map(|y| plane[y * stride + x]).collect();
            let column = inverse_line_checked(&packed)?;
            for y in 0..rh {
                plane[y * stride + x] = column[y];
            }
        }
    }
    Ok(())
}
fn gray8(coefficient: i32) -> u8 {
    (coefficient.clamp(-128, 127) + 128) as u8
}
fn image(data: &[u8]) -> Result<(usize, usize, Vec<u8>), Error> {
    let (s, dimensions) = container(data)?;
    if !s.starts_with(&[255, 79]) || u16be(s, 2)? != 0xff51 {
        return Err(bad("SOC/SIZ required"));
    }
    let mut p = 2;
    let mut size = None;
    let mut coding = None;
    let mut quant = None;
    loop {
        let marker = u16be(s, p)?;
        let length = usize::from(u16be(s, p + 2)?);
        if length < 2 {
            return Err(bad("marker length"));
        }
        let b = bytes(s, p + 4, length - 2)?;
        match marker {
            0xff51 => {
                if size.is_some() || b.len() != 39 {
                    return Err(bad("invalid SIZ"));
                }
                let w = u32be(b, 2)? as usize;
                let h = u32be(b, 6)? as usize;
                if w == 0 || h == 0 {
                    return Err(bad("zero dimensions"));
                }
                if u16be(b, 0)? != 0
                    || w > 1024
                    || h > 1024
                    || u32be(b, 10)? != 0
                    || u32be(b, 14)? != 0
                    || u32be(b, 18)? as usize != w
                    || u32be(b, 22)? as usize != h
                    || u32be(b, 26)? != 0
                    || u32be(b, 30)? != 0
                    || u16be(b, 34)? != 1
                    || b[36..] != [7, 1, 1]
                {
                    return Err(unsupported(
                        "SIZ: Gray8, zero origin, one tile, dimensions <=1024 required",
                    ));
                }
                if dimensions.is_some_and(|d| d != (w, h)) {
                    return Err(bad("JP2 and SIZ dimensions differ"));
                }
                let limits = crate::limits::current();
                crate::limits::check(w * h, limits.pixels, "pixels")?;
                crate::limits::check(
                    w * h * 64 + 8192,
                    limits.expanded_bytes,
                    "JPEG2000 workspace",
                )?;
                crate::limits::check(1, limits.frames, "frames")?;
                size = Some((w, h));
            }
            0xff52 => {
                if coding.is_some() || b.len() != 10 {
                    return Err(bad("invalid COD"));
                }
                if b[0] != 0
                    || b[1] != 0
                    || u16be(b, 2)? != 1
                    || b[4] != 0
                    || b[5] > 5
                    || b[6] != 2
                    || b[7] != 2
                    || b[8] != 0
                    || b[9] != 1
                {
                    return Err(unsupported(
                        "COD: one layer, LRCP, 0..5 levels, 16x16 block, style zero, reversible transform required",
                    ));
                }
                coding = Some(b[5]);
            }
            0xff5c => {
                if quant.is_some() || b.is_empty() {
                    return Err(bad("invalid QCD"));
                }
                quant = Some(b);
            }
            0xff64 => {
                if b.len() < 2 {
                    return Err(bad("invalid COM"));
                }
                crate::limits::charge_metadata(b.len())?;
            }
            0xff90 => {
                if b.len() != 8 || u16be(b, 0)? != 0 || b[6] != 0 || b[7] != 1 {
                    return Err(unsupported("one tile-part required"));
                }
                let (w, h) = size.ok_or_else(|| bad("missing SIZ"))?;
                let levels = coding.ok_or_else(|| bad("missing COD"))?;
                if w < (1usize << levels) || h < (1usize << levels) {
                    return Err(unsupported(
                        "each dimension must be at least 2^decomposition levels",
                    ));
                }
                let q = quant.ok_or_else(|| bad("missing QCD"))?;
                let bands = 1 + 3 * usize::from(levels);
                if q.len() != bands + 1 {
                    return Err(bad("QCD band count"));
                }
                if q[0] & 31 != 0 {
                    return Err(unsupported("quantization"));
                }
                let mut mb = Vec::new();
                for &qband in &q[1..] {
                    if qband & 7 != 0 {
                        return Err(bad("QCD reserved bits"));
                    }
                    let depth = (q[0] >> 5)
                        .checked_add(qband >> 3)
                        .and_then(|v| v.checked_sub(1))
                        .filter(|&v| v > 0 && v <= 16)
                        .ok_or_else(|| unsupported("coefficient precision"))?;
                    mb.push(depth);
                }
                let part = u32be(b, 2)? as usize;
                if part < 14 {
                    return Err(bad("tile-part size"));
                }
                let tile = bytes(s, p, part)?;
                let end = p + part;
                if bytes(s, end, 2)? != [255, 217]
                    || end + 2 != s.len()
                    || tile[12..14] != [255, 147]
                {
                    return Err(bad("tile boundary, SOD or EOC"));
                }
                let mut plane = packets(&tile[14..], w, h, levels, &mb)?;
                if levels == 1 {
                    inverse53(&mut plane, w, h);
                } else if levels > 1 {
                    inverse53_multilevel(&mut plane, w, h, levels)?;
                }
                let mut rgba = Vec::with_capacity(w * h * 4);
                for v in plane {
                    // Clamp before shifting: equivalent to a wide DC shift, without overflow.
                    let v = gray8(v);
                    rgba.extend_from_slice(&[v, v, v, 255]);
                }
                return Ok((w, h, rgba));
            }
            _ => return Err(unsupported("codestream marker")),
        }
        p += length + 2;
    }
}
/// Decode the supported small lossless grayscale profile into ordinary RGBA8.
pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let length = end.checked_sub(start).ok_or_else(|| bad("reader extent"))?;
    crate::limits::check_input_length(length, crate::limits::current().input_bytes)?;
    let data = reader.read_bytes_as_vec(usize::try_from(length)?)?;
    let (w, h, rgba) = image(&data)?;
    let response = option.drawer.init(w, h, InitOptions::new())?;
    if !response.is_some_and(|r| r.response == ResponseCommand::Abort) {
        option.drawer.draw(0, 0, w, h, &rgba, None)?;
    }
    option.drawer.terminate(None)?;
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_tag_tree_values() {
        let mut bits = Bits::new(&[0xca, 0x20]);
        let mut tree = TagTree::new(2, 2);
        for (i, value) in [0, 2, 1, 3].into_iter().enumerate() {
            for threshold in 1..=value {
                assert!(!tree.below(i % 2, i / 2, threshold, &mut bits).unwrap());
            }
            assert!(tree.below(i % 2, i / 2, value + 1, &mut bits).unwrap());
        }
        assert_eq!((bits.pos, bits.left), (2, 5));
        let mut bits = Bits::new(&[0xc8]);
        let mut tree = TagTree::new(2, 2);
        for (i, included) in [true, false, false, true].into_iter().enumerate() {
            assert_eq!(tree.below(i % 2, i / 2, 1, &mut bits).unwrap(), included);
        }
        assert_eq!(bits.left, 3);
    }
    #[test]
    fn checked_lifting_matches_and_detects_overflow() {
        for width in 1..=33 {
            let packed: Vec<_> = (0..width)
                .map(|x| ((x * 73 + width * 11) % 257) as i32 - 128)
                .collect();
            assert_eq!(
                inverse_line_checked(&packed).unwrap(),
                inverse_line(&packed)
            );
        }
        assert_eq!(inverse_line_checked(&[-5, 0, -3]).unwrap(), vec![-4, -5, 1]);
        assert_eq!(inverse_line_checked(&[i32::MIN]).unwrap(), vec![i32::MIN]);
        for packed in [[i32::MAX, i32::MAX], [i32::MIN, i32::MIN]] {
            assert!(
                inverse_line_checked(&packed)
                    .unwrap_err()
                    .to_string()
                    .contains("overflow")
            );
        }
    }
    #[test]
    fn dc_shift_extremes_match_wide_reference() {
        for v in [
            i32::MIN,
            -1024,
            -129,
            -128,
            -127,
            -1,
            0,
            126,
            127,
            128,
            1024,
            i32::MAX,
        ] {
            assert_eq!(gray8(v), (i64::from(v) + 128).clamp(0, 255) as u8);
        }
    }
    #[test]
    fn lifting_edges() {
        assert_eq!(inverse_line(&[13, 2]), vec![12, 14]);
        assert_eq!(inverse_line(&[-5, 0, -3]), vec![-4, -5, 1]);
        assert_eq!(inverse_line(&[7]), vec![7]);
    }
}
