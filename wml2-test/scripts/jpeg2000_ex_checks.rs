//! Shared native/WASM checks for the narrow JPEG2000 EX profile.
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
type Error = Box<dyn std::error::Error>;

pub fn format(bits: u8) -> PixelFormatEx {
    match bits {
        12 => PixelFormatEx::Gray12,
        16 => PixelFormatEx::Gray16,
        _ => panic!("test depth"),
    }
}

pub fn pixels(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    let fmt = format(bits);
    let mut image = image_from_ex(data, fmt, PrecisionConversion::Exact).unwrap();
    assert_eq!(
        (image.width(), image.height(), image.stride()),
        (w, h, w * 2)
    );
    assert_eq!(image.source_format(), Some(fmt));
    assert_eq!(image.source_stride(), None);
    assert_eq!(image.bytes().unwrap(), expected);
    // Odd byte strides must not be interpreted as numeric U16 strides.
    let output = PixelOutputEx {
        format: fmt,
        conversion: PrecisionConversion::Exact,
        stride: Some(w * 2 + 1),
    };
    let picked = image
        .encode_pick_ex(0, 0, w, h, output, None)
        .unwrap()
        .unwrap();
    for y in 0..h {
        let row = &picked.bytes()[y * (w * 2 + 1)..(y + 1) * (w * 2 + 1)];
        assert_eq!(&row[..w * 2], &expected[y * w * 2..(y + 1) * w * 2]);
        assert_eq!(row[w * 2], 0);
    }
    assert_eq!(image.bytes().unwrap(), expected);
    let rgba_format = if bits == 12 {
        PixelFormatEx::Rgba12
    } else {
        PixelFormatEx::Rgba16
    };
    let rgba = image_from_ex(data, rgba_format, PrecisionConversion::Exact).unwrap();
    let max = if bits == 12 { 4095u16 } else { 65535 };
    for (pixel, gray) in rgba
        .bytes()
        .unwrap()
        .chunks_exact(8)
        .zip(expected.chunks_exact(2))
    {
        assert_eq!(&pixel[..2], gray);
        assert_eq!(&pixel[2..4], gray);
        assert_eq!(&pixel[4..6], gray);
        assert_eq!(&pixel[6..], max.to_le_bytes());
    }
    // The old API does not silently start accepting high-depth JPEG2000.
    let Err(error) = image_load(data) else {
        panic!("legacy API accepted high depth")
    };
    assert!(error.to_string().contains("high sample depth"));
}

pub struct Probe {
    pub image: ImageBufferEx,
    pub events: Vec<u8>,
    pub abort: u8,
    pub fail: u8,
    bits: u8,
}
impl Probe {
    pub fn new(bits: u8) -> Self {
        Self {
            image: ImageBufferEx::with_storage(format(bits), PrecisionConversion::Exact),
            events: Vec::new(),
            abort: 0,
            fail: 0,
            bits,
        }
    }
    fn event(&mut self, id: u8) -> Response {
        self.events.push(id);
        if self.fail == id {
            return Err("probe callback error".into());
        }
        Ok((self.abort == id).then(CallbackResponse::abort))
    }
}
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        panic!("must declare Gray depth")
    }
    fn init_ex(&mut self, w: usize, h: usize, o: Option<InitOptionsEx>) -> Response {
        let declaration = o.as_ref().unwrap();
        assert_eq!(declaration.source_format, format(self.bits));
        assert_eq!(declaration.source_stride, None);
        self.image.init_ex(w, h, o)?;
        self.event(1)
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        data: &[u8],
        o: Option<DrawOptions>,
    ) -> Response {
        self.image.draw(x, y, w, h, data, o)?;
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
        panic!("no inferred metadata/profile")
    }
}

pub fn decode(data: &[u8], probe: &mut Probe, limits: DecodeLimits) -> Result<(), Error> {
    image_decoder_ex_with_limits(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: probe,
        },
        limits,
    )?;
    Ok(())
}

pub fn callbacks_and_limits(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    for abort in [0, 1, 2, 3] {
        let mut p = Probe::new(bits);
        p.abort = abort;
        decode(data, &mut p, DecodeLimits::default()).unwrap();
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
        let mut p = Probe::new(bits);
        p.fail = fail;
        assert!(
            decode(data, &mut p, DecodeLimits::default())
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
            1 => limits.pixels = w * h - 1,
            2 => limits.expanded_bytes = w * h * 2 - 1,
            3 => limits.frames = 0,
            4 => limits.metadata_bytes = 0, // OpenJPEG COM is charged.
            _ => limits.animation_bytes = w * h * 2 - 1,
        }
        let mut p = Probe::new(bits);
        assert!(decode(data, &mut p, limits).is_err(), "budget {field}");
        assert!(p.events.is_empty(), "budget {field}");
    }
    // Discover the actual combined workspace threshold, then exercise exact
    // and one-short boundaries. This is independent of its implementation formula.
    let (mut lo, mut hi) = (0, 4 * 1024 * 1024);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mut p = Probe::new(bits);
        let limits = DecodeLimits {
            expanded_bytes: mid,
            ..Default::default()
        };
        if decode(data, &mut p, limits).is_ok() {
            hi = mid;
        } else {
            assert!(p.events.is_empty());
            lo = mid + 1;
        }
    }
    assert!(lo > expected.len());
    let mut p = Probe::new(bits);
    decode(
        data,
        &mut p,
        DecodeLimits {
            expanded_bytes: lo,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.image.bytes().unwrap(), expected);
    let mut p = Probe::new(bits);
    assert!(
        decode(
            data,
            &mut p,
            DecodeLimits {
                expanded_bytes: lo - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(p.events.is_empty());
    pixels(data, expected, w, h, bits); // Error scopes restore the default budgets.
}

pub fn geometry_boundaries(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    let siz = data.windows(2).position(|v| v == [255, 0x51]).unwrap();
    let mut huge_canvas = data.to_vec();
    for offset in [siz + 6, siz + 10] {
        huge_canvas[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    }
    let mut p = Probe::new(bits);
    assert!(decode(&huge_canvas, &mut p, DecodeLimits::unlimited()).is_err());
    assert!(p.events.is_empty());
    // This remains valid on 32-bit: bounds clip before allocating tile samples.
    for offset in [siz + 22, siz + 26] {
        let mut clipped = data.to_vec();
        clipped[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        pixels(&clipped, expected, w, h, bits);
    }
}
