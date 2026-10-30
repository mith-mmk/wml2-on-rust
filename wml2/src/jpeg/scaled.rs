//! Baseline JPEG thumbnails using reduced IDCT, without a full-size raster.
use crate::draw::ImageBuffer;
use crate::error::{ImgError, ImgErrorKind};
use crate::image_info::{ImageDimensions, ImageSize};
use crate::limits::{self, DecodeLimits};
use crate::metadata::DataMap;
use std::collections::HashMap;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

type Error = Box<dyn std::error::Error>;
/// Native baseline IDCT reduction. Output dimensions are rounded up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JpegScale {
    Half,
    Quarter,
    Eighth,
}
impl JpegScale {
    pub const fn denominator(self) -> usize {
        match self {
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
        }
    }
}
/// Owned RGBA8 thumbnail and its source/output geometry. EXIF orientation is not
/// applied; `intrinsic_display` remains unknown. See `docs/jpeg-scaled.md`.
pub struct ScaledJpegImage {
    pub image: ImageBuffer,
    pub dimensions: ImageDimensions,
}
impl std::fmt::Debug for ScaledJpegImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaledJpegImage")
            .field("dimensions", &self.dimensions)
            .finish_non_exhaustive()
    }
}
/// Decode an 8-bit baseline, single-scan Gray/RGB/YCbCr JPEG using reduced IDCT.
/// Progressive, CMYK/YCCK and unsupported sampling return an error. The ordinary
/// full-resolution JPEG APIs retain their existing behavior.
pub fn image_from_scaled(bytes: &[u8], scale: JpegScale) -> Result<ScaledJpegImage, Error> {
    image_from_scaled_with_limits(bytes, scale, DecodeLimits::default())
}
/// As [`image_from_scaled`], with independent budgets. `pixels` admits the coded
/// canvas; `expanded_bytes` admits the reduced RGBA canvas plus bounded scratch
/// and metadata storage. Input is borrowed; no full-size pixel buffer is made.
pub fn image_from_scaled_with_limits(
    bytes: &[u8],
    scale: JpegScale,
    budget: DecodeLimits,
) -> Result<ScaledJpegImage, Error> {
    limits::scope(budget, || decode(bytes, scale, budget))
}
fn invalid(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::IllegalData,
        message.into(),
    ))
}
fn unsupported(message: &str) -> Error {
    Box::new(ImgError::new_const(
        ImgErrorKind::UnsupportedFeature,
        message.into(),
    ))
}
fn sum(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b)
        .ok_or_else(|| invalid("JPEG size overflow"))
}
#[derive(Clone, Copy, Default)]
struct Comp {
    id: u8,
    h: usize,
    v: usize,
    q: usize,
}
#[derive(Clone, Copy, Default)]
struct Scan {
    comp: usize,
    dc: usize,
    ac: usize,
}
struct Plan {
    width: usize,
    height: usize,
    count: usize,
    comp: [Comp; 3],
    scan: [Scan; 3],
    max_h: usize,
    max_v: usize,
    interval: usize,
    entropy: usize,
    metadata: usize,
    rgb: bool,
}
// Walk borrowed marker payloads. This is run once for admission and once for
// materialization; even a long chain of tiny APP segments needs no segment Vec.
fn headers<'a>(
    bytes: &'a [u8],
    mut visit: impl FnMut(u8, &'a [u8]) -> Result<(), Error>,
) -> Result<usize, Error> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(invalid("JPEG SOI required"));
    }
    let mut at = 2;
    loop {
        if bytes.get(at) != Some(&0xff) {
            return Err(invalid("JPEG marker required"));
        }
        while bytes.get(at) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes
            .get(at)
            .ok_or_else(|| invalid("Truncated JPEG marker"))?;
        at += 1;
        if matches!(marker, 0 | 0xd0..=0xd9 | 1) {
            return Err(invalid("Unexpected standalone JPEG marker"));
        }
        let len = bytes
            .get(at..at + 2)
            .ok_or_else(|| invalid("Truncated JPEG segment length"))?;
        let len = u16::from_be_bytes([len[0], len[1]]) as usize;
        if len < 2 {
            return Err(invalid("Invalid JPEG segment length"));
        }
        let end = sum(at, len)?;
        let data = bytes
            .get(at + 2..end)
            .ok_or_else(|| invalid("Truncated JPEG segment"))?;
        visit(marker, data)?;
        at = end;
        if marker == 0xda {
            return Ok(at);
        }
    }
}
fn plan(bytes: &[u8], budget: DecodeLimits) -> Result<Plan, Error> {
    let mut p = Plan {
        width: 0,
        height: 0,
        count: 0,
        comp: [Comp::default(); 3],
        scan: [Scan::default(); 3],
        max_h: 1,
        max_v: 1,
        interval: 0,
        entropy: 0,
        metadata: 0,
        rgb: false,
    };
    let mut adobe = None;
    let mut jfif = false;
    p.entropy = headers(bytes, |marker, data| {
        match marker {
            0xc0 => {
                if p.count != 0 || data.len() < 6 {
                    return Err(invalid("Invalid or duplicate JPEG SOF"));
                }
                if data[0] != 8 {
                    return Err(unsupported("Reduced JPEG requires 8-bit baseline"));
                }
                let n = data[5] as usize;
                if !matches!(n, 1 | 3) {
                    return Err(unsupported(
                        "Reduced JPEG supports Gray and three-component RGB/YCbCr",
                    ));
                }
                if data.len() != 6 + 3 * n {
                    return Err(invalid("Invalid JPEG SOF length"));
                }
                p.height = u16::from_be_bytes([data[1], data[2]]) as usize;
                p.width = u16::from_be_bytes([data[3], data[4]]) as usize;
                if p.width == 0 || p.height == 0 {
                    return Err(invalid("JPEG dimensions must be positive"));
                }
                limits::check(p.width * p.height, budget.pixels, "coded JPEG pixels")?;
                p.count = n;
                for i in 0..n {
                    let a = 6 + 3 * i;
                    let c = Comp {
                        id: data[a],
                        h: (data[a + 1] >> 4) as usize,
                        v: (data[a + 1] & 15) as usize,
                        q: data[a + 2] as usize,
                    };
                    if !(1..=4).contains(&c.h)
                        || !(1..=4).contains(&c.v)
                        || c.q > 3
                        || p.comp[..i].iter().any(|v| v.id == c.id)
                    {
                        return Err(invalid("Invalid JPEG component selectors/sampling"));
                    }
                    p.max_h = p.max_h.max(c.h);
                    p.max_v = p.max_v.max(c.v);
                    p.comp[i] = c;
                }
                // A noninterleaved grayscale scan has one block per MCU,
                // irrespective of its (otherwise immaterial) SOF sampling.
                if n == 1 {
                    p.max_h = 1;
                    p.max_v = 1;
                    p.comp[0].h = 1;
                    p.comp[0].v = 1;
                }
                if p.comp[..n].iter().map(|c| c.h * c.v).sum::<usize>() > 10 {
                    return Err(invalid("Too many JPEG blocks per MCU"));
                }
                if p.comp[..n]
                    .iter()
                    .any(|c| !p.max_h.is_multiple_of(c.h) || !p.max_v.is_multiple_of(c.v))
                {
                    return Err(unsupported(
                        "Fractional JPEG sampling is not supported by reduced decode",
                    ));
                }
            }
            0xda => {
                let n = p.count;
                if n == 0 || data.len() != 1 + 2 * n + 3 || data[0] as usize != n {
                    return Err(unsupported(
                        "Reduced JPEG requires one complete interleaved scan (or one grayscale scan)",
                    ));
                }
                let mut seen = [false; 3];
                for i in 0..n {
                    let id = data[1 + 2 * i];
                    let c = p.comp[..n]
                        .iter()
                        .position(|c| c.id == id)
                        .ok_or_else(|| invalid("Unknown JPEG scan component"))?;
                    let table = data[2 + 2 * i];
                    if seen[c] || table >> 4 > 3 || table & 15 > 3 {
                        return Err(invalid("Invalid JPEG scan selectors"));
                    }
                    seen[c] = true;
                    p.scan[i] = Scan {
                        comp: c,
                        dc: (table >> 4) as usize,
                        ac: (table & 15) as usize,
                    };
                }
                if data[1 + 2 * n..] != [0, 63, 0] {
                    return Err(invalid("Invalid baseline JPEG scan parameters"));
                }
            }
            0xdb | 0xc4 => (), // Tables are validated before entropy/output allocation.
            0xdd => {
                if data.len() != 2 {
                    return Err(invalid("Invalid JPEG DRI length"));
                }
                p.interval = u16::from_be_bytes([data[0], data[1]]) as usize;
            }
            0xe0..=0xef | 0xfe => {
                // Includes fixed per-entry/key overhead, not only payload bytes.
                p.metadata = sum(p.metadata, sum(data.len(), 256)?)?;
                limits::check(p.metadata, budget.metadata_bytes, "JPEG metadata")?;
                if marker == 0xe0 && data.starts_with(b"JFIF\0") {
                    if data.len() < 14 {
                        return Err(invalid("Truncated JPEG JFIF"));
                    }
                    jfif = true;
                }
                if marker == 0xee && data.starts_with(b"Adobe") {
                    if data.len() < 12 {
                        return Err(invalid("Truncated JPEG Adobe APP14"));
                    }
                    if adobe.is_some_and(|v| v != data[11]) {
                        return Err(invalid("Conflicting JPEG Adobe transforms"));
                    }
                    adobe = Some(data[11]);
                }
            }
            _ => {
                return Err(unsupported(
                    "Reduced JPEG requires baseline SOF0 Huffman coding",
                ));
            }
        }
        Ok(())
    })?;
    p.rgb = if p.count == 1 {
        false
    } else {
        match adobe {
            Some(0) => true,
            Some(1) => false,
            Some(_) => return Err(unsupported("Unsupported reduced JPEG color transform")),
            None => !jfif && p.comp.map(|c| c.id) == *b"RGB",
        }
    };
    Ok(p)
}
#[derive(Clone)]
struct Huff {
    min: [i32; 16],
    max: [i32; 16],
    pos: [usize; 16],
    val: [u8; 256],
}
struct Tables {
    q: [[u8; 64]; 4],
    present: [bool; 4],
    dc: [Option<Huff>; 4],
    ac: [Option<Huff>; 4],
}
impl Tables {
    fn read(bytes: &[u8]) -> Result<Self, Error> {
        let mut t = Self {
            q: [[0; 64]; 4],
            present: [false; 4],
            dc: std::array::from_fn(|_| None),
            ac: std::array::from_fn(|_| None),
        };
        headers(bytes, |marker, mut data| {
            if marker == 0xdb {
                if data.is_empty() {
                    return Err(invalid("Empty JPEG quantization segment"));
                }
                while !data.is_empty() {
                    let selector = data[0];
                    if selector >> 4 != 0 || selector & 15 > 3 {
                        return Err(invalid("Invalid baseline JPEG quantization selector"));
                    }
                    let q = data
                        .get(1..65)
                        .ok_or_else(|| invalid("Truncated JPEG quantization table"))?;
                    if q.contains(&0) {
                        return Err(invalid("Zero JPEG quantization value"));
                    }
                    let id = (selector & 15) as usize;
                    t.q[id].copy_from_slice(q);
                    t.present[id] = true;
                    data = &data[65..];
                }
            } else if marker == 0xc4 {
                if data.is_empty() {
                    return Err(invalid("Empty JPEG Huffman segment"));
                }
                while !data.is_empty() {
                    let prefix = data
                        .get(..17)
                        .ok_or_else(|| invalid("Truncated JPEG Huffman table"))?;
                    let selector = prefix[0];
                    if selector >> 4 > 1 || selector & 15 > 3 {
                        return Err(invalid("Invalid JPEG Huffman selector"));
                    }
                    let mut h = Huff {
                        min: [0; 16],
                        max: [-1; 16],
                        pos: [0; 16],
                        val: [0; 256],
                    };
                    let mut n = 0;
                    let mut code = 0i32;
                    for (i, &count) in prefix[1..].iter().enumerate() {
                        let count = count as usize;
                        h.pos[i] = n;
                        h.min[i] = code;
                        if count != 0 {
                            h.max[i] = code + count as i32 - 1;
                        }
                        code += count as i32;
                        // JPEG forbids the all-ones code (padding must be unambiguous).
                        if code >= 1 << (i + 1) {
                            return Err(invalid("Oversubscribed/complete JPEG Huffman tree"));
                        }
                        code *= 2;
                        n += count;
                    }
                    if n == 0 || n > 256 {
                        return Err(invalid("Invalid JPEG Huffman symbol count"));
                    }
                    let values = data
                        .get(17..17 + n)
                        .ok_or_else(|| invalid("Truncated JPEG Huffman symbols"))?;
                    let ac = selector >> 4 == 1;
                    if values.iter().any(|&v| {
                        if ac {
                            v & 15 > 10 || (v & 15 == 0 && v != 0 && v != 0xf0)
                        } else {
                            v > 11
                        }
                    }) {
                        return Err(invalid("Invalid baseline JPEG Huffman symbol"));
                    }
                    h.val[..n].copy_from_slice(values);
                    let target = if ac { &mut t.ac } else { &mut t.dc };
                    target[(selector & 15) as usize] = Some(h);
                    data = &data[17 + n..];
                }
            }
            Ok(())
        })?;
        Ok(t)
    }
}
// A strict bounded entropy reader: no recovery into another scan and no
// read-ahead past a marker. Restart padding is discarded only at MCU boundaries.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
    byte: u8,
    left: u8,
    rst: u8,
}
impl<'a> Bits<'a> {
    fn marker(&mut self) -> Result<u8, Error> {
        if self.bytes.get(self.at) != Some(&0xff) {
            return Err(invalid("JPEG marker expected after entropy"));
        }
        while self.bytes.get(self.at) == Some(&0xff) {
            self.at += 1;
        }
        let m = *self
            .bytes
            .get(self.at)
            .ok_or_else(|| invalid("Truncated JPEG entropy marker"))?;
        self.at += 1;
        self.left = 0;
        Ok(m)
    }
    fn bits(&mut self, n: usize) -> Result<i32, Error> {
        let mut value = 0;
        for _ in 0..n {
            if self.left == 0 {
                self.byte = *self
                    .bytes
                    .get(self.at)
                    .ok_or_else(|| invalid("Truncated JPEG entropy"))?;
                self.at += 1;
                if self.byte == 0xff {
                    if self.bytes.get(self.at) != Some(&0) {
                        return Err(invalid("Unexpected marker inside JPEG entropy unit"));
                    }
                    self.at += 1;
                }
                self.left = 8;
            }
            self.left -= 1;
            value = (value << 1) | ((self.byte >> self.left) & 1) as i32;
        }
        Ok(value)
    }
    fn symbol(&mut self, h: &Huff) -> Result<u8, Error> {
        let mut code = 0;
        for i in 0..16 {
            code = code * 2 + self.bits(1)?;
            if code >= h.min[i] && code <= h.max[i] {
                return Ok(h.val[h.pos[i] + (code - h.min[i]) as usize]);
            }
        }
        Err(invalid("Invalid JPEG Huffman code"))
    }
    fn signed(&mut self, n: usize) -> Result<i32, Error> {
        let v = self.bits(n)?;
        Ok(if n > 0 && v < 1 << (n - 1) {
            v + 1 - (1 << n)
        } else {
            v
        })
    }
    fn block(&mut self, dc: &Huff, ac: &Huff, pred: &mut i32) -> Result<[i32; 64], Error> {
        let size = self.symbol(dc)? as usize;
        *pred = pred
            .checked_add(self.signed(size)?)
            .ok_or_else(|| invalid("JPEG DC predictor overflow"))?;
        let mut zz = [0; 64];
        zz[0] = *pred;
        let mut k = 1;
        while k < 64 {
            let rs = self.symbol(ac)?;
            if rs == 0 {
                break;
            }
            if rs == 0xf0 {
                k += 16;
                if k > 64 {
                    return Err(invalid("JPEG zero run exceeds block"));
                }
                continue;
            }
            k += (rs >> 4) as usize;
            if k >= 64 {
                return Err(invalid("JPEG AC run exceeds block"));
            }
            zz[k] = self.signed((rs & 15) as usize)?;
            k += 1;
        }
        Ok(zz)
    }
    fn restart(&mut self) -> Result<(), Error> {
        if self.marker()? != 0xd0 + self.rst {
            return Err(invalid("Unexpected JPEG restart sequence"));
        }
        self.rst = (self.rst + 1) & 7;
        Ok(())
    }
}
// Each basis averages adjacent spatial samples of the *unclamped* inverse
// transform. This is the reduced-IDCT definition used by libjpeg jidctred.c,
// including odd high-frequency coefficients; it is not a cropped coefficient
// matrix or a resize of a reconstructed raster. N=1 is exactly DC/8.
struct Kernel {
    n: usize,
    basis: [[f64; 8]; 8],
}
impl Kernel {
    fn new(n: usize) -> Self {
        let mut basis = [[0.0; 8]; 8];
        let span = 8 / n;
        for (x, row) in basis.iter_mut().enumerate().take(n) {
            for (u, b) in row.iter_mut().enumerate() {
                let c = if u == 0 { FRAC_1_SQRT_2 } else { 1.0 };
                *b = (0..span)
                    .map(|k| (((2 * (x * span + k) + 1) * u) as f64 * PI / 16.0).cos())
                    .sum::<f64>()
                    * c
                    / (2 * span) as f64;
                if b.abs() < 1e-14 {
                    *b = 0.0;
                }
            }
        }
        Self { n, basis }
    }
    fn transform(&self, zz: &[i32; 64], q: &[u8; 64]) -> [u8; 64] {
        let mut output = [0; 64];
        let n = self.n;
        if n == 1 || zz[1..].iter().all(|&v| v == 0) {
            output[..n * n].fill(sample(zz[0] as f64 * q[0] as f64 / 8.0 + 128.0));
            return output;
        }
        match n {
            2 => self.transform_n::<2>(zz, q),
            4 => self.transform_n::<4>(zz, q),
            8 => self.transform_n::<8>(zz, q),
            _ => unreachable!(),
        }
    }
    fn transform_n<const N: usize>(&self, zz: &[i32; 64], q: &[u8; 64]) -> [u8; 64] {
        let mut coeff = [[0.0; 8]; 8];
        for k in 0..64 {
            let z = super::util::ZIG_ZAG_SEQUENCE[k];
            coeff[k / 8][k % 8] = zz[z] as f64 * q[z] as f64;
        }
        // Mirrored outputs share the even terms and negate the odd terms.
        // Reduced bases also eliminate u=4 (N=4), or u=2,4,6 (N=2).
        let mut tmp = [[0.0; 8]; 8];
        for v in 0..8 {
            for x in 0..N / 2 {
                let (even, odd) = self.terms::<N>(&coeff[v], x);
                tmp[v][x] = even + odd;
                tmp[v][N - 1 - x] = even - odd;
            }
        }
        let mut out = [0; 64];
        for x in 0..N {
            let column = std::array::from_fn(|v| tmp[v][x]);
            for y in 0..N / 2 {
                let (even, odd) = self.terms::<N>(&column, y);
                out[y * N + x] = sample(even + odd + 128.0);
                out[(N - 1 - y) * N + x] = sample(even - odd + 128.0);
            }
        }
        out
    }
    #[inline]
    fn terms<const N: usize>(&self, c: &[f64; 8], x: usize) -> (f64, f64) {
        let b = &self.basis[x];
        let mut even = c[0] * b[0];
        if N != 2 {
            even += c[2] * b[2] + c[6] * b[6];
        }
        if N == 8 {
            even += c[4] * b[4];
        }
        let odd = c[1] * b[1] + c[3] * b[3] + c[5] * b[5] + c[7] * b[7];
        (even, odd)
    }
}
fn sample(v: f64) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

fn metadata(bytes: &[u8], p: &Plan) -> Result<HashMap<String, DataMap>, Error> {
    let mut map = HashMap::new();
    map.insert("Format".into(), DataMap::Ascii("JPEG".into()));
    map.insert("width".into(), DataMap::UInt(p.width as u64));
    map.insert("height".into(), DataMap::UInt(p.height as u64));
    map.insert("Bits per pixel".into(), DataMap::UInt(8));
    map.insert(
        "Color Space".into(),
        DataMap::Ascii(
            if p.count == 1 {
                "Gray"
            } else if p.rgb {
                "RGB"
            } else {
                "YUV"
            }
            .into(),
        ),
    );
    let mut chunks: [Option<&[u8]>; 255] = [None; 255];
    let mut total = 0;
    let mut valid = true;
    headers(bytes, |marker, data| {
        if !(0xe0..=0xef).contains(&marker) && marker != 0xfe {
            return Ok(());
        }
        if marker == 0xe2 && data.starts_with(b"ICC_PROFILE\0") {
            if data.len() < 14 {
                valid = false;
                return Ok(());
            }
            let number = data[12] as usize;
            let count = data[13] as usize;
            if number == 0 || number > count || count == 0 || (total != 0 && total != count) {
                valid = false;
                return Ok(());
            }
            total = count;
            if chunks[number - 1].replace(&data[14..]).is_some() {
                valid = false;
            }
        } else if marker == 0xe1 && data.starts_with(b"Exif\0\0") {
            // Opaque, bounded EXIF is preserved without following TIFF offsets
            // or interpreting/applying orientation. No parsed-EXIF allocation.
            map.insert("EXIF Raw".into(), DataMap::Raw(data.to_vec()));
        } else if marker == 0xe1 && data.starts_with(b"http://ns.adobe.com/xap/1.0/\0") {
            map.insert(
                "http://ns.adobe.com/xap/1.0/".into(),
                DataMap::Ascii(String::from_utf8_lossy(&data[29..]).into_owned()),
            );
        } else if marker == 0xfe {
            map.insert(
                "comment".into(),
                DataMap::Ascii(String::from_utf8_lossy(data).into_owned()),
            );
        } else {
            map.insert(
                format!("App{} Raw", marker - 0xe0),
                DataMap::Raw(data.to_vec()),
            );
        }
        Ok(())
    })?;
    if valid && total > 0 && chunks[..total].iter().all(Option::is_some) {
        let len = chunks[..total]
            .iter()
            .flatten()
            .try_fold(0usize, |len, c| sum(len, c.len()))?;
        let mut profile = Vec::new();
        profile.try_reserve_exact(len)?;
        for c in chunks[..total].iter().flatten() {
            profile.extend_from_slice(c);
        }
        let key = if p.count == 3 && profile.get(16..20) == Some(b"RGB ") {
            "ICC Profile"
        } else {
            "Source ICC Profile"
        };
        map.insert(key.into(), DataMap::ICCProfile(profile));
    }
    Ok(map)
}
fn decode(bytes: &[u8], scale: JpegScale, budget: DecodeLimits) -> Result<ScaledJpegImage, Error> {
    limits::check(bytes.len(), budget.input_bytes, "JPEG input")?;
    limits::check(1, budget.frames, "JPEG frames")?;
    // Fixed-size header/table/entropy/IDCT/MCU storage, including stack arrays.
    const SCRATCH: usize = 64 * 1024;
    limits::check(SCRATCH, budget.expanded_bytes, "JPEG scratch")?;
    let p = plan(bytes, budget)?;
    let denom = scale.denominator();
    let n = 8 / denom;
    let width = p.width.div_ceil(denom);
    let height = p.height.div_ceil(denom);
    let output = width
        .checked_mul(height)
        .and_then(|v| v.checked_mul(4))
        .ok_or_else(|| invalid("JPEG output size overflow"))?;
    let workspace = p
        .metadata
        .checked_mul(8)
        .and_then(|v| v.checked_add(SCRATCH))
        .and_then(|v| v.checked_add(output))
        .ok_or_else(|| invalid("JPEG workspace overflow"))?;
    limits::check(
        workspace,
        budget.expanded_bytes,
        "JPEG reduced canvas, scratch and metadata",
    )?;
    limits::check(output, budget.animation_bytes, "JPEG retained canvas")?;
    let tables = Tables::read(bytes)?;
    for slot in &p.scan[..p.count] {
        if !tables.present[p.comp[slot.comp].q]
            || tables.dc[slot.dc].is_none()
            || tables.ac[slot.ac].is_none()
        {
            return Err(invalid("Missing JPEG quantization/Huffman table"));
        }
    }
    let kernels = [
        Kernel::new(1),
        Kernel::new(2),
        Kernel::new(4),
        Kernel::new(8),
    ];
    let mut sizes = [n; 3];
    for (i, c) in p.comp[..p.count].iter().enumerate() {
        // Retain a larger chroma IDCT when it exactly covers the reduced luma
        // grid (e.g. 4:2:0 half-size: Y=4, Cb/Cr=8), as in libjpeg jdmaster.c.
        while sizes[i] < 8
            && (p.max_h * n).is_multiple_of(c.h * sizes[i] * 2)
            && (p.max_v * n).is_multiple_of(c.v * sizes[i] * 2)
        {
            sizes[i] *= 2;
        }
    }
    let mw = p.max_h * n;
    let mh = p.max_v * n;
    let cols = p.width.div_ceil(p.max_h * 8);
    let rows = p.height.div_ceil(p.max_v * 8);
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(output)?;
    pixels.resize(output, 0);
    let mut bits = Bits {
        bytes,
        at: p.entropy,
        byte: 0,
        left: 0,
        rst: 0,
    };
    let mut pred = [0i32; 3];
    let mut planes = [[[0u8; 64]; 10]; 3];
    for my in 0..rows {
        for mx in 0..cols {
            let mcu = my * cols + mx;
            if mcu != 0 && p.interval != 0 && mcu % p.interval == 0 {
                bits.restart()?;
                pred.fill(0);
            }
            for slot in &p.scan[..p.count] {
                let ci = slot.comp;
                let c = p.comp[ci];
                let kernel = &kernels[sizes[ci].ilog2() as usize];
                for block in planes[ci].iter_mut().take(c.h * c.v) {
                    let zz = bits.block(
                        tables.dc[slot.dc].as_ref().unwrap(),
                        tables.ac[slot.ac].as_ref().unwrap(),
                        &mut pred[ci],
                    )?;
                    *block = kernel.transform(&zz, &tables.q[c.q]);
                }
            }
            for y in 0..mh.min(height - my * mh) {
                for x in 0..mw.min(width - mx * mw) {
                    let mut channel = [0u8; 3];
                    for ci in 0..p.count {
                        let c = p.comp[ci];
                        let size = sizes[ci];
                        let sx = x / (mw / (c.h * size));
                        let sy = y / (mh / (c.v * size));
                        let block = (sy / size) * c.h + sx / size;
                        channel[ci] = planes[ci][block][(sy % size) * size + sx % size];
                    }
                    let rgb = if p.count == 1 {
                        [channel[0]; 3]
                    } else if p.rgb {
                        channel
                    } else {
                        let yy = channel[0] as f64;
                        let cb = channel[1] as f64 - 128.0;
                        let cr = channel[2] as f64 - 128.0;
                        [
                            sample(yy + 1.402 * cr),
                            sample(yy - 0.344136 * cb - 0.714136 * cr),
                            sample(yy + 1.772 * cb),
                        ]
                    };
                    let at = ((my * mh + y) * width + mx * mw + x) * 4;
                    pixels[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
            }
        }
    }
    if bits.marker()? != 0xd9 {
        return Err(unsupported("Reduced JPEG requires EOI after a single scan"));
    }
    let mut image = ImageBuffer::from_buffer(width, height, pixels);
    image.metadata = Some(metadata(bytes, &p)?);
    Ok(ScaledJpegImage {
        image,
        dimensions: ImageDimensions {
            coded: ImageSize {
                width: p.width as u32,
                height: p.height as u32,
            },
            output: Some(ImageSize {
                width: width as u32,
                height: height as u32,
            }),
            intrinsic_display: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_entropy_run_and_predictor_overflow_return_errors() {
        fn single(value: u8) -> Huff {
            let mut h = Huff {
                min: [0; 16],
                max: [-1; 16],
                pos: [0; 16],
                val: [0; 256],
            };
            h.max[0] = 0;
            h.val[0] = value;
            h
        }
        let mut bits = Bits {
            bytes: &[0],
            at: 0,
            byte: 0,
            left: 0,
            rst: 0,
        };
        assert!(bits.block(&single(0), &single(0xf0), &mut 0).is_err());
        let mut bits = Bits {
            bytes: &[0b01000000],
            at: 0,
            byte: 0,
            left: 0,
            rst: 0,
        };
        assert!(bits.block(&single(1), &single(0), &mut i32::MAX).is_err());
    }
    #[test]
    fn reduced_kernel_uses_high_ac_and_unclamped_transform() {
        let q = [1; 64];
        let mut zz = [0; 64];
        // Natural horizontal frequency 7: beyond a naive 4x4 low-pass crop.
        zz[super::super::util::ZIG_ZAG_SEQUENCE[7]] = 4096;
        let four = Kernel::new(4).transform(&zz, &q);
        assert!(four[..16].iter().any(|&v| v != 128));
        assert_eq!(Kernel::new(1).transform(&zz, &q)[0], 128);
        // A large AC clips full-resolution samples before a spatial resize;
        // reduced IDCT averages before clamping and therefore differs.
        let full = Kernel::new(8).transform(&zz, &q);
        let spatial = (full[0] as u16 + full[1] as u16 + full[8] as u16 + full[9] as u16 + 2) / 4;
        assert_ne!(four[0] as u16, spatial);
    }
}
