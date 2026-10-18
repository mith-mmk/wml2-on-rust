//! Bounded 1-8bit JPEG 2000 decoder enabled by the `jpeg2000` feature.
//!
//! Raw J2K and Gray/sRGB/sYCC JP2 produce ordinary RGBA8, including alpha.
//! The established small Gray8/RGB8 path is tried first. Its unsupported profiles
//! retry the general component/tile/packet assembler, supporting multiple layers,
//! five progressions, precincts, subsampling, palette, ROI and HTONLY HTJ2K.
//! EX additionally retains unsigned Gray/RGB12/16 and explicitly declared
//! straight RGBA12/16 with reversible 5/3/RCT or bounded 9/7/ICT. Signed, mixed
//! precision, implicit/premultiplied alpha and ICC
//! remain unsupported.
//! Geometry, coefficient precision and resource budgets remain bounded.
//! All declared tiles and layer packets must be present; complete packets end
//! each tile-part. Only the final codestream part may use Psot=0.
//! Decoding completes before image callbacks. See the current supported
//! profile and validation overview in `docs/jpeg2000.md`.
use crate::draw::{DecodeOptions, InitOptions, ResponseCommand};
use crate::error::{ImgError, ImgErrorKind};
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
use std::io::SeekFrom;
#[path = "jpeg2000_general.rs"]
mod general;
#[path = "jpeg2000_ht.rs"]
mod ht;
#[path = "jpeg2000_irreversible.rs"]
mod irreversible;
#[path = "jpeg2000_tier1.rs"]
mod tier1;
use irreversible::{quant_irreversible, tile_pixels_irreversible};
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
    let size = u32be(data, p)?;
    let kind = bytes(data, p + 4, 4)?;
    let (size, header) = match size {
        0 => (data.len() - p, 8),
        1 => (
            usize::try_from(u64::from_be_bytes(bytes(data, p + 8, 8)?.try_into()?))?,
            16,
        ),
        n => (usize::try_from(n)?, 8),
    };
    if size < header {
        return Err(bad("invalid box length"));
    }
    let b = bytes(data, p, size)?;
    Ok((kind, &b[header..], p + size))
}
fn resolution_box(data: &[u8]) -> Result<(), Error> {
    let mut p = 0;
    let mut seen = [false; 2];
    while p < data.len() {
        if u32be(data, p)? == 0 {
            return Err(bad("zero-length resolution child"));
        }
        let (kind, b, next) = box_at(data, p)?;
        if let Some(index) = match kind {
            b"resc" => Some(0),
            b"resd" => Some(1),
            _ => None,
        } {
            if seen[index] || b.len() != 10 || u16be(b, 2)? == 0 || u16be(b, 6)? == 0 {
                return Err(bad("invalid resolution box"));
            }
            seen[index] = true;
        }
        p = next;
    }
    if !seen.iter().any(|&value| value) {
        return Err(bad("empty resolution box"));
    }
    Ok(())
}
type Codestream<'a> = (&'a [u8], Option<(usize, usize, usize)>);
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
    let mut color = None;
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
                let mut resolution = false;
                while q < b.len() {
                    // A zero-length nested box must extend to EOF along with
                    // its parent; jp2h must instead end before the codestream.
                    if u32be(b, q)? == 0 {
                        return Err(bad("zero-length JP2 header child"));
                    }
                    let (k, h, n) = box_at(b, q)?;
                    match k {
                        b"ihdr" => {
                            if dimensions.is_some() || q != 0 || h.len() != 14 {
                                return Err(bad("invalid image header"));
                            }
                            if !matches!(u16be(h, 8)?, 1 | 3)
                                || h[10] != 7
                                || h[11] != 7
                                || h[12] > 1
                                || h[13] > 1
                            {
                                return Err(unsupported("JP2 image header profile"));
                            }
                            dimensions = Some((
                                u32be(h, 4)? as usize,
                                u32be(h, 0)? as usize,
                                usize::from(u16be(h, 8)?),
                            ));
                        }
                        b"colr" => {
                            if color.is_some() {
                                // T.800 I.5.3.3: later colour specifications
                                // do not replace the first understood one.
                                crate::limits::charge_metadata(h.len())?;
                                q = n;
                                continue;
                            }
                            if h.len() < 3 {
                                return Err(bad("invalid colour box"));
                            }
                            if h[0] != 1 {
                                if matches!(h[0], 2 | 3) {
                                    return Err(unsupported("ICC colour profile"));
                                }
                                crate::limits::charge_metadata(h.len())?;
                                q = n;
                                continue;
                            }
                            if h.len() != 7 {
                                return Err(bad("invalid colour box"));
                            }
                            let space = u32be(h, 3)?;
                            // PREC and APPROX are ignored by JP2 readers (I.5.3.3).
                            if !matches!(space, 16 | 17) {
                                return Err(unsupported(
                                    "only enumerated grayscale or sRGB colour",
                                ));
                            }
                            color = Some(space);
                        }
                        b"pclr" | b"cmap" | b"cdef" | b"bpcc" => {
                            return Err(unsupported("JP2 channel mapping or sample layout"));
                        }
                        b"res " => {
                            if resolution {
                                return Err(bad("duplicate resolution box"));
                            }
                            resolution_box(h)?;
                            resolution = true;
                            crate::limits::charge_metadata(h.len())?;
                        }
                        b"jp2h" | b"jp2c" | b"ftyp" | b"jP  " => {
                            return Err(bad("misplaced JP2 header box"));
                        }
                        _ => crate::limits::charge_metadata(h.len())?,
                    }
                    q = n;
                }
                if dimensions.is_none() || color.is_none() {
                    return Err(bad("incomplete JP2 header"));
                }
                let components = dimensions.unwrap().2;
                if color != Some(if components == 1 { 17 } else { 16 }) {
                    return Err(unsupported("JP2 colour and component count differ"));
                }
            }
            b"jp2c" => {
                if !brand || dimensions.is_none() || color.is_none() || codestream.is_some() {
                    return Err(bad("invalid codestream box order"));
                }
                codestream = Some(b);
            }
            b"jP  " | b"ihdr" | b"colr" | b"bpcc" | b"pclr" | b"cmap" | b"cdef" => {
                return Err(bad("misplaced JP2 structural box"));
            }
            _ => crate::limits::charge_metadata(b.len())?,
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
#[derive(Clone, Copy)]
struct PacketData<'a> {
    bytes: &'a [u8],
    part_ends: &'a [usize],
}
impl PacketData<'_> {
    fn packet_end(&self, p: usize, part: &mut usize) -> Result<(), Error> {
        if let Some(&end) = self.part_ends.get(*part) {
            if p > end {
                return Err(bad("tile-part boundary splits a packet"));
            }
            if p == end {
                *part += 1;
            }
        }
        Ok(())
    }
    fn finish(&self, p: usize, part: usize) -> Result<(), Error> {
        if p != self.bytes.len() {
            return Err(bad("trailing tile data"));
        }
        if part != self.part_ends.len() {
            return Err(bad("unconsumed tile-part boundary"));
        }
        Ok(())
    }
}
// Preserve the already-verified small-image parser and its allocation behavior.
// Large-image packets use shared tag trees and independently positioned blocks.
fn packets<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    levels: u8,
    mb: &[u8],
    block: (usize, usize),
) -> Result<Vec<i32>, Error> {
    if w <= 16 && h <= 16 && levels <= 1 && w <= block.0 && h <= block.1 {
        packets_single::<SPLIT>(input, w, h, levels, mb)
    } else {
        packets_multiple::<SPLIT>(input, w, h, levels, mb, block)
    }
}
fn packets_single<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    levels: u8,
    mb: &[u8],
) -> Result<Vec<i32>, Error> {
    let data = input.bytes;
    let mut part_index = 0;
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
        if SPLIT {
            input.packet_end(p, &mut part_index)?;
        }
    }
    if SPLIT {
        input.finish(p, part_index)?;
    } else if p != data.len() {
        return Err(bad("trailing tile data"));
    }
    Ok(plane)
}
fn packets_multiple<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    levels: u8,
    mb: &[u8],
    block: (usize, usize),
) -> Result<Vec<i32>, Error> {
    let mut plane = vec![0; w * h];
    packets_planes::<SPLIT>(
        input,
        w,
        h,
        levels,
        &[mb],
        std::slice::from_mut(&mut plane),
        block,
    )?;
    Ok(plane)
}
// LRCP orders each resolution's component packets before the next resolution.
// Components use independent tag trees and coefficient planes.
fn packets_planes<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    levels: u8,
    mb: &[&[u8]],
    planes: &mut [Vec<i32>],
    block: (usize, usize),
) -> Result<(), Error> {
    packets_planes_grid::<false, SPLIT, false>(
        input,
        w,
        h,
        levels,
        mb,
        planes,
        BlockGrid {
            block,
            origin: (0, 0),
        },
    )
}
type Band = (usize, usize, usize, usize, u8);
#[inline]
fn subbands<const GLOBAL: bool>(
    w: usize,
    h: usize,
    levels: u8,
    origin: (usize, usize),
) -> Vec<Band> {
    let factor = 1usize.checked_shl(u32::from(levels)).unwrap_or(usize::MAX);
    let mut bands = Vec::with_capacity(1 + 3 * usize::from(levels));
    let (x0, y0) = origin;
    let (x1, y1) = (x0 + w, y0 + h);
    let (llw, llh) = if GLOBAL {
        (
            x1.div_ceil(factor) - x0.div_ceil(factor),
            y1.div_ceil(factor) - y0.div_ceil(factor),
        )
    } else {
        (w.div_ceil(factor), h.div_ceil(factor))
    };
    bands.push((0, 0, llw, llh, 0));
    for depth in (1..=levels).rev() {
        let (rw, rh, lw, lh) = if GLOBAL {
            let parent = 1usize << (depth - 1);
            let child = 1usize.checked_shl(u32::from(depth)).unwrap_or(usize::MAX);
            (
                x1.div_ceil(parent) - x0.div_ceil(parent),
                y1.div_ceil(parent) - y0.div_ceil(parent),
                x1.div_ceil(child) - x0.div_ceil(child),
                y1.div_ceil(child) - y0.div_ceil(child),
            )
        } else {
            let rw = w.div_ceil(1usize << (depth - 1));
            let rh = h.div_ceil(1usize << (depth - 1));
            (rw, rh, rw.div_ceil(2), rh.div_ceil(2))
        };
        bands.extend([
            (lw, 0, rw - lw, lh, 1),
            (0, lh, lw, rh - lh, 2),
            (lw, lh, rw - lw, rh - lh, 3),
        ]);
    }
    bands
}
#[derive(Clone, Copy)]
struct BlockGrid {
    block: (usize, usize),
    origin: (usize, usize),
}
fn packets_planes_grid<const GLOBAL: bool, const SPLIT: bool, const IRREVERSIBLE: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    levels: u8,
    mb: &[&[u8]],
    planes: &mut [Vec<i32>],
    grid: BlockGrid,
) -> Result<(), Error> {
    let data = input.bytes;
    let mut part_index = 0;
    // QCD and LRCP order: deepest LL, then HL/LH/HH from coarse to fine.
    // All bands occupy disjoint rectangles in a nested quadrant layout.
    let (block_width, block_height) = grid.block;
    let (x0, y0) = grid.origin;
    let (x1, y1) = (x0 + w, y0 + h);
    let bands = subbands::<GLOBAL>(w, h, levels, grid.origin);
    let mut p = 0;
    for resolution in 0..=levels {
        let first = if resolution == 0 {
            0
        } else {
            usize::from(resolution) * 3 - 2
        };
        let count = if resolution == 0 { 1 } else { 3 };
        let depth = if resolution == 0 {
            levels
        } else {
            levels - resolution + 1
        };
        if GLOBAL {
            let factor = 1usize << (levels - resolution);
            // A zero-area resolution has no precincts or packets. A nonempty
            // resolution with empty high bands still has its packet header.
            if x1.div_ceil(factor) == x0.div_ceil(factor)
                || y1.div_ceil(factor) == y0.div_ceil(factor)
            {
                continue;
            }
        }
        for (plane, mb) in planes.iter_mut().zip(mb) {
            let mut bits = Bits::new(&data[p..]);
            let mut blocks = Vec::new();
            if bits.bit()? {
                for b in first..first + count {
                    let (bx, by, bw, bh, orientation) = bands[b];
                    if bw == 0 || bh == 0 {
                        continue;
                    }
                    // T.800 B.5/B.7: global grids; low/high sub-band
                    // coordinates differ when the tile origin is unaligned.
                    let (ox, oy) = if GLOBAL {
                        (
                            subband_start(x0, depth, orientation == 1 || orientation == 3)
                                % block_width,
                            subband_start(y0, depth, orientation == 2 || orientation == 3)
                                % block_height,
                        )
                    } else {
                        (0, 0)
                    };
                    let nx = (bw + ox).div_ceil(block_width);
                    let ny = (bh + oy).div_ceil(block_height);
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
                            if passes > u32::from(planes) * 3 - 2 {
                                return Err(bad("invalid code-block pass count"));
                            }
                            if !IRREVERSIBLE && passes != u32::from(planes) * 3 - 2 {
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
                            let (x, y, width, height) = if GLOBAL {
                                let x = (cx * block_width).saturating_sub(ox);
                                let y = (cy * block_height).saturating_sub(oy);
                                (
                                    x,
                                    y,
                                    ((cx + 1) * block_width - ox).min(bw) - x,
                                    ((cy + 1) * block_height - oy).min(bh) - y,
                                )
                            } else {
                                (
                                    cx * block_width,
                                    cy * block_height,
                                    (bw - cx * block_width).min(block_width),
                                    (bh - cy * block_height).min(block_height),
                                )
                            };
                            blocks.push((
                                bx + x,
                                by + y,
                                width,
                                height,
                                orientation,
                                planes,
                                passes,
                                length,
                            ));
                        }
                    }
                }
            }
            bits.align()?;
            p += bits.pos;
            for (x, y, bw, bh, orientation, planes, passes, length) in blocks {
                let values = if IRREVERSIBLE {
                    tier1::decode_irreversible(
                        bytes(data, p, length)?,
                        bw,
                        bh,
                        orientation,
                        planes,
                        passes,
                    )?
                } else {
                    tier1::decode(bytes(data, p, length)?, bw, bh, orientation, planes)?
                };
                p += length;
                for row in 0..bh {
                    plane[(y + row) * w + x..(y + row) * w + x + bw]
                        .copy_from_slice(&values[row * bw..(row + 1) * bw]);
                }
            }
            if SPLIT {
                input.packet_end(p, &mut part_index)?;
            }
        }
    }
    if SPLIT {
        input.finish(p, part_index)?;
    } else if p != data.len() {
        return Err(bad("trailing tile data"));
    }
    Ok(())
}
// ceil((origin - high*2^(depth-1))/2^depth), rewritten so the
// intermediate stays nonnegative even for a high-pass band at origin zero.
fn subband_start(origin: usize, depth: u8, high: bool) -> usize {
    let factor = 1usize << depth;
    let offset = if high { factor / 2 } else { factor };
    (origin + offset - 1) / factor
}
// T.800 F.3.6/F.3.8: global-even samples undo the update first, then
// global-odd samples undo prediction, using whole-sample reflection.
fn inverse_line_at(packed: &[i32], origin: usize) -> Result<Vec<i32>, Error> {
    let n = packed.len();
    if n <= 1 {
        // F.3.6 uses plain division for the odd singleton, unlike the floor
        // in lifting. Valid forward coefficients are even; tolerate mutated
        // negative odd coefficients with integer division toward zero.
        return Ok(packed
            .iter()
            .map(|&v| if origin % 2 == 1 { v / 2 } else { v })
            .collect());
    }
    let odd = origin % 2;
    let low = (n + 1 - odd) / 2;
    let mut out = vec![0i64; n];
    for (i, value) in out.iter_mut().enumerate() {
        let index = if (i + odd) & 1 == 0 {
            i / 2
        } else {
            low + i / 2
        };
        *value = i64::from(packed[index]);
    }
    for i in (odd..n).step_by(2) {
        let l = if i == 0 { 1 } else { i - 1 };
        let r = if i + 1 == n { i - 1 } else { i + 1 };
        out[i] -= (out[l] + out[r] + 2).div_euclid(4);
    }
    for i in (1 - odd..n).step_by(2) {
        let l = if i == 0 { 1 } else { i - 1 };
        let r = if i + 1 == n { i - 1 } else { i + 1 };
        out[i] += (out[l] + out[r]).div_euclid(2);
    }
    out.into_iter()
        .map(|v| i32::try_from(v).map_err(|_| bad("wavelet coefficient overflow")))
        .collect()
}
#[inline(never)]
fn synthesize_unaligned(
    plane: &mut [i32],
    w: usize,
    h: usize,
    levels: u8,
    origin: (usize, usize),
) -> Result<(), Error> {
    let (x0, y0) = origin;
    let (x1, y1) = (x0 + w, y0 + h);
    for depth in (1..=levels).rev() {
        let factor = 1usize << (depth - 1);
        let (rx, ry) = (x0.div_ceil(factor), y0.div_ceil(factor));
        let (rw, rh) = (x1.div_ceil(factor) - rx, y1.div_ceil(factor) - ry);
        if rw == 0 || rh == 0 {
            continue;
        }
        for y in 0..rh {
            let row = inverse_line_at(&plane[y * w..y * w + rw], rx)?;
            plane[y * w..y * w + rw].copy_from_slice(&row);
        }
        for x in 0..rw {
            let packed: Vec<_> = (0..rh).map(|y| plane[y * w + x]).collect();
            let column = inverse_line_at(&packed, ry)?;
            for y in 0..rh {
                plane[y * w + x] = column[y];
            }
        }
    }
    Ok(())
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
// ITU-T T.800 G.2.2: inverse RCT after wavelet synthesis and before DC shift.
// Floor division matters for negative chroma sums. Wide intermediates also
// handle legal entropy mutations yielding extreme reconstructed coefficients.
fn inverse_rct(y: i32, cb: i32, cr: i32) -> [u8; 3] {
    let green = i64::from(y) - (i64::from(cb) + i64::from(cr)).div_euclid(4);
    [green + i64::from(cr), green, green + i64::from(cb)].map(|v| (v + 128).clamp(0, 255) as u8)
}
fn synthesize(plane: &mut [i32], w: usize, h: usize, levels: u8) -> Result<(), Error> {
    if levels == 1 {
        inverse53(plane, w, h);
    } else if levels > 1 {
        inverse53_multilevel(plane, w, h, levels)?;
    }
    Ok(())
}
fn quant_planes(q: &[u8], levels: u8) -> Result<Vec<u8>, Error> {
    if q.len() != 2 + 3 * usize::from(levels) {
        return Err(bad("QCD band count"));
    }
    if q[0] & 31 != 0 {
        return Err(unsupported("quantization"));
    }
    q[1..]
        .iter()
        .map(|&band| {
            if band & 7 != 0 {
                return Err(bad("QCD reserved bits"));
            }
            (q[0] >> 5)
                .checked_add(band >> 3)
                .and_then(|v| v.checked_sub(1))
                .filter(|&v| v > 0 && v <= 16)
                .ok_or_else(|| unsupported("coefficient precision"))
        })
        .collect()
}
#[derive(Clone, Copy)]
struct Size {
    w: usize,
    h: usize,
    components: usize,
    tile_w: usize,
    tile_h: usize,
    image_origin: (usize, usize),
    tile_origin: (usize, usize),
}
#[derive(Clone, Copy)]
struct Coding {
    levels: u8,
    mct: bool,
    irreversible: bool,
    block: (usize, usize),
    precincts: Option<[u8; 6]>,
}
fn single_precinct(
    coding: Coding,
    w: usize,
    h: usize,
    origin: (usize, usize),
) -> Result<(), Error> {
    let Some(precincts) = coding.precincts else {
        return Ok(());
    };
    for resolution in 0..=coding.levels {
        let factor = 1usize << (coding.levels - resolution);
        let (x0, y0) = (origin.0.div_ceil(factor), origin.1.div_ceil(factor));
        let (x1, y1) = (
            (origin.0 + w).div_ceil(factor),
            (origin.1 + h).div_ceil(factor),
        );
        if x0 == x1 || y0 == y1 {
            continue;
        }
        let entry = precincts[usize::from(resolution)];
        let (px, py) = (entry & 15, entry >> 4);
        let shift = u32::from(resolution != 0);
        // Keep exactly the existing code-block grid, with one packet for each
        // nonempty component/resolution. Smaller or crossing precincts need
        // a different packet iterator and remain explicitly unsupported.
        if (x0 >> px) != ((x1 - 1) >> px)
            || (y0 >> py) != ((y1 - 1) >> py)
            || (1usize << px) < coding.block.0 << shift
            || (1usize << py) < coding.block.1 << shift
        {
            return Err(unsupported(
                "multiple precincts or precinct-clipped code-blocks",
            ));
        }
    }
    Ok(())
}
fn tile_pixels<const SPLIT: bool>(
    input: PacketData<'_>,
    w: usize,
    h: usize,
    origin: (usize, usize),
    coding: Coding,
    component_mb: &[Vec<u8>],
    component_steps: &[Vec<f64>],
) -> Result<Vec<u8>, Error> {
    single_precinct(coding, w, h, origin)?;
    if coding.irreversible {
        return tile_pixels_irreversible::<SPLIT>(
            input,
            w,
            h,
            origin,
            coding,
            component_mb,
            component_steps,
        );
    }
    let Coding {
        levels, mct, block, ..
    } = coding;
    let mut rgba = Vec::with_capacity(w * h * 4);
    if component_mb.len() == 1 && origin == (0, 0) {
        let mut plane = packets::<SPLIT>(input, w, h, levels, &component_mb[0], block)?;
        synthesize(&mut plane, w, h, levels)?;
        for v in plane {
            let v = gray8(v);
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    } else {
        let mut planes = vec![vec![0; w * h]; component_mb.len()];
        let mb: Vec<_> = component_mb.iter().map(Vec::as_slice).collect();
        if origin == (0, 0) {
            packets_planes::<SPLIT>(input, w, h, levels, &mb, &mut planes, block)?;
        } else {
            packets_planes_grid::<true, SPLIT, false>(
                input,
                w,
                h,
                levels,
                &mb,
                &mut planes,
                BlockGrid { block, origin },
            )?;
        }
        let factor = 1usize << levels;
        for plane in &mut planes {
            if origin.0 & (factor - 1) == 0 && origin.1 & (factor - 1) == 0 {
                synthesize(plane, w, h, levels)?;
            } else {
                synthesize_unaligned(plane, w, h, levels, origin)?;
            }
        }
        if planes.len() == 1 {
            for &v in &planes[0] {
                let v = gray8(v);
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        } else {
            for ((&v0, &v1), &v2) in planes[0].iter().zip(&planes[1]).zip(&planes[2]) {
                let rgb = if mct {
                    inverse_rct(v0, v1, v2)
                } else {
                    [gray8(v0), gray8(v1), gray8(v2)]
                };
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
    Ok(rgba)
}
fn tile_part<'a>(
    s: &'a [u8],
    p: usize,
    tile_index: usize,
    part_index: u8,
    declared: &mut u8,
) -> Result<&'a [u8], Error> {
    if u16be(s, p)? != 0xff90 || u16be(s, p + 2)? != 10 {
        return Err(bad("SOT required for every tile-part"));
    }
    let b = bytes(s, p + 4, 8)?;
    if usize::from(u16be(b, 0)?) != tile_index || b[6] != part_index {
        return Err(unsupported(
            "tile-part ID or sequence requires directory assembly",
        ));
    }
    // T.800 A.4.2 permits each header's TNsot to be zero or the correct total.
    // Nonzero declarations agree, including when a later part supplies it.
    let total = b[7];
    if total > 18 || part_index >= 18 {
        return Err(unsupported("at most 18 tile-parts supported"));
    }
    if total != 0 {
        if *declared != 0 && *declared != total {
            return Err(bad("inconsistent tile-part count"));
        }
        *declared = total;
    }
    if *declared != 0 && part_index >= *declared {
        return Err(bad("tile-part exceeds declared count"));
    }
    let stated_length = u32be(b, 2)? as usize;
    // T.800 A.4.2: Psot=0 extends only the final codestream part to EOC.
    // The container gives a bounded slice; use its verified suffix rather
    // than searching for marker-like bytes inside packet data.
    let length = if stated_length == 0 {
        let payload_end = s
            .strip_suffix(&[255, 217])
            .ok_or_else(|| bad("terminal tile-part requires EOC"))?
            .len();
        payload_end
            .checked_sub(p)
            .ok_or_else(|| bad("terminal tile-part extent"))?
    } else {
        stated_length
    };
    if length <= 14 {
        return Err(bad("nonempty tile-part required"));
    }
    let part = bytes(s, p, length)?;
    if part[12..14] != [255, 147] {
        return Err(unsupported(
            "tile-header overrides are unsupported; SOD required",
        ));
    }
    Ok(part)
}
// Psot, never marker scanning inside packet data, locates the next header.
// Other IDs end a contiguous tile group; the outer raster/EOC checks validate
// whether that ID or EOC is legal at this position.
fn same_tile(s: &[u8], p: usize, index: usize) -> Result<bool, Error> {
    match u16be(s, p)? {
        0xff90 => Ok(usize::from(u16be(s, p + 4)?) == index),
        0xffd9 => Ok(false),
        _ => Err(bad("SOT or EOC required after tile-part")),
    }
}
fn tiles(
    s: &[u8],
    mut p: usize,
    size: Size,
    coding: Coding,
    component_mb: &[Vec<u8>],
    component_steps: &[Vec<f64>],
) -> Result<Vec<u8>, Error> {
    let Size {
        w,
        h,
        tile_w,
        tile_h,
        image_origin: (x0, y0),
        tile_origin: (tx0, ty0),
        ..
    } = size;
    let (x1, y1) = (x0 + w, y0 + h);
    let nx = (x1 - tx0).div_ceil(tile_w);
    let ny = (y1 - ty0).div_ceil(tile_h);
    let count = nx * ny; // SIZ bounded to 256 before any coefficient allocation.
    // Retain the single-tile allocation/move path. Multi-tile scratch is dropped
    // after each row copy; only the whole-canvas RGBA output survives.
    let mut rgba = if count == 1 {
        Vec::new()
    } else {
        vec![0; w * h * 4]
    };
    for index in 0..count {
        // One LRCP layer, one precinct: at most six resolutions times three
        // components, with every supported part containing a whole packet.
        let mut declared = 0;
        let first = tile_part(s, p, index, 0, &mut declared)?;
        // Keep known single-part tiles on their borrowed fast path. Unknown
        // single-part groups only need one bounded next-header lookahead.
        let single = declared == 1 || (declared == 0 && !same_tile(s, p + first.len(), index)?);
        // T.800 B.3: intersect the nominal tile with the image interval.
        // Coefficients keep reference-grid coordinates; drawing uses the
        // cropped image's local origin and does not allocate global padding.
        let nominal_x = tx0 + (index % nx) * tile_w;
        let nominal_y = ty0 + (index / nx) * tile_h;
        let x = nominal_x.max(x0);
        let y = nominal_y.max(y0);
        let tw = (nominal_x + tile_w).min(x1) - x;
        let th = (nominal_y + tile_h).min(y1) - y;
        let pixels = if single {
            p += first.len();
            tile_pixels::<false>(
                PacketData {
                    bytes: &first[14..],
                    part_ends: &[],
                },
                tw,
                th,
                (x, y),
                coding,
                component_mb,
                component_steps,
            )?
        } else {
            // Gather at most 18 borrowed payloads first; validate all lengths
            // and charge the extra join allocation before reserving/copying.
            let mut bodies = Vec::with_capacity(18);
            let mut ends = Vec::with_capacity(18);
            let mut total = 0usize;
            let mut actual = 0u8;
            let mut part = first;
            loop {
                let body = &part[14..];
                total = total
                    .checked_add(body.len())
                    .ok_or_else(|| bad("joined tile data overflow"))?;
                bodies.push(body);
                ends.push(total);
                p += part.len();
                actual += 1;
                if declared != 0 && actual == declared {
                    break;
                }
                if !same_tile(s, p, index)? {
                    break;
                }
                // Bound actual parts even if every TNsot is zero. No 19th
                // body or coefficient scratch is allocated on this path.
                if actual == 18 {
                    return Err(unsupported("at most 18 tile-parts supported"));
                }
                part = tile_part(s, p, index, actual, &mut declared)?;
            }
            if declared != 0 && actual != declared {
                return Err(bad("incomplete declared tile-part count"));
            }
            let required = (w * h * size.components * 64 + 8192)
                .checked_add(total)
                .ok_or_else(|| bad("JPEG2000 workspace overflow"))?;
            crate::limits::check(
                required,
                crate::limits::current().expanded_bytes,
                "JPEG2000 joined tile workspace",
            )?;
            let mut joined = Vec::with_capacity(total);
            for body in bodies {
                joined.extend_from_slice(body);
            }
            tile_pixels::<true>(
                PacketData {
                    bytes: &joined,
                    part_ends: &ends,
                },
                tw,
                th,
                (x, y),
                coding,
                component_mb,
                component_steps,
            )?
        };
        let (out_x, out_y) = (x - x0, y - y0);
        if count == 1 {
            rgba = pixels;
        } else {
            for row in 0..th {
                rgba[((out_y + row) * w + out_x) * 4..((out_y + row) * w + out_x + tw) * 4]
                    .copy_from_slice(&pixels[row * tw * 4..(row + 1) * tw * 4]);
            }
        }
    }
    if bytes(s, p, 2)? != [255, 217] || p + 2 != s.len() {
        return Err(unsupported(
            "tile directory, EOC or trailing data requires general validation",
        ));
    }
    Ok(rgba)
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
    let mut component_quant = [None; 3];
    loop {
        let marker = u16be(s, p)?;
        let length = usize::from(u16be(s, p + 2)?);
        if length < 2 {
            return Err(bad("marker length"));
        }
        let b = bytes(s, p + 4, length - 2)?;
        match marker {
            0xff51 => {
                if size.is_some() || b.len() < 36 {
                    return Err(bad("invalid SIZ"));
                }
                let components = usize::from(u16be(b, 34)?);
                if !matches!(components, 1 | 3) {
                    return Err(unsupported("only Gray8 or RGB8 components"));
                }
                if b.len() != 36 + 3 * components {
                    return Err(bad("SIZ component length"));
                }
                let x1 = u32be(b, 2)? as usize;
                let y1 = u32be(b, 6)? as usize;
                let x0 = u32be(b, 10)? as usize;
                let y0 = u32be(b, 14)? as usize;
                let w = x1
                    .checked_sub(x0)
                    .ok_or_else(|| bad("image origin beyond extent"))?;
                let h = y1
                    .checked_sub(y0)
                    .ok_or_else(|| bad("image origin beyond extent"))?;
                if w == 0 || h == 0 {
                    return Err(bad("zero dimensions"));
                }
                let tile_w = u32be(b, 18)? as usize;
                let tile_h = u32be(b, 22)? as usize;
                let tx0 = u32be(b, 26)? as usize;
                let ty0 = u32be(b, 30)? as usize;
                if tile_w == 0 || tile_h == 0 {
                    return Err(bad("zero tile dimensions"));
                }
                if !matches!(u16be(b, 0)?, 0..=2)
                    || w > 1024
                    || h > 1024
                    || tile_w > 1024
                    || tile_h > 1024
                    || !b[36..].chunks_exact(3).all(|c| c == [7, 1, 1])
                {
                    return Err(unsupported(
                        "SIZ: unsigned Gray8/RGB8, no subsampling, canvas/tile dimensions <=1024 required",
                    ));
                }
                // Implicit precincts are 32768 on each resolution's grid.
                // Bounding reference extents keeps every tile/resolution in
                // that first precinct, without allocating the blank prefix.
                if x1 > 32768 || y1 > 32768 {
                    return Err(unsupported("reference extents must be <=32768"));
                }
                // These guards precede additions/counts: offsets and tile
                // lengths are now small, and the first tile must intersect.
                if tx0 > x0 || ty0 > y0 || tx0 + tile_w <= x0 || ty0 + tile_h <= y0 {
                    return Err(bad("tile grid must cover the image origin"));
                }
                if (x1 - tx0).div_ceil(tile_w) * (y1 - ty0).div_ceil(tile_h) > 256 {
                    return Err(unsupported("at most 256 tiles required"));
                }
                if dimensions.is_some_and(|d| d != (w, h, components)) {
                    return Err(bad("JP2 and SIZ dimensions or component count differ"));
                }
                let limits = crate::limits::current();
                crate::limits::check(w * h, limits.pixels, "pixels")?;
                crate::limits::check(
                    w * h * 64 * components + 8192,
                    limits.expanded_bytes,
                    "JPEG2000 workspace",
                )?;
                crate::limits::check(1, limits.frames, "frames")?;
                size = Some(Size {
                    w,
                    h,
                    components,
                    tile_w,
                    tile_h,
                    image_origin: (x0, y0),
                    tile_origin: (tx0, ty0),
                });
            }
            0xff52 => {
                if coding.is_some() || b.len() < 10 {
                    return Err(bad("invalid COD"));
                }
                if b[5] > 5 {
                    return Err(unsupported("more than five decompositions"));
                }
                let precincts = if b[0] & 1 != 0 {
                    if b.len() != 11 + usize::from(b[5]) {
                        return Err(bad("COD precinct count"));
                    }
                    let mut entries = [0; 6];
                    entries[..=usize::from(b[5])].copy_from_slice(&b[10..]);
                    Some(entries)
                } else {
                    if b.len() != 10 {
                        return Err(bad("invalid COD"));
                    }
                    None
                };
                if b[0] & !1 != 0
                    || b[1] != 0
                    || u16be(b, 2)? != 1
                    || b[4] > 1
                    || b[5] > 5
                    || b[6] > 8
                    || b[7] > 8
                    || u16::from(b[6]) + u16::from(b[7]) > 8
                    || b[8] != 0
                    || b[9] > 1
                {
                    return Err(unsupported(
                        "COD: one layer, LRCP, 0..5 levels, power-of-two blocks 4..1024 with area <=4096, style zero, 5/3 or 9/7 transform required",
                    ));
                }
                if b[4] == 1 && size.ok_or_else(|| bad("COD before SIZ"))?.components != 3 {
                    return Err(unsupported("MCT requires three components"));
                }
                // T.800 Table A.18: offsets add two to each exponent; their
                // sum must be <=8, bounding every full block to 4096 samples.
                let block = (1usize << (b[6] + 2), 1usize << (b[7] + 2));
                coding = Some(Coding {
                    levels: b[5],
                    mct: b[4] == 1,
                    irreversible: b[9] == 0,
                    block,
                    precincts,
                });
            }
            0xff5c => {
                if quant.is_some() || b.is_empty() {
                    return Err(bad("invalid QCD"));
                }
                quant = Some(b);
            }
            0xff5d => {
                let components = size.ok_or_else(|| bad("QCC before SIZ"))?.components;
                if components != 3 {
                    return Err(unsupported("QCC requires RGB8"));
                }
                if b.len() < 2 {
                    return Err(bad("invalid QCC"));
                }
                let component = usize::from(b[0]);
                if component >= components || component_quant[component].is_some() {
                    return Err(bad("invalid or duplicate QCC component"));
                }
                component_quant[component] = Some(&b[1..]);
            }
            0xff64 => {
                if b.len() < 2 {
                    return Err(bad("invalid COM"));
                }
                crate::limits::charge_metadata(b.len())?;
            }
            0xff90 => {
                let size = size.ok_or_else(|| bad("missing SIZ"))?;
                let coding = coding.ok_or_else(|| bad("missing COD"))?;
                let default_quant = quant.ok_or_else(|| bad("missing QCD"))?;
                // Validate mandatory QCD even when all components have QCC.
                let (default_mb, default_steps) = if coding.irreversible {
                    quant_irreversible(default_quant, coding.levels)?
                } else {
                    (quant_planes(default_quant, coding.levels)?, Vec::new())
                };
                let mut component_mb = vec![default_mb; size.components];
                let mut component_steps = if coding.irreversible {
                    vec![default_steps; size.components]
                } else {
                    Vec::new()
                };
                for (component, override_quant) in
                    component_quant[..size.components].iter().enumerate()
                {
                    if let Some(q) = override_quant {
                        if coding.irreversible {
                            let (mb, steps) = quant_irreversible(q, coding.levels)?;
                            component_mb[component] = mb;
                            component_steps[component] = steps;
                        } else {
                            component_mb[component] = quant_planes(q, coding.levels)?;
                        }
                    }
                }
                let rgba = tiles(s, p, size, coding, &component_mb, &component_steps)?;
                return Ok((size.w, size.h, rgba));
            }
            _ => return Err(unsupported("codestream marker")),
        }
        p += length + 2;
    }
}
/// Decode the supported signed/unsigned 1–8-bit profile into ordinary RGBA8.
pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    crate::decode_guard::run(
        reader,
        option,
        crate::limits::DecodeLimits::default(),
        decode_inner,
    )
}
/// Decode unsigned, full-resolution Gray/RGB/declared straight RGBA8 at a
/// codestream resolution level. `reduce` discards that many finest DWT levels
/// (scale 2^reduce), rather than resizing a fully decoded image.
///
/// Zero uses the existing full-resolution decoder, including its broader
/// accepted profile. Nonzero reduction requires the documented reduced profile:
/// classic 5/3 or bounded 9/7, no subsampling, signed/mixed precision, HT, encoded
/// ROI, palette/mapping, implicit/premultiplied alpha or multi-tile-part PPT.
/// PPM and single-tile-part PPT remain supported. Every packet header and
/// payload extent is checked, but discarded entropy is not Tier-1 decoded.
/// Original source pixels/input/metadata remain limited. Callbacks receive the
/// reduced RGBA8 dimensions only after structural and selected-data validation.
pub fn decode_with_reduction<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    reduce: u8,
    limits: crate::limits::DecodeLimits,
) -> Result<Option<ImgWarnings>, Error> {
    check_reduction(reduce)?;
    crate::decode_guard::run(reader, option, limits, |reader, option| {
        if reduce == 0 {
            return decode_inner(reader, option);
        }
        let data = read_reduced_input(reader)?;
        let (w, h, rgba) = general::image_reduced(&data, reduce)?;
        drop(data);
        let response = option.drawer.init(w, h, InitOptions::new())?;
        if !response.is_some_and(|r| r.response == ResponseCommand::Abort) {
            option.drawer.draw(0, 0, w, h, &rgba, None)?;
        }
        option.drawer.terminate(None)?;
        Ok(None)
    })
}

fn check_reduction(reduce: u8) -> Result<(), Error> {
    if reduce > 32 {
        return Err(Box::new(ImgError::new_const(
            ImgErrorKind::InvalidParameter,
            "JPEG2000 reduction must be at most 32 DWT levels".into(),
        )));
    }
    Ok(())
}

fn read_reduced_input<B: BinaryReader>(reader: &mut B) -> Result<Vec<u8>, Error> {
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let length = end.checked_sub(start).ok_or_else(|| bad("reader extent"))?;
    crate::limits::check_input_length(length, crate::limits::current().input_bytes)?;
    Ok(reader.read_bytes_as_vec(usize::try_from(length)?)?)
}

pub(crate) fn decode_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let length = end.checked_sub(start).ok_or_else(|| bad("reader extent"))?;
    crate::limits::check_input_length(length, crate::limits::current().input_bytes)?;
    let data = reader.read_bytes_as_vec(usize::try_from(length)?)?;
    let (w, h, rgba) = crate::limits::metadata_attempt(|| image(&data)).or_else(|error| {
        if error
            .downcast_ref::<ImgError>()
            .is_some_and(ImgError::is_unsupported)
        {
            general::image(&data)
        } else {
            Err(error)
        }
    })?;
    let response = option.drawer.init(w, h, InitOptions::new())?;
    if !response.is_some_and(|r| r.response == ResponseCommand::Abort) {
        option.drawer.draw(0, 0, w, h, &rgba, None)?;
    }
    option.drawer.terminate(None)?;
    Ok(None)
}
/// Static EX output retains unsigned Gray/RGB/declared straight RGBA12/16 in LE bytes.
#[cfg(feature = "image-buffer-ex")]
pub(crate) fn decode_ex_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let image = read_ex_image(reader)?;
    emit_ex_image(&image, option)
}

/// Decode the unsigned 12/16-bit EX profile at a native resolution level.
/// Zero delegates the existing full-resolution EX path. Nonzero reduction has
/// the same packet/entropy guarantee as [`decode_with_reduction`], retaining
/// Gray12/16 or RGB/declared straight RGBA12/16 little-endian source bytes.
/// Reduced bounds use both absolute reference-grid endpoints with ceil division;
/// an excessive reduction or empty global result fails before callbacks.
#[cfg(feature = "image-buffer-ex")]
pub fn decode_ex_with_reduction<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
    reduce: u8,
    limits: crate::limits::DecodeLimits,
) -> Result<Option<ImgWarnings>, Error> {
    check_reduction(reduce)?;
    crate::decode_guard::run_ex(reader, option, limits, |reader, option| {
        if reduce == 0 {
            return decode_ex_inner(reader, option);
        }
        let image = read_reduced_ex_image(reader, reduce)?;
        emit_ex_image(&image, option)
    })
}

#[cfg(feature = "image-buffer-ex")]
fn read_reduced_ex_image<B: BinaryReader>(
    reader: &mut B,
    reduce: u8,
) -> Result<general::DecodedImage, Error> {
    let input = read_reduced_input(reader)?;
    general::image_ex_reduced(&input, reduce)
}

#[cfg(feature = "image-buffer-ex")]
fn read_ex_image<B: BinaryReader>(reader: &mut B) -> Result<general::DecodedImage, Error> {
    let start = reader.offset()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(start))?;
    let length = end.checked_sub(start).ok_or_else(|| bad("reader extent"))?;
    crate::limits::check_input_length(length, crate::limits::current().input_bytes)?;
    let input = reader.read_bytes_as_vec(usize::try_from(length)?)?;
    general::image_ex(&input)
}

#[cfg(feature = "image-buffer-ex")]
fn ex_declaration(image: &general::DecodedImage) -> crate::draw::InitOptionsEx {
    use crate::draw::{InitOptionsEx, PixelFormatEx};
    InitOptionsEx {
        source_format: match (image.channels, image.depth) {
            (1, 12) => PixelFormatEx::Gray12,
            (1, _) => PixelFormatEx::Gray16,
            (_, 12) => PixelFormatEx::Rgba12,
            _ => PixelFormatEx::Rgba16,
        },
        source_stride: None,
        legacy: InitOptions::new(),
    }
}

#[cfg(feature = "image-buffer-ex")]
fn emit_ex_image(
    image: &general::DecodedImage,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    option
        .drawer
        .init_ex(image.width, image.height, Some(ex_declaration(image)))?;
    option
        .drawer
        .draw(0, 0, image.width, image.height, &image.bytes, None)?;
    option.drawer.terminate(None)?;
    Ok(None)
}

// The byte-slice convenience API can adopt a fully decoded native canvas.
// Keep the ordinary guard/scope and the existing callback conversion path;
// only the same-format result bypasses an otherwise redundant allocation/copy.
#[cfg(feature = "image-buffer-ex")]
pub(crate) fn image_from_ex_owned(
    buffer: &[u8],
    storage: crate::draw::PixelFormatEx,
    precision: crate::draw::PrecisionConversion,
    limits: crate::limits::DecodeLimits,
) -> Result<crate::draw::ImageBufferEx, Error> {
    image_from_ex_owned_inner::<false>(buffer, storage, precision, 0, limits)
}

/// Decode JPEG2000 into owned EX storage using native resolution reduction.
/// Exact/Round conversion, source declaration and byte strides are retained.
/// The same-format canvas is moved into the result; other storage formats use
/// guarded callbacks. `reduce = 0` uses the existing owned EX path unchanged.
/// See [`decode_ex_with_reduction`] for the reduced profile and guarantee.
#[cfg(feature = "image-buffer-ex")]
pub fn image_from_ex_with_reduction(
    buffer: &[u8],
    storage: crate::draw::PixelFormatEx,
    precision: crate::draw::PrecisionConversion,
    reduce: u8,
    limits: crate::limits::DecodeLimits,
) -> Result<crate::draw::ImageBufferEx, Error> {
    check_reduction(reduce)?;
    if reduce == 0 {
        return image_from_ex_owned(buffer, storage, precision, limits);
    }
    image_from_ex_owned_inner::<true>(buffer, storage, precision, reduce, limits)
}

#[cfg(feature = "image-buffer-ex")]
fn image_from_ex_owned_inner<const REDUCED: bool>(
    buffer: &[u8],
    storage: crate::draw::PixelFormatEx,
    precision: crate::draw::PrecisionConversion,
    reduce: u8,
    limits: crate::limits::DecodeLimits,
) -> Result<crate::draw::ImageBufferEx, Error> {
    use crate::draw::ImageBufferEx;
    let mut reader = bin_rs::reader::BytesReader::new(buffer);
    let mut converted = ImageBufferEx::with_storage(storage, precision);
    let mut native = None;
    crate::decode_guard::run_ex(
        &mut reader,
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut converted,
        },
        limits,
        |reader, option| {
            let image = if REDUCED {
                read_reduced_ex_image(reader, reduce)?
            } else {
                read_ex_image(reader)?
            };
            let declaration = ex_declaration(&image);
            if storage == declaration.source_format {
                crate::decode_guard::ex_source_bytes(
                    image.width,
                    image.height,
                    &declaration,
                    limits,
                )?;
                native = Some(ImageBufferEx::from_decoded_bytes(
                    image.width,
                    image.height,
                    storage,
                    precision,
                    image.bytes,
                )?);
            } else {
                emit_ex_image(&image, option)?;
            }
            Ok(None)
        },
    )?;
    Ok(native.unwrap_or(converted))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn odd_origin_singleton_reflection_and_overflow() {
        assert_eq!(inverse_line_at(&[], 1).unwrap(), Vec::<i32>::new());
        assert_eq!(inverse_line_at(&[-3], 0).unwrap(), vec![-3]);
        assert_eq!(inverse_line_at(&[-3], 1).unwrap(), vec![-1]);
        assert_eq!(inverse_line_at(&[6], 1).unwrap(), vec![3]);
        // Odd-start two-sample forward lifting of [3,4] produces low=4,
        // high=3-4=-1; reflection duplicates the sole neighbour at each end.
        assert_eq!(inverse_line_at(&[4, -1], 1).unwrap(), vec![3, 4]);
        assert!(inverse_line_at(&[i32::MAX, i32::MIN], 1).is_err());
        assert!(inverse_line_at(&[i32::MIN, i32::MAX], 1).is_err());
    }
    #[test]
    fn inverse_rct_negative_rounding_and_extremes() {
        assert_eq!(inverse_rct(-128, 0, 0), [0, 0, 0]);
        assert_eq!(inverse_rct(127, 0, 0), [255, 255, 255]);
        assert_eq!(inverse_rct(-1, -1, -2), [126, 128, 127]);
        // Independent forward transform round trips across unsigned extremes.
        for r in [0i32, 1, 127, 128, 254, 255] {
            for g in [0i32, 1, 127, 128, 254, 255] {
                for b in [0i32, 1, 127, 128, 254, 255] {
                    let y = (r + 2 * g + b).div_euclid(4) - 128;
                    assert_eq!(inverse_rct(y, b - g, r - g), [r as u8, g as u8, b as u8]);
                }
            }
        }
        assert_eq!(inverse_rct(i32::MAX, i32::MIN, i32::MAX), [255, 255, 128]);
        assert_eq!(inverse_rct(i32::MIN, i32::MAX, i32::MIN), [0, 0, 128]);
    }
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
                .map(|x| ((x * 73 + width * 11) % 257) - 128)
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
