//! Native/WASM behavioral checks for the opt-in JPEG2000 resolution API.
use bin_rs::reader::BytesReader;
use wml2::{draw::*, jpeg2000, limits::DecodeLimits, metadata::DataMap};
type Error = Box<dyn std::error::Error>;
#[cfg(feature = "image-buffer-ex")]
#[allow(dead_code)]
#[path = "jpeg2000_ex_checks.rs"]
mod old_ex;

#[derive(Default, Debug)]
pub struct Difference {
    pub maximum: u16,
    pub total: u64,
    pub samples: usize,
}
fn reference(actual: &[u8], expected: &[u8], bits: u8) -> Difference {
    assert_eq!(actual.len(), expected.len());
    let mut result = Difference::default();
    if bits == 8 {
        for (&a, &b) in actual.iter().zip(expected) {
            let error = u16::from(a.abs_diff(b));
            result.maximum = result.maximum.max(error);
            result.total += u64::from(error);
            result.samples += 1;
        }
    } else {
        for (a, b) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
            let a = u16::from_le_bytes([a[0], a[1]]);
            let b = u16::from_le_bytes([b[0], b[1]]);
            assert!(u32::from(a) < 1u32 << bits);
            let error = a.abs_diff(b);
            result.maximum = result.maximum.max(error);
            result.total += u64::from(error);
            result.samples += 1;
        }
    }
    // Independent reduced references establish these source-domain regression
    // caps. Old full-resolution reference checks keep their own unchanged caps.
    assert!(
        result.maximum <= if bits == 16 { 4 } else { 1 },
        "{result:?}"
    );
    result
}

pub fn legacy(data: &[u8], reduce: u8, limits: DecodeLimits) -> Result<ImageBuffer, Error> {
    let mut image = ImageBuffer::new();
    jpeg2000::decode_with_reduction(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        reduce,
        limits,
    )?;
    Ok(image)
}
pub fn legacy_pixels(
    data: &[u8],
    expected: &[u8],
    reduce: u8,
    width: usize,
    height: usize,
) -> Difference {
    let image = legacy(data, reduce, DecodeLimits::default()).unwrap();
    assert_eq!((image.width, image.height), (width, height));
    reference(image.buffer.as_ref().unwrap(), expected, 8)
}
pub fn legacy_zero(data: &[u8]) {
    let a = image_load(data).unwrap();
    let b = legacy(data, 0, DecodeLimits::default()).unwrap();
    assert_eq!((a.width, a.height), (b.width, b.height));
    assert_eq!(a.buffer, b.buffer);
}

#[cfg(feature = "image-buffer-ex")]
pub fn format(bits: u8, nc: usize) -> PixelFormatEx {
    match (bits, nc) {
        (12, 1) => PixelFormatEx::Gray12,
        (16, 1) => PixelFormatEx::Gray16,
        (12, _) => PixelFormatEx::Rgba12,
        (16, _) => PixelFormatEx::Rgba16,
        _ => panic!("reference depth"),
    }
}
#[cfg(feature = "image-buffer-ex")]
pub fn ex_callback(
    data: &[u8],
    fmt: PixelFormatEx,
    policy: PrecisionConversion,
    reduce: u8,
    limits: DecodeLimits,
) -> Result<ImageBufferEx, Error> {
    let mut image = ImageBufferEx::with_storage(fmt, policy);
    jpeg2000::decode_ex_with_reduction(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        reduce,
        limits,
    )?;
    Ok(image)
}
#[cfg(feature = "image-buffer-ex")]
pub fn ex_pixels(
    data: &[u8],
    expected: &[u8],
    reduce: u8,
    width: usize,
    height: usize,
    bits: u8,
    nc: usize,
) -> Difference {
    let fmt = format(bits, nc);
    let image = jpeg2000::image_from_ex_with_reduction(
        data,
        fmt,
        PrecisionConversion::Exact,
        reduce,
        DecodeLimits::default(),
    )
    .unwrap();
    let callback = ex_callback(
        data,
        fmt,
        PrecisionConversion::Round,
        reduce,
        DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!((image.width(), image.height()), (width, height));
    assert_eq!(image.bytes(), callback.bytes());
    assert_eq!(image.source_format(), Some(fmt));
    assert_eq!(callback.source_format(), Some(fmt));
    assert_eq!(image.source_stride(), None);
    assert_eq!(image.precision_conversion(), PrecisionConversion::Exact);
    assert_eq!(callback.precision_conversion(), PrecisionConversion::Round);
    let row = width * if nc == 1 { 2 } else { 8 };
    assert_eq!(image.stride(), row);
    assert!(image.metadata.is_none());
    reference(image.bytes().unwrap(), expected, bits)
}
#[cfg(feature = "image-buffer-ex")]
pub fn ex_zero(data: &[u8], bits: u8, nc: usize) {
    let fmt = format(bits, nc);
    for policy in [PrecisionConversion::Exact, PrecisionConversion::Round] {
        let a = image_from_ex(data, fmt, policy).unwrap();
        let b =
            jpeg2000::image_from_ex_with_reduction(data, fmt, policy, 0, DecodeLimits::default())
                .unwrap();
        assert_eq!(
            (a.width(), a.height(), a.stride()),
            (b.width(), b.height(), b.stride())
        );
        assert_eq!(a.bytes(), b.bytes());
        assert_eq!(a.source_format(), b.source_format());
        assert_eq!(a.source_stride(), b.source_stride());
        assert_eq!(a.precision_conversion(), b.precision_conversion());
    }
}

struct LegacyProbe {
    image: ImageBuffer,
    events: Vec<u8>,
    abort: u8,
    fail: u8,
}
impl Default for LegacyProbe {
    fn default() -> Self {
        Self {
            image: ImageBuffer::new(),
            events: Vec::new(),
            abort: 0,
            fail: 0,
        }
    }
}
impl LegacyProbe {
    fn event(&mut self, event: u8) -> Response {
        self.events.push(event);
        if self.fail == event {
            return Err("reduced callback error".into());
        }
        Ok((self.abort == event).then(CallbackResponse::abort))
    }
}
impl DrawCallback for LegacyProbe {
    fn init(&mut self, w: usize, h: usize, o: Option<InitOptions>) -> Response {
        self.image.init(w, h, o)?;
        self.event(1)
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        p: &[u8],
        o: Option<DrawOptions>,
    ) -> Response {
        self.image.draw(x, y, w, h, p, o)?;
        self.event(2)
    }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
        self.event(3)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        panic!("static only")
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        panic!("no verbose")
    }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
        panic!("no inferred metadata")
    }
}
fn legacy_probe(
    data: &[u8],
    reduce: u8,
    limits: DecodeLimits,
    probe: &mut LegacyProbe,
) -> Result<(), Error> {
    jpeg2000::decode_with_reduction(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: probe,
        },
        reduce,
        limits,
    )?;
    Ok(())
}
pub fn source_pixels(data: &[u8]) -> usize {
    let p = data.windows(2).position(|s| s == [255, 81]).unwrap();
    let read = |n| u32::from_be_bytes(data[p + n..p + n + 4].try_into().unwrap()) as usize;
    (read(6) - read(14))
        .checked_mul(read(10) - read(18))
        .unwrap()
}
pub fn legacy_contracts(data: &[u8], reduce: u8) {
    let image = legacy(data, reduce, DecodeLimits::default()).unwrap();
    let expected = image.buffer.unwrap();
    for abort in [0, 1, 2, 3] {
        let mut p = LegacyProbe {
            abort,
            ..Default::default()
        };
        legacy_probe(data, reduce, DecodeLimits::default(), &mut p).unwrap();
        assert_eq!(
            p.events,
            if abort == 1 {
                vec![1, 3]
            } else {
                vec![1, 2, 3]
            }
        );
        if abort != 1 {
            assert_eq!(p.image.buffer.unwrap(), expected);
        }
    }
    for fail in [1, 2, 3] {
        let mut p = LegacyProbe {
            fail,
            ..Default::default()
        };
        assert!(
            legacy_probe(data, reduce, DecodeLimits::default(), &mut p)
                .unwrap_err()
                .to_string()
                .contains("reduced callback error")
        );
        assert_eq!(p.events, (1..=fail).collect::<Vec<_>>());
    }
    for field in 0..6 {
        let mut limits = DecodeLimits::default();
        match field {
            0 => limits.input_bytes = data.len() - 1,
            1 => limits.pixels = source_pixels(data) - 1,
            2 => limits.expanded_bytes = expected.len() - 1,
            3 => limits.frames = 0,
            4 => limits.metadata_bytes = 0,
            _ => limits.animation_bytes = expected.len() - 1,
        }
        let mut p = LegacyProbe::default();
        assert!(
            legacy_probe(data, reduce, limits, &mut p).is_err(),
            "legacy limit{field}"
        );
        assert!(p.events.is_empty(), "legacy limit{field}");
    }
    let (mut lo, mut hi) = (0, 8 * 1024 * 1024);
    assert!(
        legacy(
            data,
            reduce,
            DecodeLimits {
                expanded_bytes: hi,
                ..Default::default()
            }
        )
        .is_ok()
    );
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mut p = LegacyProbe::default();
        if legacy_probe(
            data,
            reduce,
            DecodeLimits {
                expanded_bytes: mid,
                ..Default::default()
            },
            &mut p,
        )
        .is_ok()
        {
            hi = mid
        } else {
            assert!(p.events.is_empty());
            lo = mid + 1;
        }
    }
    assert!(lo > 0);
    assert!(
        legacy(
            data,
            reduce,
            DecodeLimits {
                expanded_bytes: lo,
                ..Default::default()
            }
        )
        .is_ok()
    );
    let mut p = LegacyProbe::default();
    assert!(
        legacy_probe(
            data,
            reduce,
            DecodeLimits {
                expanded_bytes: lo - 1,
                ..Default::default()
            },
            &mut p
        )
        .is_err()
    );
    assert!(p.events.is_empty());
    assert_eq!(
        legacy(data, reduce, DecodeLimits::default())
            .unwrap()
            .buffer
            .unwrap(),
        expected
    );
}

#[cfg(feature = "image-buffer-ex")]
pub fn ex_contracts(data: &[u8], reduce: u8, bits: u8, nc: usize) {
    let fmt = format(bits, nc);
    let image = jpeg2000::image_from_ex_with_reduction(
        data,
        fmt,
        PrecisionConversion::Exact,
        reduce,
        DecodeLimits::default(),
    )
    .unwrap();
    let expected = image.bytes().unwrap().to_vec();
    for abort in [0, 1, 2, 3] {
        let mut p = old_ex::Probe::with_format(fmt);
        p.abort = abort;
        jpeg2000::decode_ex_with_reduction(
            &mut BytesReader::new(data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut p,
            },
            reduce,
            DecodeLimits::default(),
        )
        .unwrap();
        assert_eq!(
            p.events,
            if abort == 1 {
                vec![1, 3]
            } else {
                vec![1, 2, 3]
            }
        );
        if abort != 1 {
            assert_eq!(p.image.bytes().unwrap(), expected);
        }
    }
    for fail in [1, 2, 3] {
        let mut p = old_ex::Probe::with_format(fmt);
        p.fail = fail;
        assert!(
            jpeg2000::decode_ex_with_reduction(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut p
                },
                reduce,
                DecodeLimits::default()
            )
            .unwrap_err()
            .to_string()
            .contains("probe callback error")
        );
        assert_eq!(p.events, (1..=fail).collect::<Vec<_>>());
    }
    for field in 0..6 {
        let mut limits = DecodeLimits::default();
        match field {
            0 => limits.input_bytes = data.len() - 1,
            1 => limits.pixels = source_pixels(data) - 1,
            2 => limits.expanded_bytes = expected.len() - 1,
            3 => limits.frames = 0,
            4 => limits.metadata_bytes = 0,
            _ => limits.animation_bytes = expected.len() - 1,
        }
        let mut p = old_ex::Probe::with_format(fmt);
        assert!(
            jpeg2000::decode_ex_with_reduction(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut p
                },
                reduce,
                limits
            )
            .is_err(),
            "EX limit{field}"
        );
        assert!(p.events.is_empty(), "EX limit{field}");
        assert!(
            jpeg2000::image_from_ex_with_reduction(
                data,
                fmt,
                PrecisionConversion::Exact,
                reduce,
                limits
            )
            .is_err()
        );
    }
    let (mut lo, mut hi) = (0, 8 * 1024 * 1024);
    let accepted = |budget| {
        jpeg2000::image_from_ex_with_reduction(
            data,
            fmt,
            PrecisionConversion::Exact,
            reduce,
            DecodeLimits {
                expanded_bytes: budget,
                ..Default::default()
            },
        )
        .is_ok()
    };
    assert!(accepted(hi));
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if accepted(mid) {
            hi = mid
        } else {
            lo = mid + 1;
        }
    }
    assert!(lo > 0 && accepted(lo) && !accepted(lo - 1));
    let mut p = old_ex::Probe::with_format(fmt);
    assert!(
        jpeg2000::decode_ex_with_reduction(
            &mut BytesReader::new(data),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut p
            },
            reduce,
            DecodeLimits {
                expanded_bytes: lo - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(p.events.is_empty());
    for storage in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba16,
    ] {
        for policy in [PrecisionConversion::Exact, PrecisionConversion::Round] {
            let a = jpeg2000::image_from_ex_with_reduction(
                data,
                storage,
                policy,
                reduce,
                DecodeLimits::default(),
            );
            let b = ex_callback(data, storage, policy, reduce, DecodeLimits::default());
            match (a, b) {
                (Ok(a), Ok(b)) => {
                    assert_eq!(a.bytes(), b.bytes());
                    assert_eq!(a.source_format(), Some(fmt));
                    assert_eq!(a.source_stride(), None);
                    assert_eq!(a.precision_conversion(), policy);
                    assert_eq!(a.stride(), b.stride());
                }
                (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
                _ => panic!("owned/callback conversion acceptance differs"),
            }
        }
    }
    let mut image = jpeg2000::image_from_ex_with_reduction(
        data,
        fmt,
        PrecisionConversion::Exact,
        reduce,
        DecodeLimits::default(),
    )
    .unwrap();
    let row = image.width() * if nc == 1 { 2 } else { 8 };
    let stride = row + 3;
    let h = image.height();
    let picked = image
        .encode_pick_ex(
            0,
            0,
            image.width(),
            h,
            PixelOutputEx {
                format: fmt,
                conversion: PrecisionConversion::Exact,
                stride: Some(stride),
            },
            None,
        )
        .unwrap()
        .unwrap();
    for y in 0..h {
        assert_eq!(
            &picked.bytes()[y * stride..y * stride + row],
            &expected[y * row..(y + 1) * row]
        );
        assert_eq!(&picked.bytes()[y * stride + row..(y + 1) * stride], &[0; 3]);
    }
}

pub fn rejects_before_callbacks(data: &[u8], reduce: u8, bits: u8) {
    if bits == 8 {
        let mut p = LegacyProbe::default();
        assert!(legacy_probe(data, reduce, DecodeLimits::default(), &mut p).is_err());
        assert!(p.events.is_empty());
    }
    #[cfg(not(feature = "image-buffer-ex"))]
    if bits != 8 {
        let mut p = LegacyProbe::default();
        assert!(legacy_probe(data, reduce, DecodeLimits::default(), &mut p).is_err());
        assert!(p.events.is_empty());
    }
    #[cfg(feature = "image-buffer-ex")]
    if bits != 8 {
        let mut p = old_ex::Probe::with_format(PixelFormatEx::Rgba16);
        assert!(
            jpeg2000::decode_ex_with_reduction(
                &mut BytesReader::new(data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut p
                },
                reduce,
                DecodeLimits::default()
            )
            .is_err()
        );
        assert!(p.events.is_empty());
        assert!(
            jpeg2000::image_from_ex_with_reduction(
                data,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Round,
                reduce,
                DecodeLimits::default()
            )
            .is_err()
        );
    }
}
pub fn shifted_origins(data: &[u8], reduce: u8, bits: u8, nc: usize) {
    let p = data.windows(2).position(|s| s == [255, 81]).unwrap();
    for delta in [0x80000000u32, 0xffffff80] {
        let mut shifted = data.to_vec();
        for at in [6, 10, 14, 18, 30, 34] {
            let n = u32::from_be_bytes(data[p + at..p + at + 4].try_into().unwrap());
            shifted[p + at..p + at + 4]
                .copy_from_slice(&n.checked_add(delta).unwrap().to_be_bytes());
        }
        if bits == 8 {
            assert_eq!(
                legacy(data, reduce, DecodeLimits::default())
                    .unwrap()
                    .buffer,
                legacy(&shifted, reduce, DecodeLimits::default())
                    .unwrap()
                    .buffer
            );
        }
        #[cfg(feature = "image-buffer-ex")]
        if bits != 8 {
            let fmt = format(bits, nc);
            let decode = |v: &[u8]| {
                jpeg2000::image_from_ex_with_reduction(
                    v,
                    fmt,
                    PrecisionConversion::Exact,
                    reduce,
                    DecodeLimits::default(),
                )
                .unwrap()
            };
            assert_eq!(decode(data).bytes(), decode(&shifted).bytes());
        }
    }
    let _ = nc;
}

pub struct PacketPositions {
    pub eph: usize,
    pub payload_start: usize,
    pub payload_end: usize,
    pub plt: usize,
    pub tlm_length: usize,
    pub sot_length: usize,
}
pub fn packet_structure(data: &[u8], positions: &PacketPositions) {
    let expected = legacy(data, 1, DecodeLimits::default())
        .unwrap()
        .buffer
        .unwrap();
    assert!(positions.payload_start < positions.payload_end);
    assert!(
        data[positions.payload_start..positions.payload_end]
            .iter()
            .any(|&v| v != 255)
    );
    // This data belongs only to a discarded resolution. Its entropy is outside
    // the new API guarantee, while its original length/header state remains.
    let mut entropy = data.to_vec();
    entropy[positions.payload_start..positions.payload_end].fill(255);
    assert_eq!(
        legacy(&entropy, 1, DecodeLimits::default())
            .unwrap()
            .buffer
            .unwrap(),
        expected
    );
    let mut eph = data.to_vec();
    assert_eq!(&eph[positions.eph..positions.eph + 2], &[255, 146]);
    eph[positions.eph + 1] = 147;
    rejects_before_callbacks(&eph, 1, 8);
    let mut plt = data.to_vec();
    plt[positions.plt] ^= 1;
    rejects_before_callbacks(&plt, 1, 8);
    let mut tlm = data.to_vec();
    let value = u32::from_be_bytes(
        tlm[positions.tlm_length..positions.tlm_length + 4]
            .try_into()
            .unwrap(),
    );
    tlm[positions.tlm_length..positions.tlm_length + 4].copy_from_slice(&(value + 1).to_be_bytes());
    rejects_before_callbacks(&tlm, 1, 8);
    // Keep TLM/SOT consistent with the shortened physical part, but leave the
    // discarded packet's declared contribution length unchanged.
    let mut truncated = data.to_vec();
    truncated.remove(positions.payload_end - 1);
    for at in [positions.tlm_length, positions.sot_length] {
        let value = u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
        truncated[at..at + 4].copy_from_slice(&(value - 1).to_be_bytes());
    }
    rejects_before_callbacks(&truncated, 1, 8);
}
pub fn deferred_profiles(ppt: &[u8], mct: &[u8], empty: &[u8]) {
    let error = legacy(ppt, 1, DecodeLimits::default()).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("multi-tile-part PPT reduction is deferred")
    );
    rejects_before_callbacks(ppt, 1, 8);
    let error = legacy(mct, 1, DecodeLimits::default()).err().unwrap();
    assert!(
        error
            .to_string()
            .contains("equal colour component DWT levels")
    );
    rejects_before_callbacks(mct, 1, 8);
    legacy_zero(empty);
    let error = legacy(empty, 1, DecodeLimits::default()).err().unwrap();
    assert!(error.to_string().contains("empty image"));
    rejects_before_callbacks(empty, 1, 8);
}
pub fn unlimited_geometry_overflow(data: &[u8], bits: u8, nc: usize) {
    let siz = data.windows(2).position(|v| v == [255, 81]).unwrap();
    let mut huge = data.to_vec();
    for at in [6, 10] {
        huge[siz + at..siz + at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    }
    // Original source geometry remains checked even when reduction could make
    // its output small; unlimited budgets never disable checked arithmetic.
    for reduce in [1, 32] {
        if bits == 8 {
            assert!(legacy(&huge, reduce, DecodeLimits::unlimited()).is_err());
        }
        #[cfg(not(feature = "image-buffer-ex"))]
        if bits != 8 {
            assert!(legacy(&huge, reduce, DecodeLimits::unlimited()).is_err());
        }
        #[cfg(feature = "image-buffer-ex")]
        if bits != 8 {
            assert!(
                jpeg2000::image_from_ex_with_reduction(
                    &huge,
                    format(bits, nc),
                    PrecisionConversion::Exact,
                    reduce,
                    DecodeLimits::unlimited()
                )
                .is_err()
            );
        }
    }
    let _ = nc;
}
