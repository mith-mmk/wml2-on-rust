//! Run identical synthetic benchmarks on the base and modified revisions.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
use wml2::draw::*;
struct Meter;
static COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn allocated(n: usize) {
    COUNT.fetch_add(1, Relaxed);
    TOTAL.fetch_add(n, Relaxed);
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            allocated(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            allocated(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), Relaxed);
            allocated(n);
        }
        q
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;
#[cfg(any(
    feature = "jpeg",
    feature = "webp",
    feature = "png",
    feature = "tiff",
    feature = "avifenc",
    feature = "image-buffer-ex"
))]
fn hash(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
fn measure(name: &str, i: usize, body: impl FnOnce() -> (u64, u128)) {
    let live = LIVE.load(Relaxed);
    PEAK.store(live, Relaxed);
    COUNT.store(0, Relaxed);
    TOTAL.store(0, Relaxed);
    let start = Instant::now();
    let (digest, first) = body();
    let ns = start.elapsed().as_nanos();
    let count = COUNT.load(Relaxed);
    let total = TOTAL.load(Relaxed);
    let peak = PEAK.load(Relaxed).saturating_sub(live);
    println!("{name},{i},{ns},{count},{total},{peak},{first},{digest}");
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "draw".into());
    let repeat = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    println!("case,iteration,ns,allocations,allocated_bytes,peak_live_bytes,first_draw_ns,hash");
    #[cfg(feature = "avifenc")]
    if mode == "avif-encode" {
        let controls = std::collections::HashMap::from([(
            "lossless".into(),
            wml2::metadata::DataMap::UInt(1),
        )]);
        for i in 0..repeat {
            measure("avif-encode-2x2", i, || {
                let mut image = ImageBuffer::from_buffer(
                    2,
                    2,
                    vec![
                        0, 17, 255, 255, 85, 136, 17, 0, 170, 34, 221, 255, 255, 238, 204, 119,
                    ],
                );
                let encoded = image_to(
                    &mut image,
                    wml2::util::ImageFormat::Avif,
                    Some(controls.clone()),
                )
                .unwrap();
                (hash(&encoded), 0)
            });
        }
    }
    #[cfg(feature = "image-buffer-ex")]
    if mode == "decode-ex-file" {
        let encoded = std::fs::read(
            std::env::args()
                .nth(3)
                .expect("decode-ex-file <repeat> <path>"),
        )
        .unwrap();
        for i in 0..repeat {
            measure("decode-ex-file", i, || {
                let image =
                    image_from_ex(&encoded, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                        .unwrap();
                (hash(image.bytes().unwrap()), 0)
            });
        }
    }
    #[cfg(feature = "image-buffer-ex")]
    if mode == "ex-view" {
        let (width, height) = (1024, 768);
        let pixels: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 997 + 1) as u16,
                    (i * 337 + 129) as u16,
                    (i * 503 + 32768) as u16,
                    65535,
                ]
                .into_iter()
                .flat_map(u16::to_le_bytes)
            })
            .collect();
        let image =
            ImageBufferEx::from_bytes(width, height, PixelFormatEx::Rgba16, None, pixels).unwrap();
        for i in 0..repeat {
            measure("ex-copy-then-pick-row", i, || {
                let mut legacy = image.to_rgba8(PrecisionConversion::Round).unwrap();
                let bytes = legacy.encode_pick(0, 0, width, 1, None).unwrap().unwrap();
                (hash(&bytes), 0)
            });
            measure("ex-borrowed-pick-row", i, || {
                let bytes = image
                    .legacy_picker(PrecisionConversion::Round)
                    .encode_pick(0, 0, width, 1, None)
                    .unwrap()
                    .unwrap();
                (hash(&bytes), 0)
            });
        }
    }
    #[cfg(any(feature = "png", feature = "tiff"))]
    if mode == "decode-file" {
        let path = std::env::args()
            .nth(3)
            .expect("decode-file <repeat> <path>");
        let encoded = std::fs::read(path).unwrap();
        for i in 0..repeat {
            measure("decode-file", i, || {
                let mut sink = Sink {
                    image: ImageBuffer::new(),
                    start: Instant::now(),
                    first: 0,
                };
                image_loader(
                    &encoded,
                    &mut DecodeOptions {
                        debug_flag: 0,
                        drawer: &mut sink,
                    },
                )
                .unwrap();
                (hash(sink.image.buffer.as_ref().unwrap()), sink.first)
            });
        }
    }
    if mode == "draw" {
        let mut image = ImageBuffer::from_buffer(3840, 2160, vec![0; 3840 * 2160 * 4]);
        let pixels: Vec<u8> = (0..3840 * 2160 * 4).map(|i| (i * 17) as u8).collect();
        for i in 0..repeat {
            measure("draw-4k", i, || {
                image.draw(0, 0, 3840, 2160, &pixels, None).unwrap();
                (image.buffer.as_ref().unwrap()[998] as u64, 0)
            });
            measure("pick-4k", i, || {
                let v = image.encode_pick(0, 0, 3840, 2160, None).unwrap().unwrap();
                (std::hint::black_box(v[998]) as u64, 0)
            });
            measure("draw-small-10000", i, || {
                for n in 0..10000 {
                    image
                        .draw(n % 3800, n % 2100, 16, 16, &pixels[..1024], None)
                        .unwrap();
                }
                (image.buffer.as_ref().unwrap()[998] as u64, 0)
            });
        }
    }
    #[cfg(feature = "jpeg")]
    if mode == "jpeg" {
        for (width, height) in [(16, 16), (1024, 768)] {
            let mut source = ImageBuffer::from_buffer(
                width,
                height,
                (0..width * height)
                    .flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 31) as u8, 255])
                    .collect(),
            );
            let encoded = image_to(&mut source, wml2::util::ImageFormat::Jpeg, None).unwrap();
            let name = format!("jpeg-{width}x{height}");
            for i in 0..repeat {
                measure(&name, i, || {
                    let mut sink = Sink {
                        image: ImageBuffer::new(),
                        start: Instant::now(),
                        first: 0,
                    };
                    image_loader(
                        &encoded,
                        &mut DecodeOptions {
                            debug_flag: 0,
                            drawer: &mut sink,
                        },
                    )
                    .unwrap();
                    (hash(sink.image.buffer.as_ref().unwrap()), sink.first)
                });
            }
        }
    }
    #[cfg(feature = "webp")]
    if mode == "animation" {
        let mut image = ImageBuffer::new();
        image
            .init(
                128,
                128,
                Some(InitOptions {
                    background: None,
                    animation: true,
                    loop_count: 0,
                }),
            )
            .unwrap();
        for n in 0..24 {
            image.next(Some(NextOptions::wait(20))).unwrap();
            image
                .draw(0, 0, 128, 128, &vec![n * 7; 128 * 128 * 4], None)
                .unwrap();
        }
        for i in 0..repeat {
            measure("webp-animation-24", i, || {
                (
                    hash(&image_to(&mut image, wml2::util::ImageFormat::Webp, None).unwrap()),
                    0,
                )
            });
        }
    }
    #[cfg(feature = "color-management")]
    if mode == "icc" {
        icc_bench(repeat);
    }
}
#[cfg(any(feature = "jpeg", feature = "png", feature = "tiff"))]
struct Sink {
    image: ImageBuffer,
    start: Instant,
    first: u128,
}
#[cfg(any(feature = "jpeg", feature = "png", feature = "tiff"))]
impl DrawCallback for Sink {
    fn init(&mut self, w: usize, h: usize, o: Option<InitOptions>) -> Response {
        self.image.init(w, h, o)
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
        if self.first == 0 {
            self.first = self.start.elapsed().as_nanos();
        }
        self.image.draw(x, y, w, h, p, o)
    }
    fn next(&mut self, o: Option<NextOptions>) -> Response {
        self.image.next(o)
    }
    fn terminate(&mut self, o: Option<TerminateOptions>) -> Response {
        self.image.terminate(o)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
    fn set_metadata(&mut self, k: &str, v: wml2::metadata::DataMap) -> Response {
        self.image.set_metadata(k, v)
    }
}
#[cfg(feature = "color-management")]
fn icc_bench(repeat: usize) {
    use wml2::highres::*;
    let mut profile = vec![0u8; 188];
    profile[..4].copy_from_slice(&188u32.to_be_bytes());
    profile[8] = 4;
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"GRAY");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[128..132].copy_from_slice(&2u32.to_be_bytes());
    profile[132..136].copy_from_slice(b"kTRC");
    profile[136..140].copy_from_slice(&156u32.to_be_bytes());
    profile[140..144].copy_from_slice(&12u32.to_be_bytes());
    profile[144..148].copy_from_slice(b"wtpt");
    profile[148..152].copy_from_slice(&168u32.to_be_bytes());
    profile[152..156].copy_from_slice(&20u32.to_be_bytes());
    profile[156..160].copy_from_slice(b"curv");
    profile[168..172].copy_from_slice(b"XYZ ");
    for (i, v) in [0.9642f32, 1.0, 0.8249].iter().enumerate() {
        profile[176 + i * 4..180 + i * 4]
            .copy_from_slice(&((*v * 65536.).round() as u32).to_be_bytes());
    }
    let descriptor = ImageDescriptor::gray(1024, 768, 16).unwrap();
    let pixels = PixelBuffer::u16(vec![
        Plane::new(
            descriptor.planes()[0].layout().clone(),
            vec![32768; 1024 * 768],
        )
        .unwrap(),
    ])
    .unwrap();
    let frame = ImageFrame::new(descriptor, pixels).unwrap();
    for i in 0..repeat {
        measure("icc-u16-gray", i, || {
            let result = icc::transform_frame(&frame, &profile, &profile).unwrap();
            (
                result.pixels().u16_planes().unwrap()[0].samples()[99] as u64,
                0,
            )
        });
    }
}
