//! Development-only core comparison. Never use this probe as WML2 admission.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;
type Error = Box<dyn std::error::Error>;
struct Meter;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
fn allocated(n: usize) {
    TOTAL.fetch_add(n, Relaxed);
    let live = LIVE.fetch_add(n, Relaxed) + n;
    PEAK.fetch_max(live, Relaxed);
}
// SAFETY: forwards the exact allocation/deallocation layouts to System. The
// counters do not allocate, and failed allocations do not change live bytes.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies the GlobalAlloc layout contract.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: same layout contract; System initializes the complete span.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            allocated(layout.size());
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        // SAFETY: forwards the caller's original pointer and layout.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: forwards the caller's valid old allocation and requested size.
        let next = unsafe { System.realloc(ptr, layout, size) };
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            allocated(size);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Meter = Meter;
struct Image {
    size: (usize, usize),
    pixels: Vec<u16>,
    profile: Vec<u8>,
}
fn complete<T, U>(r: jxl07::api::ProcessingResult<T, U>) -> Result<T, Error> {
    match r {
        jxl07::api::ProcessingResult::Complete { result } => Ok(result),
        _ => Err("truncated core input".into()),
    }
}
fn decode07(mut input: &[u8]) -> Result<Image, Error> {
    use jxl07::api::*;
    let mut options = JxlDecoderOptions::default();
    options.high_precision = true;
    options.render_spot_colors = false;
    options.sample_limit = Some(4 * 1024 * 1024 * 4 + 1);
    let mut decoder =
        complete(JxlDecoder::<states::Initialized>::new(options).process(&mut input, None)?)?;
    let size = decoder.basic_info().size;
    if size.0.checked_mul(size.1).ok_or("size overflow")? > 4 * 1024 * 1024 {
        return Err("fixture too large".into());
    }
    decoder.set_pixel_format(JxlPixelFormat::rgba16(
        decoder.basic_info().extra_channels.len(),
    ))?;
    let profile = match decoder.output_color_profile() {
        JxlColorProfile::Icc(b) => b.clone(),
        JxlColorProfile::Simple(e) => e.maybe_create_profile()?.ok_or("no ICC")?,
    };
    let decoder = complete(decoder.process(&mut input, None)?)?;
    let mut pixels = vec![0u16; size.0 * size.1 * 4];
    // SAFETY: initialized u16 storage, native endian format, aligned rows. The
    // byte borrow ends before pixels is read again and contains no invalid bits.
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast(), pixels.len() * 2) };
    let mut buffers = [JxlOutputBuffer::new(bytes, size.1, size.0 * 8)];
    let decoder = complete(decoder.process(&mut input, &mut buffers, None)?)?;
    if decoder.has_more_frames() {
        return Err("not a still fixture".into());
    }
    Ok(Image {
        size,
        pixels,
        profile,
    })
}
fn decode08(mut input: &[u8]) -> Result<Image, Error> {
    use jxl08::api::*;
    let mut options = JxlDecoderOptions::default();
    options.high_precision = true;
    options.adjust_orientation = true;
    options.alpha_mode = AlphaMode::AsIs;
    options.render_spot_colors = false;
    options.sample_limit = Some(4 * 1024 * 1024 * 4 + 1);
    // The probe is restricted to known fixtures. This core option is NOT an
    // equivalent for WML2's pre-allocation ICC/layer/workspace checks.
    options.max_profile_level = ProfileLevel::Main10;
    let mut decoder = JxlDecoder::new(options);
    decoder.close_input();
    if decoder.process(&mut input, None, None)? != Event::BasicInfo {
        return Err("no basic info".into());
    }
    let info = decoder.basic_info().ok_or("missing basic info")?;
    let size = info.size;
    if size.0.checked_mul(size.1).ok_or("size overflow")? > 4 * 1024 * 1024 {
        return Err("fixture too large".into());
    }
    decoder.set_pixel_format(JxlPixelFormat::rgba16(info.extra_channels.len()))?;
    let profile = decoder
        .output_color_profile()
        .ok_or("missing profile")?
        .try_as_icc()
        .ok_or("no ICC")?
        .into_owned();
    let mut pixels = vec![0u16; size.0 * size.1 * 4];
    // SAFETY: same initialized/aligned native-endian storage contract as 0.7.4.
    let bytes =
        unsafe { std::slice::from_raw_parts_mut(pixels.as_mut_ptr().cast(), pixels.len() * 2) };
    let mut buffers = [JxlOutputBuffer::new(bytes, size.1, size.0 * 8)];
    let mut frames = 0;
    loop {
        match decoder.process(&mut input, Some(&mut buffers), None)? {
            Event::FrameHeader => {}
            Event::FrameComplete {
                has_more_frames: false,
            } => {
                frames += 1;
            }
            Event::Complete => break,
            other => return Err(format!("unexpected event {other:?}").into()),
        }
    }
    if frames != 1 {
        return Err("not exactly one frame".into());
    }
    Ok(Image {
        size,
        pixels,
        profile,
    })
}
fn srgb(image: &Image, destination: &[u8]) -> Result<Vec<u8>, Error> {
    use icc_profile::{ParseLimits, Transform, TransformLimits, TransformOptions};
    let transform = Transform::from_bytes_with_limits(
        &image.profile,
        destination,
        TransformOptions::default(),
        ParseLimits::default(),
        TransformLimits::default(),
    )?;
    let channels = transform.input_channels();
    if !matches!(channels, 1 | 3) {
        return Err("unsupported source channels".into());
    }
    let mut result = Vec::with_capacity(image.pixels.len());
    for chunk in image.pixels.chunks(256 * 4) {
        let input: Vec<f32> = chunk
            .chunks_exact(4)
            .flat_map(|p| p[..channels].iter().map(|&v| v as f32 / 65535.0))
            .collect();
        let mut output = vec![0.0; chunk.len() / 4 * 3];
        transform.transform_f32(&input, &mut output)?;
        for (rgb, pixel) in output.chunks_exact(3).zip(chunk.chunks_exact(4)) {
            for &c in rgb {
                let v = (c.clamp(0.0, 1.0) * 65535.0).round() as u32;
                result.push(((v * 255 + 32767) / 65535) as u8);
            }
            result.push(((u32::from(pixel[3]) * 255 + 32767) / 65535) as u8);
        }
    }
    Ok(result)
}
fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let input = std::fs::read(&args[1])?;
    let version = &args[2];
    let n: usize = args[3].parse()?;
    if n == 0 {
        return Err("iterations must be positive".into());
    }
    println!("width,height,ns,allocated_bytes,peak_extra_live_bytes,hash");
    for i in 0..n {
        let base = LIVE.load(Relaxed);
        PEAK.store(base, Relaxed);
        let before = TOTAL.load(Relaxed);
        let start = Instant::now();
        let image = if version == "07" {
            decode07(&input)?
        } else if version == "08" {
            decode08(&input)?
        } else {
            return Err("version must be 07 or 08".into());
        };
        let ns = start.elapsed().as_nanos();
        let total = TOTAL.load(Relaxed) - before;
        let peak = PEAK.load(Relaxed).saturating_sub(base);
        let hash = image
            .pixels
            .iter()
            .fold(0u64, |h, &p| h.wrapping_mul(31).wrapping_add(p as u64));
        println!(
            "{},{},{ns},{total},{peak},{hash}",
            image.size.0, image.size.1
        );
        if i == 0 && args.len() > 4 {
            let bytes = if args.len() > 5 {
                srgb(&image, &std::fs::read(&args[5])?)?
            } else {
                image.pixels.iter().flat_map(|v| v.to_le_bytes()).collect()
            };
            std::fs::write(&args[4], bytes)?;
        }
    }
    Ok(())
}
