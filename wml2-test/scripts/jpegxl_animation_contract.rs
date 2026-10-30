//! Callback-transport experiment using independently decoded fixture frames.
//! This does not decode JPEG XL or provide production animation admission.
use wml2::draw::*;
use wml2::limits::DecodeLimits;
type Error = Box<dyn std::error::Error>;
fn exact_ms(ticks: u32, numerator: u32, denominator: u32) -> Result<u64, Error> {
    if numerator == 0 || denominator == 0 {
        return Err("invalid timebase".into());
    }
    let product = u128::from(ticks) * 1000 * u128::from(denominator);
    if product % u128::from(numerator) != 0 {
        return Err("fractional milliseconds".into());
    }
    Ok(u64::try_from(product / u128::from(numerator))?)
}
fn retained_bytes(w: usize, h: usize, n: usize) -> Result<usize, Error> {
    Ok(n.checked_add(1)
        .and_then(|n| w.checked_mul(h)?.checked_mul(4)?.checked_mul(n))
        .ok_or("callback storage overflow")?)
}
struct AbortProbe {
    target: usize,
    nexts: usize,
    draws: usize,
    terminations: usize,
}
impl DrawCallback for AbortProbe {
    fn init(
        &mut self,
        _: usize,
        _: usize,
        _: Option<InitOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        Ok(None)
    }
    fn draw(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &[u8],
        _: Option<DrawOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.draws += 1;
        Ok(None)
    }
    fn next(&mut self, _: Option<NextOptions>) -> Result<Option<CallbackResponse>, Error> {
        self.nexts += 1;
        Ok(Some(if self.nexts == self.target {
            CallbackResponse::abort()
        } else {
            CallbackResponse::cont()
        }))
    }
    fn terminate(
        &mut self,
        _: Option<TerminateOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        self.terminations += 1;
        Ok(None)
    }
    fn verbose(
        &mut self,
        _: &str,
        _: Option<VerboseOptions>,
    ) -> Result<Option<CallbackResponse>, Error> {
        Ok(None)
    }
    fn set_metadata(
        &mut self,
        _: &str,
        _: wml2::metadata::DataMap,
    ) -> Result<Option<CallbackResponse>, Error> {
        Ok(None)
    }
}
fn abort_model() -> Result<(), Error> {
    for target in 1..=3 {
        let mut sink = AbortProbe {
            target,
            nexts: 0,
            draws: 0,
            terminations: 0,
        };
        sink.init(
            1,
            1,
            Some(InitOptions {
                loop_count: 1,
                background: None,
                animation: true,
            }),
        )?;
        for i in 0..3 {
            if i == 0 {
                sink.draw(0, 0, 1, 1, &[0; 4], None)?;
            }
            let r = sink.next(Some(NextOptions::wait(10)))?;
            if r.is_some_and(|r| r.response == ResponseCommand::Abort) {
                break;
            }
            sink.draw(0, 0, 1, 1, &[0; 4], None)?;
        }
        sink.terminate(None)?;
        assert_eq!(
            (sink.nexts, sink.draws, sink.terminations),
            (target, target, 1)
        );
    }
    Ok(())
}
fn main() -> Result<(), Error> {
    abort_model()?;
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(exact_ms(5, 100, 1)?, 50);
    assert_eq!(exact_ms(u32::MAX, 1, 1024)?, 4398046510080000);
    assert!(exact_ms(5, 30000, 1001).is_err());
    assert!(exact_ms(1, 0, 1).is_err());
    assert!(exact_ms(1, 1, 0).is_err());
    assert!(exact_ms(u32::MAX, 1, u32::MAX).is_err());
    assert!(retained_bytes(usize::MAX, 1, 1).is_err());
    if args[1] == "--timing" {
        let n = args[2].parse()?;
        let d = args[3].parse()?;
        let ms: Vec<_> = args[4]
            .split(',')
            .map(|v| exact_ms(v.parse().unwrap(), n, d))
            .collect::<Result<_, _>>()?;
        println!(
            "timing {}",
            ms.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        return Ok(());
    }
    let w: usize = args[1].parse()?;
    let h: usize = args[2].parse()?;
    let loops: u32 = args[3].parse()?;
    let waits: Vec<u64> = args[4]
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    if w > 64 || h > 64 || waits.len() > 16 {
        return Err("small trusted transport only".into());
    }
    let mut image = ImageBuffer::new();
    image.init(
        w,
        h,
        Some(InitOptions {
            loop_count: loops,
            background: None,
            animation: true,
        }),
    )?;
    let mut expected = Vec::new();
    for (i, &wait) in waits.iter().enumerate() {
        let raw = std::fs::read(format!("{}.{}.rgba16le", args[5], i))?;
        let data: Vec<u8> = raw
            .chunks_exact(2)
            .map(|b| ((u32::from(u16::from_le_bytes([b[0], b[1]])) * 255 + 32767) / 65535) as u8)
            .collect();
        if data.len() != w * h * 4 {
            return Err("oracle layout".into());
        }
        if i == 0 {
            image.draw(0, 0, w, h, &data, None)?;
        }
        image.next(Some(NextOptions {
            flag: if i == 0 {
                NextOption::Continue
            } else {
                NextOption::Next
            },
            await_time: wait,
            image_rect: Some(ImageRect {
                start_x: 0,
                start_y: 0,
                width: w,
                height: h,
            }),
            dispose_option: Some(NextDispose::None),
            blend: Some(NextBlend::Override),
        }))?;
        image.draw(0, 0, w, h, &data, None)?;
        expected.push(data);
    }
    image.terminate(None)?;
    let animation = image.animation.as_ref().ok_or("animation missing")?;
    assert_eq!(animation.len(), waits.len());
    assert_eq!(image.buffer.as_ref().unwrap(), &expected[0]);
    assert_eq!(image.loop_count, Some(loops));
    for ((layer, expected), &wait) in animation.iter().zip(&expected).zip(&waits) {
        assert_eq!(&layer.buffer, expected);
        assert_eq!(layer.control.await_time, wait);
        assert!(matches!(layer.control.blend, Some(NextBlend::Override)));
    }
    let bytes = image.buffer.as_ref().unwrap().len()
        + animation.iter().map(|f| f.buffer.len()).sum::<usize>();
    assert_eq!(bytes, retained_bytes(w, h, animation.len())?);
    let mut ex = ImageBufferEx::new();
    assert!(
        ex.init_ex(
            w,
            h,
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Rgba8,
                source_stride: None,
                legacy: Some(InitOptions {
                    loop_count: loops,
                    background: None,
                    animation: true
                })
            })
        )
        .is_err()
    );
    // Published APIs must continue to reject animation rather than return one frame.
    let source = std::fs::read(&args[6])?;
    for limits in [DecodeLimits::default(), DecodeLimits::unlimited()] {
        assert!(image_from_with_limits(&source, limits).is_err());
        assert!(
            image_from_ex_with_limits(
                &source,
                PixelFormatEx::Rgba8,
                PrecisionConversion::Exact,
                limits
            )
            .is_err()
        );
        assert!(wml2::jpegxl::image_from_srgb_with_limits(&source, limits).is_err());
    }
    println!(
        "transport {} {} {} {} {}",
        animation.len(),
        bytes,
        image.first_wait_time.unwrap(),
        loops,
        ex.bytes().is_none()
    );
    Ok(())
}
