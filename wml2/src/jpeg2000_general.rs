//! General Part-1 packet/tile assembly. The established small-profile decoder
//! remains the first path; this module owns the expanded component geometry.
use super::{Bits, Error, bad, box_at, bytes, resolution_box, tier1, u16be, u32be, unsupported};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone)]
struct Component {
    depth: u8,
    signed: bool,
    sx: usize,
    sy: usize,
}
#[derive(Clone)]
struct Style {
    levels: u8,
    bw: usize,
    bh: usize,
    flags: u8,
    filter: u8,
    precincts: Vec<u8>,
}
#[derive(Clone)]
struct Config {
    default: Style,
    default_quant: Vec<u8>,
    styles: Vec<Style>,
    quant: Vec<Vec<u8>>,
    roi: Vec<u8>,
    layers: u16,
    order: u8,
    mct: bool,
    scod: u8,
    poc: Vec<Poc>,
    ht: bool,
}
#[derive(Clone, Copy)]
struct Poc {
    r0: u8,
    c0: usize,
    l1: u16,
    r1: u8,
    c1: usize,
    order: u8,
}
struct Geometry {
    rsiz: u16,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    tx: usize,
    ty: usize,
    tw: usize,
    th: usize,
    components: Vec<Component>,
}
#[derive(Default)]
struct Container {
    space: Option<u32>,
    ihdr: Option<(usize, usize, usize, u8)>,
    depths: Vec<u8>,
    palette: Vec<Vec<u8>>,
    mapping: Vec<(usize, u8, usize)>,
    channels: Vec<(usize, u16, u16)>,
    channel_definitions: usize,
}
fn check(n: usize, name: &str) -> Result<(), Error> {
    crate::limits::check(n, crate::limits::current().expanded_bytes, name)
}
fn count(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or_else(|| bad("size overflow"))
}
fn array<T: Clone>(n: usize, v: T) -> Result<Vec<T>, Error> {
    check(count(n, std::mem::size_of::<T>())?, "JPEG2000 allocation")?;
    let mut out = Vec::new();
    out.try_reserve_exact(n)?;
    out.resize(n, v);
    Ok(out)
}
fn div(v: usize, d: u8) -> usize {
    ((v as u64).div_ceil(1u64 << d)) as usize
}
fn band_start(v: usize, d: u8, high: bool) -> usize {
    let f = 1u64 << d;
    ((v as u64 + if high { f / 2 } else { f } - 1) / f) as usize
}

fn container(data: &[u8]) -> Result<(&[u8], Container), Error> {
    if data.starts_with(&[255, 79]) {
        return Ok((data, Container::default()));
    }
    if !data.starts_with(super::SIGNATURE) {
        return Err(bad("missing signature"));
    }
    let mut meta = Container::default();
    let mut stream = None;
    let mut brand = false;
    let mut p = 12;
    while p < data.len() {
        let (k, b, next) = box_at(data, p)?;
        match k {
            b"ftyp" => {
                if brand || p != 12 || b.len() < 12 || (b.len() - 8) % 4 != 0 {
                    return Err(bad("file type box"));
                }
                if !b[8..].chunks_exact(4).any(|v| v == b"jp2 ") {
                    return Err(unsupported("JP2 compatibility brand required"));
                }
                brand = true;
            }
            b"jp2h" => {
                if !brand || meta.ihdr.is_some() || stream.is_some() {
                    return Err(bad("JP2 header order"));
                }
                let mut q = 0;
                let mut resolution = false;
                while q < b.len() {
                    if u32be(b, q)? == 0 {
                        return Err(bad("zero-length JP2 header child"));
                    }
                    let (kind, v, n) = box_at(b, q)?;
                    match kind {
                        b"ihdr" => {
                            if q != 0 || v.len() != 14 || meta.ihdr.is_some() {
                                return Err(bad("image header"));
                            }
                            if v[11] != 7 || v[12] > 1 || v[13] > 1 {
                                return Err(bad("image header fields"));
                            }
                            meta.ihdr = Some((
                                u32be(v, 4)? as usize,
                                u32be(v, 0)? as usize,
                                usize::from(u16be(v, 8)?),
                                v[10],
                            ));
                        }
                        b"bpcc" => {
                            if !meta.depths.is_empty() {
                                return Err(bad("duplicate bpcc"));
                            }
                            meta.depths = v.to_vec();
                        }
                        b"colr" => {
                            if meta.space.is_some() {
                                crate::limits::charge_metadata(v.len())?;
                            } else {
                                if v.len() < 3 {
                                    return Err(bad("colour box"));
                                }
                                match v[0] {
                                    1 => {
                                        if v.len() != 7 {
                                            return Err(bad("colour box length"));
                                        }
                                        let c = u32be(v, 3)?;
                                        if !matches!(c, 16..=18) {
                                            return Err(unsupported("enumerated colour space"));
                                        }
                                        meta.space = Some(c);
                                    }
                                    2 | 3 => {
                                        return Err(unsupported("ICC colour profile deferred"));
                                    }
                                    _ => crate::limits::charge_metadata(v.len())?,
                                }
                            }
                        }
                        b"pclr" => {
                            if !meta.palette.is_empty() || v.len() < 3 {
                                return Err(bad("palette"));
                            }
                            let entries = usize::from(u16be(v, 0)?);
                            let columns = usize::from(v[2]);
                            if entries == 0 || entries > 1024 || columns == 0 {
                                return Err(bad("palette dimensions"));
                            }
                            let depths = bytes(v, 3, columns)?;
                            if depths.iter().any(|x| x & 127 > 7) {
                                return Err(unsupported("high-depth palette deferred"));
                            }
                            if v.len() != 3 + columns + entries * columns {
                                return Err(bad("palette length"));
                            }
                            for row in v[3 + columns..].chunks_exact(columns) {
                                meta.palette.push(
                                    row.iter()
                                        .zip(depths)
                                        .map(|(&x, &d)| {
                                            let depth = (d & 127) + 1;
                                            let val = if d & 128 != 0 {
                                                (i32::from(x) << (32 - depth)) >> (32 - depth)
                                            } else {
                                                i32::from(x)
                                            };
                                            let val = val
                                                + if d & 128 != 0 { 1 << (depth - 1) } else { 0 };
                                            ((val.clamp(0, (1 << depth) - 1) * 255)
                                                / ((1 << depth) - 1))
                                                as u8
                                        })
                                        .collect(),
                                );
                            }
                        }
                        b"cmap" => {
                            if !meta.mapping.is_empty() || v.is_empty() || v.len() % 4 != 0 {
                                return Err(bad("component mapping"));
                            }
                            for x in v.chunks_exact(4) {
                                if x[2] > 1 || (x[2] == 0 && x[3] != 0) {
                                    return Err(bad("component mapping type"));
                                }
                                meta.mapping.push((
                                    usize::from(u16be(x, 0)?),
                                    x[2],
                                    usize::from(x[3]),
                                ));
                            }
                        }
                        b"cdef" => {
                            meta.channel_definitions += 1;
                            if !meta.channels.is_empty() || v.len() < 2 {
                                return Err(bad("channel definitions"));
                            }
                            if v.len() != 2 + usize::from(u16be(v, 0)?) * 6 {
                                return Err(bad("channel definitions length"));
                            }
                            for x in v[2..].chunks_exact(6) {
                                meta.channels.push((
                                    usize::from(u16be(x, 0)?),
                                    u16be(x, 2)?,
                                    u16be(x, 4)?,
                                ));
                            }
                        }
                        b"res " => {
                            if resolution {
                                return Err(bad("duplicate resolution box"));
                            }
                            resolution_box(v)?;
                            resolution = true;
                            crate::limits::charge_metadata(v.len())?;
                        }
                        b"jp2h" | b"jp2c" | b"ftyp" | b"jP  " => {
                            return Err(bad("misplaced structural box"));
                        }
                        _ => crate::limits::charge_metadata(v.len())?,
                    }
                    q = n;
                }
                if meta.ihdr.is_none() || meta.space.is_none() {
                    return Err(bad("incomplete JP2 header"));
                }
            }
            b"jp2c" => {
                if !brand || meta.ihdr.is_none() || stream.is_some() {
                    return Err(bad("codestream box order"));
                }
                stream = Some(b);
            }
            b"jP  " | b"ihdr" | b"colr" | b"bpcc" | b"pclr" | b"cmap" | b"cdef" => {
                return Err(bad("misplaced JP2 structural box"));
            }
            _ => crate::limits::charge_metadata(b.len())?,
        }
        p = next;
    }
    if !meta.palette.is_empty() && meta.mapping.is_empty()
        || meta.palette.is_empty() && meta.mapping.iter().any(|&(_, kind, _)| kind == 1)
    {
        return Err(bad("palette/mapping pair required"));
    }
    Ok((stream.ok_or_else(|| bad("missing codestream"))?, meta))
}
fn output_pixel_bytes<const EX: bool>(components: usize) -> usize {
    if EX {
        if components == 1 { 2 } else { 8 }
    } else {
        4
    }
}

// Channel definitions describe the reconstructed channels, after inverse RCT.
// Require a complete, unambiguous bijection; no implicit alpha association.
fn ex_channel_order(nc: usize, meta: &Container) -> Result<[usize; 4], Error> {
    if meta.channel_definitions > 1 {
        return Err(bad("EX duplicate channel definitions"));
    }
    if !meta.palette.is_empty() || !meta.mapping.is_empty() {
        return Err(unsupported("EX palette/component mapping deferred"));
    }
    if nc == 1 {
        if meta.space.is_some_and(|s| s != 17) || meta.channel_definitions != 0 {
            return Err(unsupported(
                "EX Gray requires Gray without channel definitions",
            ));
        }
        return Ok([0, 1, 2, 3]);
    }
    if meta.space.is_some_and(|s| s != 16) {
        return Err(unsupported("EX RGB/RGBA requires RGB colour declaration"));
    }
    if nc == 3 && meta.channel_definitions == 0 {
        return Ok([0, 1, 2, 3]);
    }
    if meta.channels.len() != nc || nc == 4 && meta.space != Some(16) {
        return Err(unsupported(
            "EX RGBA requires complete JP2 straight-alpha CDEF",
        ));
    }
    let mut order = [usize::MAX; 4];
    let mut seen = [false; 4];
    for &(index, kind, association) in &meta.channels {
        if index >= nc || seen[index] {
            return Err(bad("EX channel definition index"));
        }
        seen[index] = true;
        let slot = match (kind, association) {
            (0, 1..=3) if index < 3 => usize::from(association - 1),
            (1, 0) if nc == 4 && index == 3 => 3,
            (2, _) => return Err(unsupported("EX premultiplied alpha deferred")),
            _ => {
                return Err(unsupported(
                    "EX channel definition/alpha association deferred",
                ));
            }
        };
        if order[slot] != usize::MAX {
            return Err(bad("EX duplicate channel association"));
        }
        order[slot] = index;
    }
    if order[..nc].contains(&usize::MAX) {
        return Err(bad("EX incomplete channel definitions"));
    }
    Ok(order)
}

fn inverse_rct_wide(y: i32, cb: i32, cr: i32) -> [i64; 3] {
    let (y, cb, cr) = (i64::from(y), i64::from(cb), i64::from(cr));
    let green = y - (cb + cr).div_euclid(4);
    [green + cr, green, green + cb]
}

fn geometry<const EX: bool>(b: &[u8]) -> Result<Geometry, Error> {
    if b.len() < 36 {
        return Err(bad("SIZ length"));
    }
    if !matches!(u16be(b, 0)?, 0..=2 | 0x4000) {
        return Err(unsupported("SIZ capabilities"));
    }
    let nc = usize::from(u16be(b, 34)?);
    if nc == 0 || nc > 16384 || b.len() != 36 + 3 * nc {
        return Err(bad("SIZ components"));
    }
    let x1 = u32be(b, 2)? as usize;
    let y1 = u32be(b, 6)? as usize;
    let x0 = u32be(b, 10)? as usize;
    let y0 = u32be(b, 14)? as usize;
    let tw = u32be(b, 18)? as usize;
    let th = u32be(b, 22)? as usize;
    let tx = u32be(b, 26)? as usize;
    let ty = u32be(b, 30)? as usize;
    if x1 <= x0
        || y1 <= y0
        || tw == 0
        || th == 0
        || tx > x0
        || ty > y0
        || tw <= x0 - tx
        || th <= y0 - ty
    {
        return Err(bad("SIZ geometry"));
    }
    let pixels = count(x1 - x0, y1 - y0)?;
    crate::limits::check(pixels, crate::limits::current().pixels, "pixels")?;
    check(
        count(pixels, output_pixel_bytes::<EX>(nc))?,
        "JPEG2000 output",
    )?;
    crate::limits::check(1, crate::limits::current().frames, "frames")?;
    let nt = count((x1 - tx).div_ceil(tw), (y1 - ty).div_ceil(th))?;
    if nt > 65535 {
        return Err(bad("tile count"));
    }
    let mut components = Vec::with_capacity(nc);
    for c in b[36..].chunks_exact(3) {
        let depth = (c[0] & 127) + 1;
        if EX {
            if !matches!(nc, 1 | 3 | 4)
                || !matches!(depth, 12 | 16)
                || c[0] != b[36]
                || c[0] & 128 != 0
                || c[1..] != [1, 1]
            {
                return Err(unsupported(
                    "EX requires uniform unsigned 12/16-bit full-resolution Gray, RGB or declared RGBA",
                ));
            }
        } else if depth > 8 {
            return Err(unsupported("high sample depth deferred"));
        }
        if c[1] == 0 || c[2] == 0 {
            return Err(bad("zero subsampling"));
        }
        components.push(Component {
            depth,
            signed: c[0] & 128 != 0,
            sx: c[1] as usize,
            sy: c[2] as usize,
        });
    }
    Ok(Geometry {
        rsiz: u16be(b, 0)?,
        x0,
        y0,
        x1,
        y1,
        tx,
        ty,
        tw,
        th,
        components,
    })
}
fn style(b: &[u8], explicit: bool) -> Result<Style, Error> {
    if b.len() < 5
        || b[0] > 32
        || b[1] > 8
        || b[2] > 8
        || u16::from(b[1]) + u16::from(b[2]) > 8
        || b[4] > 1
    {
        return Err(bad("coding style"));
    }
    if b[3] & 0xc0 == 0xc0 {
        return Err(unsupported("HTJ2K MIXED code-block dispatch"));
    }
    let n = b[0] as usize + 1;
    if b.len() != 5 + if explicit { n } else { 0 } {
        return Err(bad("precinct count"));
    }
    let precincts = if explicit {
        b[5..].to_vec()
    } else {
        vec![255; n]
    };
    if precincts.iter().skip(1).any(|v| v & 15 == 0 || v >> 4 == 0) {
        return Err(bad("zero high-resolution precinct exponent"));
    }
    Ok(Style {
        levels: b[0],
        bw: 1usize << (b[1] + 2),
        bh: 1usize << (b[2] + 2),
        flags: b[3],
        filter: b[4],
        precincts,
    })
}
fn cod(b: &[u8], cfg: &mut Config) -> Result<(), Error> {
    if b.len() < 10 || b[0] & !7 != 0 || b[1] > 4 || u16be(b, 2)? == 0 || b[4] > 1 {
        return Err(bad("COD fields"));
    }
    let s = style(&b[5..], b[0] & 1 != 0)?;
    cfg.default = s.clone();
    cfg.styles.fill(s);
    cfg.layers = u16be(b, 2)?;
    cfg.order = b[1];
    cfg.mct = b[4] != 0;
    cfg.scod = b[0];
    Ok(())
}
fn component_index<'a>(b: &'a [u8], nc: usize) -> Result<(usize, &'a [u8]), Error> {
    let n = if nc < 257 { 1 } else { 2 };
    let index = if n == 1 {
        *b.first().ok_or_else(|| bad("component index"))? as usize
    } else {
        u16be(b, 0)? as usize
    };
    if index >= nc {
        return Err(bad("component index"));
    }
    Ok((index, bytes(b, n, b.len() - n)?))
}
// T.800 A.3/A.6: default/component precedence is independent of the order
// of marker segments in either the main header or the first tile-part header.
fn marker_priority(m: u16) -> u8 {
    match m {
        0xff52 => 0,
        0xff53 => 1,
        0xff5c => 2,
        0xff5d => 3,
        _ => 4,
    }
}
fn marker(m: u16, b: &[u8], cfg: &mut Config) -> Result<(), Error> {
    match m {
        0xff52 => cod(b, cfg)?,
        0xff53 => {
            let (c, b) = component_index(b, cfg.styles.len())?;
            let sc = *b.first().ok_or_else(|| bad("COC length"))?;
            if sc & !1 != 0 {
                return Err(bad("COC flags"));
            }
            cfg.styles[c] = style(&b[1..], sc & 1 != 0)?;
        }
        0xff5c => {
            // At most 1 + 3*32 bands, with two bytes per step. Bound the
            // default before cloning it for every component.
            if b.is_empty() || b.len() > 195 || b[0] & 31 > 2 {
                return Err(bad("quantization style"));
            }
            cfg.default_quant = b.to_vec();
            cfg.quant.fill(b.to_vec());
        }
        0xff5d => {
            let (c, b) = component_index(b, cfg.styles.len())?;
            cfg.quant[c] = b.to_vec();
        }
        0xff5e => {
            let (c, b) = component_index(b, cfg.styles.len())?;
            if b.len() != 2 || b[0] != 0 || b[1] > 30 {
                return Err(bad("ROI shift"));
            }
            cfg.roi[c] = b[1];
        }
        0xff5f => {
            let n = if cfg.styles.len() < 257 { 7 } else { 9 };
            if b.is_empty() || b.len() % n != 0 {
                return Err(bad("POC length"));
            }
            for v in b.chunks_exact(n) {
                let (c0, l1, r1, c1, order) = if n == 7 {
                    (
                        v[1] as usize,
                        u16be(v, 2)?,
                        v[4],
                        if v[5] == 0 { 256 } else { v[5] as usize },
                        v[6],
                    )
                } else {
                    (
                        u16be(v, 1)? as usize,
                        u16be(v, 3)?,
                        v[5],
                        u16be(v, 6)? as usize,
                        v[8],
                    )
                };
                if c0 >= c1
                    || c0 >= cfg.styles.len()
                    || v[0] >= r1
                    || r1 > 33
                    || l1 == 0
                    || order > 4
                {
                    return Err(bad("POC ranges"));
                }
                cfg.poc.push(Poc {
                    r0: v[0],
                    c0,
                    l1: l1.min(cfg.layers),
                    r1,
                    c1: c1.min(cfg.styles.len()),
                    order,
                });
            }
        }
        0xff64 => {
            if b.len() < 2 {
                return Err(bad("COM length"));
            }
            crate::limits::charge_metadata(b.len())?;
        }
        0xff63 => {
            if b.len() != 4 * cfg.styles.len() {
                return Err(bad("CRG length"));
            }
            crate::limits::charge_metadata(b.len())?;
        }
        0xff50 => {
            let capabilities = u32be(b, 0)?;
            if capabilities != 0x0002_0000 || b.len() != 6 {
                return Err(bad("CAP length"));
            }
            cfg.ht = true;
        }
        _ => return Err(unsupported("codestream marker")),
    }
    Ok(())
}
fn segment(s: &[u8], p: usize) -> Result<(u16, &[u8], usize), Error> {
    let m = u16be(s, p)?;
    if (0xff30..=0xff3f).contains(&m) {
        return Ok((m, &[], p + 2));
    }
    let n = usize::from(u16be(s, p + 2)?);
    if n < 2 {
        return Err(bad("marker length"));
    }
    Ok((m, bytes(s, p + 4, n - 2)?, p + 2 + n))
}
struct Part<'a> {
    body: &'a [u8],
    headers: Vec<u8>,
    poc: Vec<Poc>,
    lengths: Vec<usize>,
}
struct Tile<'a> {
    config: Config,
    parts: Vec<Part<'a>>,
    total: u8,
}
fn packed(map: &BTreeMap<u8, Vec<u8>>) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    for (i, (&z, b)) in map.iter().enumerate() {
        if i != z as usize {
            return Err(bad("packed header sequence"));
        }
        out.extend_from_slice(b);
    }
    Ok(out)
}
fn insert_packed(map: &mut BTreeMap<u8, Vec<u8>>, b: &[u8]) -> Result<(), Error> {
    let (&z, payload) = b.split_first().ok_or_else(|| bad("packed header length"))?;
    crate::limits::charge_metadata(payload.len())?;
    if map.insert(z, payload.to_vec()).is_some() {
        return Err(bad("duplicate packed header index"));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Node {
    low: u32,
    value: u32,
}
struct Tree {
    levels: Vec<(usize, Vec<Node>)>,
}
impl Tree {
    fn new(mut w: usize, mut h: usize) -> Result<Self, Error> {
        let mut levels = Vec::new();
        loop {
            levels.push((
                w,
                array(
                    count(w, h)?,
                    Node {
                        low: 0,
                        value: u32::MAX,
                    },
                )?,
            ));
            if w == 1 && h == 1 {
                break;
            }
            w = w.div_ceil(2);
            h = h.div_ceil(2);
        }
        Ok(Self { levels })
    }
    fn below(&mut self, x: usize, y: usize, t: u32, bits: &mut Bits<'_>) -> Result<bool, Error> {
        let mut low = 0;
        for d in (0..self.levels.len()).rev() {
            let (w, n) = &mut self.levels[d];
            let a = &mut n[(y >> d) * *w + (x >> d)];
            a.low = a.low.max(low);
            while a.low < t && a.low < a.value {
                if bits.bit()? {
                    a.value = a.low;
                } else {
                    a.low += 1;
                }
            }
            low = a.low.min(a.value);
        }
        Ok(self.levels[0].1[y * self.levels[0].0 + x].value < t)
    }
}
struct Block {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    orientation: u8,
    mb: u8,
    planes: u8,
    lblock: u32,
    passes: u32,
    segments: Vec<tier1::Segment>,
}
struct Group {
    blocks: Vec<Block>,
    nx: usize,
    inclusion: Tree,
    zero: Tree,
}
struct Precinct {
    groups: Vec<Group>,
    component: usize,
    resolution: u8,
    index: usize,
    x: u64,
    y: u64,
}
struct Plane {
    w: usize,
    h: usize,
    x0: usize,
    y0: usize,
    coefficients: Vec<i32>,
    samples: Vec<f64>,
}
#[derive(Clone, Copy)]
struct Packet {
    precinct: usize,
    layer: u16,
}

fn quant(q: &[u8], s: &Style, depth: u8) -> Result<(Vec<u8>, Vec<f64>), Error> {
    if q.is_empty() {
        return Err(bad("missing quantization"));
    }
    let bands = 1 + 3 * s.levels as usize;
    let mode = q[0] & 31;
    let guard = q[0] >> 5;
    if mode > 2
        || q.len()
            < 1 + if mode == 0 {
                bands
            } else if mode == 1 {
                2
            } else {
                2 * bands
            }
    {
        return Err(bad("quantization shape"));
    }
    if s.filter == 1 && mode != 0 {
        return Err(bad("reversible scalar quantization"));
    }
    let mut mb = Vec::new();
    let mut steps = Vec::new();
    for band in 0..bands {
        let (word, exp) = if mode == 0 {
            (0, u16::from(q[1 + band] >> 3))
        } else {
            let word = u16be(q, 1 + 2 * if mode == 1 { 0 } else { band })?;
            let e = (word >> 11)
                .checked_sub(if mode == 1 && band > 0 {
                    ((band - 1) / 3) as u16
                } else {
                    0
                })
                .ok_or_else(|| bad("quantization exponent"))?;
            (word, e)
        };
        if mode == 0 && q[1 + band] & 7 != 0 {
            return Err(bad("quantization reserved bits"));
        }
        let planes = u16::from(guard) + exp;
        // The existing 8-bit path retains its proven 16-bit coefficient cap.
        // Gray12/16 needs sample depth + subband gain + guard precision.
        // The segmented Tier-1 decoder is independently bounded to 28 planes.
        if planes == 0 || planes > if s.filter == 1 && depth <= 8 { 17 } else { 25 } {
            return Err(bad("coefficient precision"));
        }
        mb.push((planes - 1) as u8);
        let gain = if band == 0 {
            0
        } else if band % 3 == 0 {
            2
        } else {
            1
        };
        steps.push(if mode == 0 {
            1.0
        } else {
            (1.0 + f64::from(word & 2047) / 2048.0)
                * 2f64.powi(i32::from(depth) + gain - i32::from(exp))
        });
    }
    Ok((mb, steps))
}

struct TileWorkspace {
    planes: Vec<Plane>,
    precincts: Vec<Precinct>,
    steps: Vec<Vec<f64>>,
    retained_bytes: usize,
}
fn precincts<const EX: bool>(
    g: &Geometry,
    cfg: &Config,
    bounds: (usize, usize, usize, usize),
) -> Result<TileWorkspace, Error> {
    let (tx0, ty0, tx1, ty1) = bounds;
    let mut planes = Vec::new();
    let mut all = Vec::new();
    let mut quant_steps = Vec::new();
    let mut allocation = 0usize;
    let mut output_storage = count(
        count(g.x1 - g.x0, g.y1 - g.y0)?,
        output_pixel_bytes::<EX>(g.components.len()),
    )?;
    for c in g.components.iter().filter(|_| !EX) {
        output_storage = output_storage
            .checked_add(count(
                g.x1.div_ceil(c.sx) - g.x0.div_ceil(c.sx),
                g.y1.div_ceil(c.sy) - g.y0.div_ceil(c.sy),
            )?)
            .ok_or_else(|| bad("component storage overflow"))?;
    }
    for (c, (component, s)) in g.components.iter().zip(&cfg.styles).enumerate() {
        let x0 = tx0.div_ceil(component.sx);
        let y0 = ty0.div_ceil(component.sy);
        let x1 = tx1.div_ceil(component.sx);
        let y1 = ty1.div_ceil(component.sy);
        let w = x1 - x0;
        let h = y1 - y0;
        allocation = allocation
            .checked_add(count(
                count(w, h)?,
                if EX && s.filter == 0 { 28 } else { 20 },
            )?)
            .ok_or_else(|| bad("workspace overflow"))?;
        check(
            allocation
                .checked_add(output_storage)
                .ok_or_else(|| bad("workspace overflow"))?,
            "JPEG2000 workspace",
        )?;
        planes.push(Plane {
            w,
            h,
            x0,
            y0,
            coefficients: array(count(w, h)?, 0)?,
            samples: Vec::new(),
        });
        let (mb, steps) = quant(&cfg.quant[c], s, component.depth)?;
        quant_steps.push(steps);
        for r in 0..=s.levels {
            let d = s.levels - r;
            let (rx0, ry0, rx1, ry1) = (div(x0, d), div(y0, d), div(x1, d), div(y1, d));
            if rx0 == rx1 || ry0 == ry1 {
                continue;
            }
            let e = s.precincts[r as usize];
            let (px, py) = (e & 15, e >> 4);
            let (pw, ph) = (1usize << px, 1usize << py);
            let mut index = 0;
            for iy in ry0 / ph..ry1.div_ceil(ph) {
                for ix in rx0 / pw..rx1.div_ceil(pw) {
                    let mut groups = Vec::new();
                    for orientation in if r == 0 { 0..1 } else { 1..4 } {
                        let depth = if r == 0 { s.levels } else { s.levels - r + 1 };
                        let bx0 = band_start(x0, depth, orientation & 1 != 0);
                        let by0 = band_start(y0, depth, orientation & 2 != 0);
                        let bx1 = band_start(x1, depth, orientation & 1 != 0);
                        let by1 = band_start(y1, depth, orientation & 2 != 0);
                        let shift = usize::from(r != 0);
                        let qx0 = (ix * pw) >> shift;
                        let qy0 = (iy * ph) >> shift;
                        let qx1 = ((((ix as u64 + 1) * pw as u64) >> shift).min(usize::MAX as u64))
                            as usize;
                        let qy1 = ((((iy as u64 + 1) * ph as u64) >> shift).min(usize::MAX as u64))
                            as usize;
                        let ax = bx0.max(qx0);
                        let ay = by0.max(qy0);
                        let ex = bx1.min(qx1);
                        let ey = by1.min(qy1);
                        if ax >= ex || ay >= ey {
                            continue;
                        }
                        let cw = s.bw.min(pw >> shift);
                        let ch = s.bh.min(ph >> shift);
                        let nx = ex.div_ceil(cw) - ax / cw;
                        let ny = ey.div_ceil(ch) - ay / ch;
                        let band = if r == 0 {
                            0
                        } else {
                            r as usize * 3 - 3 + orientation as usize
                        };
                        let packed_x = if orientation & 1 != 0 {
                            div(x1, depth) - div(x0, depth)
                        } else {
                            0
                        };
                        let packed_y = if orientation & 2 != 0 {
                            div(y1, depth) - div(y0, depth)
                        } else {
                            0
                        };
                        let block_count = count(nx, ny)?;
                        // At most 82 classic passes (28 planes). EX includes
                        // worst-case segment/contribution Vec capacities as
                        // well as the established block/tag-tree allowance.
                        let block_bytes = if EX {
                            256 + 128 * std::mem::size_of::<tier1::Segment>()
                                + 256 * std::mem::size_of::<(usize, usize, usize, usize)>()
                        } else {
                            256
                        };
                        allocation = allocation
                            .checked_add(count(block_count, block_bytes)?)
                            .ok_or_else(|| bad("block allocation overflow"))?;
                        check(
                            allocation
                                .checked_add(output_storage)
                                .ok_or_else(|| bad("workspace overflow"))?,
                            "JPEG2000 block state",
                        )?;
                        let mut blocks = Vec::new();
                        blocks.try_reserve_exact(block_count)?;
                        for by in ay / ch..ey.div_ceil(ch) {
                            for bx in ax / cw..ex.div_ceil(cw) {
                                let x = ax.max(bx * cw);
                                let y = ay.max(by * ch);
                                let precision = mb[band]
                                    .checked_add(cfg.roi[c])
                                    .filter(|&v| v <= 28)
                                    .ok_or_else(|| bad("ROI coefficient precision"))?;
                                blocks.push(Block {
                                    x: packed_x + x - bx0,
                                    y: packed_y + y - by0,
                                    w: ex.min((bx + 1).saturating_mul(cw)) - x,
                                    h: ey.min((by + 1).saturating_mul(ch)) - y,
                                    orientation,
                                    mb: precision,
                                    planes: 0,
                                    lblock: 3,
                                    passes: 0,
                                    segments: Vec::new(),
                                });
                            }
                        }
                        groups.push(Group {
                            blocks,
                            nx,
                            inclusion: Tree::new(nx, ny)?,
                            zero: Tree::new(nx, ny)?,
                        });
                    }
                    all.push(Precinct {
                        groups,
                        component: c,
                        resolution: r,
                        index,
                        x: ((ix * pw) as u64 * (1u64 << d) * component.sx as u64).max(tx0 as u64),
                        y: ((iy * ph) as u64 * (1u64 << d) * component.sy as u64).max(ty0 as u64),
                    });
                    index += 1;
                }
            }
        }
    }
    Ok(TileWorkspace {
        planes,
        precincts: all,
        steps: quant_steps,
        retained_bytes: allocation
            .checked_add(output_storage)
            .ok_or_else(|| bad("workspace overflow"))?,
    })
}
fn packet_order(precincts: &[Precinct], cfg: &Config, poc: &[Poc]) -> Result<Vec<Packet>, Error> {
    check(
        count(count(precincts.len(), cfg.layers as usize)?, 96)?,
        "JPEG2000 packet directory",
    )?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let default = [Poc {
        r0: 0,
        c0: 0,
        l1: cfg.layers,
        r1: 33,
        c1: cfg.styles.len(),
        order: cfg.order,
    }];
    let changes = if poc.is_empty() { &default[..] } else { poc };
    let work = changes.iter().try_fold(0usize, |n, p| {
        n.checked_add(p.l1 as usize)
            .ok_or_else(|| bad("progression work overflow"))
    })?;
    check(
        count(count(precincts.len(), work)?, 16)?,
        "JPEG2000 progression work",
    )?;
    for change in changes {
        let mut part = Vec::new();
        for (i, p) in precincts.iter().enumerate() {
            if p.component >= change.c0
                && p.component < change.c1
                && p.resolution >= change.r0
                && p.resolution < change.r1
            {
                for layer in 0..change.l1 {
                    if seen.insert((i, layer)) {
                        part.push(Packet { precinct: i, layer });
                    }
                }
            }
        }
        check(count(out.len() + part.len(), 64)?, "JPEG2000 packet state")?;
        part.sort_by_key(|k| {
            let p = &precincts[k.precinct];
            let l = k.layer as u64;
            let r = p.resolution as u64;
            let c = p.component as u64;
            let i = p.index as u64;
            match change.order {
                0 => [l, r, c, i, 0],
                1 => [r, l, c, i, 0],
                2 => [r, p.y, p.x, c, l],
                3 => [p.y, p.x, c, r, l],
                _ => [c, p.y, p.x, r, l],
            }
        });
        out.extend(part);
    }
    Ok(out)
}
fn packet(
    part: &Part<'_>,
    offset: &mut usize,
    header_offset: &mut usize,
    p: &mut Precinct,
    layer: u16,
    cfg: &Config,
) -> Result<(), Error> {
    let mut at = *offset;
    if cfg.scod & 2 != 0 && part.body.get(at..at + 2) == Some(&[255, 145]) {
        bytes(part.body, at, 6)?;
        if u16be(part.body, at + 2)? != 4 {
            return Err(bad("SOP length"));
        }
        at += 6;
    }
    let packed = !part.headers.is_empty();
    if at > part.body.len() || *header_offset > part.headers.len() {
        return Err(bad("packet extent"));
    }
    let header = if packed {
        &part.headers[*header_offset..]
    } else {
        &part.body[at..]
    };
    let mut bits = Bits::new(header);
    let mut contributions = Vec::new();
    if bits.bit()? {
        for (group_index, group) in p.groups.iter_mut().enumerate() {
            for (i, block) in group.blocks.iter_mut().enumerate() {
                let included = if block.passes == 0 {
                    group.inclusion.below(
                        i % group.nx,
                        i / group.nx,
                        u32::from(layer) + 1,
                        &mut bits,
                    )?
                } else {
                    bits.bit()?
                };
                if !included {
                    continue;
                }
                if block.passes == 0 {
                    let mut threshold = 1;
                    while !group
                        .zero
                        .below(i % group.nx, i / group.nx, threshold, &mut bits)?
                    {
                        threshold += 1;
                        if threshold > u32::from(block.mb) + 1 {
                            return Err(bad("zero bit planes"));
                        }
                    }
                    block.planes = block.mb - (threshold - 1) as u8;
                }
                let mut passes = bits.passes()?;
                if block.planes == 0
                    || block.passes + passes
                        > u32::from(block.planes) * 3
                            - if cfg.styles[p.component].flags & 64 != 0 {
                                0
                            } else {
                                2
                            }
                {
                    return Err(bad("code-block passes"));
                }
                while bits.bit()? {
                    block.lblock += 1;
                    if block.lblock > 32 {
                        return Err(bad("code-block length"));
                    }
                }
                while passes > 0 {
                    let current = block.segments.last().map(|s| s.passes).unwrap_or(0);
                    let start = block.passes - current;
                    let cap = tier1::segment_capacity(cfg.styles[p.component].flags, start);
                    let new = block.segments.is_empty() || current == cap;
                    if new {
                        block.segments.push(tier1::Segment {
                            data: Vec::new(),
                            passes: 0,
                        });
                    }
                    let last = block.segments.len() - 1;
                    let start = block.passes - block.segments[last].passes;
                    let remain = tier1::segment_capacity(cfg.styles[p.component].flags, start)
                        - block.segments[last].passes;
                    let take = passes.min(remain);
                    let length = bits.number(block.lblock + take.ilog2())? as usize;
                    contributions.push((group_index, i, last, length));
                    block.segments[last].passes += take;
                    block.passes += take;
                    passes -= take;
                }
            }
        }
    }
    bits.align()?;
    if packed {
        *header_offset += bits.pos;
    } else {
        at += bits.pos;
    }
    if cfg.scod & 4 != 0 {
        let h = if packed { &part.headers[..] } else { part.body };
        let pos = if packed { *header_offset } else { at };
        if bytes(h, pos, 2)? != [255, 146] {
            return Err(bad("EPH required"));
        }
        if packed {
            *header_offset += 2;
        } else {
            at += 2;
        }
    }
    for (g, b, s, len) in contributions {
        let segment = &mut p.groups[g].blocks[b].segments[s];
        segment.data.extend_from_slice(bytes(part.body, at, len)?);
        at += len;
    }
    *offset = at;
    Ok(())
}

fn reconstruct<const EX: bool>(
    planes: &mut [Plane],
    precincts: Vec<Precinct>,
    cfg: &Config,
    steps: &[Vec<f64>],
) -> Result<(), Error> {
    for p in precincts {
        let c = p.component;
        let s = &cfg.styles[c];
        for group in p.groups {
            for block in group.blocks {
                if block.passes == 0 {
                    continue;
                }
                let values = if s.flags & 64 != 0 {
                    super::ht::decode(&block.segments, block.w, block.h, block.planes, s.flags)?
                } else {
                    tier1::decode_segments(
                        &block.segments,
                        block.w,
                        block.h,
                        block.orientation,
                        block.planes,
                        s.flags,
                    )?
                };
                let plane = &mut planes[c];
                for y in 0..block.h {
                    for x in 0..block.w {
                        let v = values[y * block.w + x];
                        let magnitude = v.unsigned_abs();
                        let shift = cfg.roi[c];
                        let v = if shift != 0 && u64::from(magnitude) >= (1u64 << (shift + 1)) {
                            let value = (magnitude >> shift) as i32;
                            if v < 0 { -value } else { value }
                        } else {
                            v
                        };
                        plane.coefficients[(block.y + y) * plane.w + block.x + x] = v;
                    }
                }
            }
        }
    }
    for (c, plane) in planes.iter_mut().enumerate() {
        let s = &cfg.styles[c];
        if s.filter == 1 {
            for v in &mut plane.coefficients {
                *v /= 2;
            }
            super::synthesize_unaligned(
                &mut plane.coefficients,
                plane.w,
                plane.h,
                s.levels,
                (plane.x0, plane.y0),
            )?;
            if EX {
                // Keep signed centered integer coefficients until LE packing;
                // do not narrow to u8 or introduce a floating reconstruction.
                continue;
            }
            plane.samples = plane.coefficients.iter().map(|&v| f64::from(v)).collect();
        } else {
            plane.samples = array(plane.coefficients.len(), 0.0)?;
            let bands = super::subbands::<true>(plane.w, plane.h, s.levels, (plane.x0, plane.y0));
            for (&(bx, by, bw, bh, _), &step) in bands.iter().zip(&steps[c]) {
                for y in by..by + bh {
                    for x in bx..bx + bw {
                        plane.samples[y * plane.w + x] =
                            f64::from(plane.coefficients[y * plane.w + x]) * (step * 0.5);
                    }
                }
            }
            if EX {
                // Dequantization is the final EX reader; release before DWT.
                plane.coefficients = Vec::new();
            }
            super::irreversible::synthesize(
                &mut plane.samples,
                plane.w,
                plane.h,
                s.levels,
                (plane.x0, plane.y0),
            )?;
        }
        // Keep the established legacy allocation/drop order in its hot path.
        plane.coefficients.clear();
    }
    if EX {
        return Ok(());
    }
    if cfg.mct {
        if planes.len() < 3
            || planes[..3].iter().any(|p| {
                p.w != planes[0].w
                    || p.h != planes[0].h
                    || p.x0 != planes[0].x0
                    || p.y0 != planes[0].y0
            })
        {
            return Err(bad("MCT component dimensions"));
        }
        for i in 0..planes[0].samples.len() {
            let (y, cb, cr) = (
                planes[0].samples[i],
                planes[1].samples[i],
                planes[2].samples[i],
            );
            let (r, g, b) = if cfg.styles[0].filter == 1 {
                let green = y - ((cb + cr) / 4.0).floor();
                (green + cr, green, green + cb)
            } else {
                (
                    y + 1.402 * cr,
                    y - 0.34413 * cb - 0.71414 * cr,
                    y + 1.772 * cb,
                )
            };
            planes[0].samples[i] = r;
            planes[1].samples[i] = g;
            planes[2].samples[i] = b;
        }
    }
    Ok(())
}
fn render(
    rgba: &mut [u8],
    g: &Geometry,
    meta: &Container,
    planes: &[Vec<u8>],
) -> Result<(), Error> {
    let channels = if meta.mapping.is_empty() {
        g.components.len()
    } else {
        meta.mapping.len()
    };
    let gray = meta.space == Some(17) || (meta.space.is_none() && channels <= 2);
    let color_count = if gray { 1 } else { 3 };
    if meta.space == Some(17) && !matches!(channels, 1 | 2) {
        return Err(bad("grayscale channel count"));
    }
    if channels < color_count || channels > 4 {
        return Err(unsupported(
            "multicomponent image requires an explicit display mapping",
        ));
    }
    let mut order = [0, 1, 2];
    let mut alpha = if channels == color_count + 1 {
        Some((channels - 1, false))
    } else {
        None
    };
    let mut seen = HashSet::new();
    for &(index, kind, association) in &meta.channels {
        if index >= channels || !seen.insert(index) {
            return Err(bad("channel definition index"));
        }
        match kind {
            0 => {
                if association == 0 && gray {
                    order[0] = index;
                } else {
                    if association == 0 || association as usize > color_count {
                        return Err(bad("colour association"));
                    }
                    order[association as usize - 1] = index;
                }
            }
            1 | 2 => {
                if association != 0 {
                    return Err(unsupported("per-colour alpha association"));
                }
                alpha = Some((index, kind == 2));
            }
            65535 => {}
            _ => return Err(bad("channel type")),
        }
    }
    for &(c, kind, column) in &meta.mapping {
        if c >= planes.len()
            || kind == 1 && (meta.palette.is_empty() || column >= meta.palette[0].len())
        {
            return Err(bad("component mapping index"));
        }
    }
    let width = g.x1 - g.x0;
    for y in g.y0..g.y1 {
        for x in g.x0..g.x1 {
            let mut values = [0u8; 4];
            for (channel, out) in values[..channels].iter_mut().enumerate() {
                let (c, kind, column) = if meta.mapping.is_empty() {
                    (channel, 0, 0)
                } else {
                    meta.mapping[channel]
                };
                let component = &g.components[c];
                let plane = &planes[c];
                let cx = x / component.sx;
                let cy = y / component.sy;
                let x0 = g.x0.div_ceil(component.sx);
                let y0 = g.y0.div_ceil(component.sy);
                let w = g.x1.div_ceil(component.sx) - x0;
                let shifted = if cx < x0 || cy < y0 {
                    0
                } else {
                    plane[(cy - y0) * w + cx - x0]
                };
                if kind == 1 {
                    *out = meta.palette[usize::from(shifted).min(meta.palette.len() - 1)][column];
                } else {
                    *out = (f64::from(shifted) * 255.0 / f64::from((1u32 << component.depth) - 1))
                        .round_ties_even() as u8;
                }
            }
            let mut rgb = if gray {
                [values[order[0]]; 3]
            } else {
                order.map(|i| values[i])
            };
            if meta.space == Some(18) {
                let y = f64::from(rgb[0]);
                let cb = f64::from(rgb[1]) - 128.0;
                let cr = f64::from(rgb[2]) - 128.0;
                rgb = [
                    y + 1.402 * cr,
                    y - 0.344136 * cb - 0.714136 * cr,
                    y + 1.772 * cb,
                ]
                .map(|v| v.trunc().clamp(0.0, 255.0) as u8);
            }
            let a = alpha.map_or(255, |(i, _)| values[i]);
            if alpha.is_some_and(|(_, premult)| premult) {
                rgb = rgb.map(|v| {
                    if a == 0 {
                        0
                    } else {
                        (u32::from(v) * 255 / u32::from(a)).min(255) as u8
                    }
                });
            }
            let out = ((y - g.y0) * width + x - g.x0) * 4;
            rgba[out..out + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    Ok(())
}

pub(super) struct DecodedImage {
    pub width: usize,
    pub height: usize,
    #[cfg(feature = "image-buffer-ex")]
    pub depth: u8,
    #[cfg(feature = "image-buffer-ex")]
    pub channels: usize,
    pub bytes: Vec<u8>,
}

pub(super) fn image(data: &[u8]) -> Result<(usize, usize, Vec<u8>), Error> {
    let image = image_inner::<false>(data)?;
    Ok((image.width, image.height, image.bytes))
}

#[cfg(feature = "image-buffer-ex")]
pub(super) fn image_ex(data: &[u8]) -> Result<DecodedImage, Error> {
    image_inner::<true>(data)
}

// Separate instantiations preserve the established RGBA8 fast/general path.
fn image_inner<const EX: bool>(data: &[u8]) -> Result<DecodedImage, Error> {
    let (s, meta) = container(data)?;
    if u16be(s, 0)? != 0xff4f || u16be(s, 2)? != 0xff51 {
        return Err(bad("SOC/SIZ required"));
    }
    let (_, b, mut p) = segment(s, 2)?;
    let g = geometry::<EX>(b)?;
    let nc = g.components.len();
    let channel_order = if EX {
        ex_channel_order(nc, &meta)?
    } else {
        [0, 1, 2, 3]
    };
    if let Some((w, h, n, bpc)) = meta.ihdr {
        if (w, h, n) != (g.x1 - g.x0, g.y1 - g.y0, nc) {
            return Err(bad("JP2/SIZ disagreement"));
        }
        let depths = if bpc == 255 {
            if meta.depths.len() != nc {
                return Err(bad("bpcc required"));
            }
            meta.depths.clone()
        } else {
            if !meta.depths.is_empty() {
                return Err(bad("unexpected bpcc"));
            }
            vec![bpc; nc]
        };
        for (c, &d) in g.components.iter().zip(&depths) {
            if (d & 127) + 1 != c.depth || (d & 128 != 0) != c.signed {
                return Err(bad("JP2/SIZ sample disagreement"));
            }
        }
    }
    let empty = Style {
        levels: 0,
        bw: 64,
        bh: 64,
        flags: 0,
        filter: 1,
        precincts: vec![255],
    };
    let mut cfg = Config {
        default: empty.clone(),
        default_quant: Vec::new(),
        styles: vec![empty; nc],
        quant: vec![Vec::new(); nc],
        roi: vec![0; nc],
        layers: 0,
        order: 0,
        mct: false,
        scod: 0,
        poc: Vec::new(),
        ht: false,
    };
    let mut ppm = BTreeMap::new();
    let mut tlm = BTreeMap::new();
    let mut seen_cod = false;
    let mut seen_qcd = false;
    let mut overrides = HashSet::new();
    let mut main_header = Vec::new();
    while u16be(s, p)? != 0xff90 {
        let (m, b, next) = segment(s, p)?;
        if matches!(m, 0xff53 | 0xff5d | 0xff5e) {
            let (c, _) = component_index(b, nc)?;
            if !overrides.insert((m, c)) {
                return Err(bad("duplicate component override"));
            }
        }
        match m {
            0xff30..=0xff3f => {}
            0xff60 => insert_packed(&mut ppm, b)?,
            0xff55 => insert_packed(&mut tlm, b)?,
            0xff52 => {
                if seen_cod {
                    return Err(bad("duplicate COD"));
                }
                main_header.push((m, b));
                seen_cod = true;
            }
            0xff5c => {
                if seen_qcd {
                    return Err(bad("duplicate QCD"));
                }
                main_header.push((m, b));
                seen_qcd = true;
            }
            _ => main_header.push((m, b)),
        }
        p = next;
    }
    if !seen_cod || !seen_qcd {
        return Err(bad("COD/QCD required"));
    }
    main_header.sort_by_key(|&(m, _)| marker_priority(m));
    for (m, b) in main_header {
        marker(m, b, &mut cfg)?;
    }
    if g.rsiz == 0x4000 && !cfg.ht {
        return Err(bad("extended capabilities require CAP"));
    }
    if EX {
        validate_ex_config(&cfg)?;
    }
    let mut qstyle = cfg.default.clone();
    qstyle.filter = 0;
    let _ = quant(&cfg.default_quant, &qstyle, 8)?;
    let bands = 1 + 3 * cfg.default.levels as usize;
    let expected = 1 + match cfg.default_quant[0] & 31 {
        0 => bands,
        1 => 2,
        _ => 2 * bands,
    };
    if cfg.default_quant.len() != expected {
        return Err(bad("default quantization band count"));
    }
    let packed_headers = packed(&ppm)?;
    let mut ppm_pos = 0;
    let mut tiles: BTreeMap<usize, Tile<'_>> = BTreeMap::new();
    let nx = (g.x1 - g.tx).div_ceil(g.tw);
    let ny = (g.y1 - g.ty).div_ceil(g.th);
    let mut tlm_lengths = Vec::new();
    for (_, b) in tlm {
        if b.is_empty() {
            return Err(bad("TLM"));
        }
        let st = b[0];
        let ti = usize::from((st >> 4) & 3);
        let len = if st & 64 != 0 { 4 } else { 2 };
        if ti > 2 || st & 143 != 0 || (b.len() - 1) % (ti + len) != 0 {
            return Err(bad("TLM fields"));
        }
        for v in b[1..].chunks_exact(ti + len) {
            let id = match ti {
                0 => None,
                1 => Some(v[0] as usize),
                _ => Some(u16be(v, 0)? as usize),
            };
            let length = if len == 4 {
                u32be(v, ti)? as usize
            } else {
                u16be(v, ti)? as usize
            };
            tlm_lengths.push((id, length));
        }
    }
    let mut parts_total = 0;
    while u16be(s, p)? == 0xff90 {
        let (_, b, header_end) = segment(s, p)?;
        if b.len() != 8 {
            return Err(bad("SOT length"));
        }
        let id = u16be(b, 0)? as usize;
        let psot = u32be(b, 2)? as usize;
        let index = b[6] as usize;
        let total = b[7];
        if id >= nx * ny {
            return Err(bad("tile ID"));
        }
        let end = if psot == 0 {
            if !s.ends_with(&[255, 217]) {
                return Err(bad("terminal EOC"));
            }
            s.len() - 2
        } else {
            p.checked_add(psot)
                .ok_or_else(|| bad("tile-part length overflow"))?
        };
        if end < header_end + 2 || end > s.len() {
            return Err(bad("tile-part extent"));
        }
        if let Some(&(tid, len)) = tlm_lengths.get(parts_total) {
            if tid.is_some_and(|t| t != id) || len != end - p {
                return Err(bad("TLM/SOT disagreement"));
            }
        }
        if !tiles.contains_key(&id) {
            // Bound repeated style/quantization/POC clones before allocation.
            let config_bytes = count(nc + 1, 512)?
                .checked_add(count(cfg.poc.len(), std::mem::size_of::<Poc>())?)
                .ok_or_else(|| bad("tile configuration overflow"))?;
            check(
                count(tiles.len() + 1, config_bytes)?,
                "JPEG2000 tile configurations",
            )?;
        }
        let tile = tiles.entry(id).or_insert_with(|| Tile {
            config: cfg.clone(),
            parts: Vec::new(),
            total: 0,
        });
        if index != tile.parts.len() || index >= 255 {
            return Err(bad("tile-part sequence"));
        }
        if total != 0 {
            if tile.total != 0 && tile.total != total {
                return Err(bad("tile-part total"));
            }
            tile.total = total;
        }
        if tile.total != 0 && index >= tile.total as usize {
            return Err(bad("tile-part index"));
        }
        let mut q = header_end;
        let mut ppt = BTreeMap::new();
        let mut plt = BTreeMap::new();
        let mut part_poc = Vec::new();
        let mut tile_header = Vec::new();
        while u16be(s, q)? != 0xff93 {
            let (m, b, next) = segment(s, q)?;
            if next > end {
                return Err(bad("tile header extent"));
            }
            match m {
                0xff30..=0xff3f => {}
                0xff61 => insert_packed(&mut ppt, b)?,
                0xff58 => insert_packed(&mut plt, b)?,
                _ => {
                    if index != 0 && matches!(m, 0xff52 | 0xff53 | 0xff5c | 0xff5d | 0xff5e) {
                        return Err(bad("coding override after first tile-part"));
                    }
                    tile_header.push((m, b));
                }
            }
            q = next;
        }
        q += 2;
        if q > end {
            return Err(bad("tile header extent"));
        }
        tile_header.sort_by_key(|&(m, _)| marker_priority(m));
        for (m, b) in tile_header {
            if m == 0xff5f {
                tile.config.poc.clear();
            }
            marker(m, b, &mut tile.config)?;
            if m == 0xff5f {
                part_poc = tile.config.poc.clone();
            }
        }
        if EX {
            validate_ex_config(&tile.config)?;
        }
        let mut headers = packed(&ppt)?;
        if !ppm.is_empty() {
            if !headers.is_empty() {
                return Err(bad("PPM/PPT conflict"));
            }
            let n = u32be(&packed_headers, ppm_pos)? as usize;
            ppm_pos += 4;
            headers = bytes(&packed_headers, ppm_pos, n)?.to_vec();
            ppm_pos += n;
        }
        let mut lengths = Vec::new();
        let mut len = 0usize;
        let mut continued = false;
        for v in packed(&plt)? {
            len = len
                .checked_mul(128)
                .and_then(|n| n.checked_add(usize::from(v & 127)))
                .ok_or_else(|| bad("PLT length overflow"))?;
            continued = v & 128 != 0;
            if !continued {
                lengths.push(len);
                len = 0;
            }
        }
        if continued {
            return Err(bad("truncated PLT value"));
        }
        tile.parts.push(Part {
            body: bytes(s, q, end - q)?,
            headers,
            poc: part_poc,
            lengths,
        });
        p = end;
        parts_total += 1;
    }
    if u16be(s, p)? != 0xffd9 || !s[p + 2..].is_empty() && s[p + 2..] != [255, 217] {
        return Err(bad("EOC or trailing data"));
    }
    if tiles.len() != nx * ny
        || ppm_pos != packed_headers.len()
        || !tlm_lengths.is_empty() && tlm_lengths.len() != parts_total
    {
        return Err(bad("incomplete tile/header directory"));
    }
    let width = g.x1 - g.x0;
    let height = g.y1 - g.y0;
    let mut rgba = array(
        count(count(width, height)?, output_pixel_bytes::<EX>(nc))?,
        0u8,
    )?;
    let mut component_samples = Vec::new();
    let mut output_bytes = rgba.len();
    for c in g.components.iter().filter(|_| !EX) {
        let n = count(
            g.x1.div_ceil(c.sx) - g.x0.div_ceil(c.sx),
            g.y1.div_ceil(c.sy) - g.y0.div_ceil(c.sy),
        )?;
        output_bytes = output_bytes
            .checked_add(n)
            .ok_or_else(|| bad("component storage overflow"))?;
        check(output_bytes, "JPEG2000 component storage")?;
        component_samples.push(array(n, 0u8)?);
    }
    for (id, tile) in tiles {
        if tile.total != 0 && tile.total as usize != tile.parts.len() {
            return Err(bad("tile-part total mismatch"));
        }
        let ix = id % nx;
        let iy = id / nx;
        let bounds = (
            (g.tx + ix * g.tw).max(g.x0),
            (g.ty + iy * g.th).max(g.y0),
            g.tx.saturating_add((ix + 1).saturating_mul(g.tw)).min(g.x1),
            g.ty.saturating_add((iy + 1).saturating_mul(g.th)).min(g.y1),
        );
        let TileWorkspace {
            mut planes,
            mut precincts,
            steps,
            retained_bytes: workspace,
        } = precincts::<EX>(&g, &tile.config, bounds)?;
        if EX {
            // Account for output + coefficients/lifting/block state + packet
            // ordering, consumed keys and replacement POC directories together.
            let packets = count(precincts.len(), tile.config.layers as usize)?;
            let progression = std::iter::once(tile.config.poc.as_slice())
                .chain(tile.parts.iter().map(|p| p.poc.as_slice()))
                .try_fold(0usize, |max, poc| -> Result<usize, Error> {
                    let work = if poc.is_empty() {
                        tile.config.layers as usize
                    } else {
                        poc.iter().try_fold(0usize, |n, p| {
                            n.checked_add(p.l1 as usize)
                                .ok_or_else(|| bad("EX progression overflow"))
                        })?
                    };
                    Ok(max.max(work))
                })?;
            let compressed = tile.parts.iter().try_fold(0usize, |total, part| {
                total
                    .checked_add(part.body.len())
                    .ok_or_else(|| bad("EX segment storage overflow"))
            })?;
            let ordering_bytes = count(count(precincts.len(), progression)?, 32)?;
            let directory = count(packets, 192)?
                .checked_add(count(compressed, 2)?)
                .and_then(|n| n.checked_add(ordering_bytes))
                .and_then(|n| n.checked_add(workspace))
                .ok_or_else(|| bad("EX workspace overflow"))?;
            check(directory, "JPEG2000 EX combined workspace")?;
        }
        let mut consumed = HashSet::new();
        if tile.config.styles.iter().any(|s| s.flags & 64 != 0) && !tile.config.ht {
            return Err(bad("HT coding requires CAP"));
        }
        let mut order = packet_order(
            &precincts,
            &tile.config,
            tile.parts
                .first()
                .filter(|p| !p.poc.is_empty())
                .map_or(cfg.poc.as_slice(), |p| p.poc.as_slice()),
        )?;
        let mut order_pos = 0;
        for part in &tile.parts {
            if !part.poc.is_empty() {
                order = packet_order(&precincts, &tile.config, &part.poc)?;
                order_pos = 0;
            }
            let mut offset = 0;
            let mut hpos = 0;
            let mut packet_index = 0;
            while offset < part.body.len() || hpos < part.headers.len() {
                while order_pos < order.len()
                    && consumed.contains(&(order[order_pos].precinct, order[order_pos].layer))
                {
                    order_pos += 1;
                }
                let key = *order
                    .get(order_pos)
                    .ok_or_else(|| bad("excess packet data"))?;
                let before = offset;
                let hb = hpos;
                packet(
                    part,
                    &mut offset,
                    &mut hpos,
                    &mut precincts[key.precinct],
                    key.layer,
                    &tile.config,
                )?;
                if let Some(&length) = part.lengths.get(packet_index) {
                    if length != offset - before + hpos - hb {
                        return Err(bad("PLT packet length mismatch"));
                    }
                }
                consumed.insert((key.precinct, key.layer));
                order_pos += 1;
                packet_index += 1;
            }
            if offset != part.body.len()
                || hpos != part.headers.len()
                || !part.lengths.is_empty() && packet_index != part.lengths.len()
            {
                return Err(bad("unconsumed packet data"));
            }
        }
        let expected = precincts
            .len()
            .checked_mul(tile.config.layers as usize)
            .ok_or_else(|| bad("packet count overflow"))?;
        if consumed.len() != expected {
            return Err(bad(&format!(
                "missing packets: {}/{expected}",
                consumed.len()
            )));
        }
        reconstruct::<EX>(&mut planes, precincts, &tile.config, &steps)?;
        if EX {
            let plane = &planes[0];
            // Subtract absolute origins before combining row and column offsets.
            let x0 = plane.x0 - g.x0;
            let y0 = plane.y0 - g.y0;
            let shift = 1i64 << (g.components[0].depth - 1);
            let max = (1i64 << g.components[0].depth) - 1;
            if tile.config.styles[0].filter == 0 {
                let channels = if nc == 1 { 1 } else { 4 };
                for y in 0..plane.h {
                    for x in 0..plane.w {
                        let index = y * plane.w + x;
                        let mut values = [0.0; 4];
                        for (v, p) in values.iter_mut().zip(&planes) {
                            *v = p.samples[index];
                        }
                        if tile.config.mct {
                            let rgb = inverse_ict(values[0], values[1], values[2]);
                            values[..3].copy_from_slice(&rgb);
                        }
                        let at = ((y0 + y) * width + x0 + x) * channels * 2;
                        for c in 0..channels {
                            let sample = if c == 3 && nc == 3 {
                                max as u16
                            } else {
                                let v = values[channel_order[c]];
                                if !v.is_finite() {
                                    return Err(bad("nonfinite EX colour sample"));
                                }
                                (v.round_ties_even() + shift as f64).clamp(0.0, max as f64) as u16
                            };
                            rgba[at + c * 2..at + c * 2 + 2].copy_from_slice(&sample.to_le_bytes());
                        }
                    }
                }
                continue;
            }
            if nc == 1 {
                for y in 0..plane.h {
                    for x in 0..plane.w {
                        let sample = (i64::from(plane.coefficients[y * plane.w + x]) + shift)
                            .clamp(0, max) as u16;
                        let at = ((y0 + y) * width + x0 + x) * 2;
                        rgba[at..at + 2].copy_from_slice(&sample.to_le_bytes());
                    }
                }
                continue;
            }
            for y in 0..plane.h {
                for x in 0..plane.w {
                    let index = y * plane.w + x;
                    let mut values = [0i64; 4];
                    for (value, p) in values.iter_mut().zip(&planes) {
                        *value = i64::from(p.coefficients[index]);
                    }
                    if tile.config.mct {
                        let rgb = inverse_rct_wide(
                            planes[0].coefficients[index],
                            planes[1].coefficients[index],
                            planes[2].coefficients[index],
                        );
                        values[..3].copy_from_slice(&rgb);
                    }
                    let at = ((y0 + y) * width + x0 + x) * 8;
                    for c in 0..4 {
                        let sample = if c == 3 && nc == 3 {
                            max as u16
                        } else {
                            (values[channel_order[c]] + shift).clamp(0, max) as u16
                        };
                        rgba[at + c * 2..at + c * 2 + 2].copy_from_slice(&sample.to_le_bytes());
                    }
                }
            }
            continue;
        }
        for (c, plane) in planes.iter().enumerate() {
            let component = &g.components[c];
            let x0 = g.x0.div_ceil(component.sx);
            let y0 = g.y0.div_ceil(component.sy);
            let w = g.x1.div_ceil(component.sx) - x0;
            for y in 0..plane.h {
                for x in 0..plane.w {
                    let sample = plane.samples[y * plane.w + x].round_ties_even()
                        + (1u32 << (component.depth - 1)) as f64;
                    component_samples[c][(plane.y0 + y - y0) * w + plane.x0 + x - x0] =
                        sample.clamp(0.0, ((1u32 << component.depth) - 1) as f64) as u8;
                }
            }
        }
    }
    if !EX {
        render(&mut rgba, &g, &meta, &component_samples)?;
    }
    Ok(DecodedImage {
        width,
        height,
        #[cfg(feature = "image-buffer-ex")]
        depth: g.components[0].depth,
        #[cfg(feature = "image-buffer-ex")]
        channels: if nc == 1 { 1 } else { 4 },
        bytes: rgba,
    })
}

// Keep the established reversible profile; bound the new floating profile.
fn validate_ex_config(cfg: &Config) -> Result<(), Error> {
    let filter = cfg.styles[0].filter;
    if cfg.mct && cfg.styles.len() < 3
        || cfg.ht
        || cfg.roi.iter().any(|&s| s != 0)
        || cfg
            .styles
            .iter()
            .any(|s| s.filter != filter || s.flags & 64 != 0 || filter == 0 && s.levels > 6)
        || cfg.quant.iter().any(|q| {
            q.first().is_none_or(|v| {
                if filter == 1 {
                    v & 31 != 0
                } else {
                    !matches!(v & 31, 1 | 2)
                }
            })
        })
    {
        return Err(unsupported(
            "EX requires uniform 5/3 scalar-free or 9/7 scalar-quantized (at most 6 levels); no mixed filters, Gray MCT, HT or ROI",
        ));
    }
    Ok(())
}

fn inverse_ict(y: f64, cb: f64, cr: f64) -> [f64; 3] {
    [
        y + 1.402 * cr,
        y - 0.34413 * cb - 0.71414 * cr,
        y + 1.772 * cb,
    ]
}

#[cfg(test)]
mod ex_color_tests {
    use super::inverse_rct_wide;
    #[test]
    fn reversible_color_floor_and_wide_intermediates() {
        assert_eq!(inverse_rct_wide(-1, -1, -2), [-2, 0, -1]);
        assert_eq!(
            inverse_rct_wide(i32::MAX, i32::MIN, i32::MAX),
            [4294967295, 2147483648, 0]
        );
        assert_eq!(
            inverse_rct_wide(i32::MIN, i32::MAX, i32::MIN),
            [-4294967295, -2147483647, 0]
        );
    }
}
