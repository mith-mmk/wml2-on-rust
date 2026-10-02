use crate::draw::*;
use crate::limits::{self, DecodeLimits};
use crate::metadata::DataMap;
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
type Error = Box<dyn std::error::Error>;

#[derive(Debug)]
struct Aborted;
impl std::fmt::Display for Aborted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("callback aborted")
    }
}
impl std::error::Error for Aborted {}
fn response(result: Response, handle_abort: bool) -> Response {
    match result? {
        Some(value) if handle_abort && value.response == ResponseCommand::Abort => {
            Err(Box::new(Aborted))
        }
        value => Ok(value),
    }
}

struct Guard<'a> {
    drawer: &'a mut dyn DrawCallback,
    limits: DecodeLimits,
    canvas: (usize, usize),
    frames: usize,
    bytes: usize,
    terminated: bool,
    handle_abort: bool,
    ready: bool,
    stopped: bool,
    #[cfg(feature = "image-buffer-ex")]
    source: Option<(PixelFormatEx, Option<usize>)>,
}
impl Guard<'_> {
    fn accept(&mut self, result: Response) -> Response {
        if matches!(&result, Ok(Some(r)) if r.response == ResponseCommand::Abort) {
            self.stopped = true;
        }
        self.ready = matches!(&result, Ok(None))
            || matches!(&result, Ok(Some(r)) if r.response != ResponseCommand::Abort);
        response(result, self.handle_abort)
    }
    fn forward(&mut self, result: Response) -> Response {
        if matches!(&result, Ok(Some(r)) if r.response == ResponseCommand::Abort) {
            self.ready = false;
            self.stopped = true;
        }
        response(result, self.handle_abort)
    }
    fn size(&self, width: usize, height: usize) -> Result<usize, Error> {
        let pixels = width
            .checked_mul(height)
            .ok_or_else(|| std::io::Error::other("pixel count overflow"))?;
        limits::check(pixels, self.limits.pixels, "pixels")?;
        let bytes = pixels
            .checked_mul(4)
            .ok_or_else(|| std::io::Error::other("image size overflow"))?;
        limits::check(bytes, self.limits.expanded_bytes, "RGBA image")?;
        Ok(bytes)
    }
}
impl DrawCallback for Guard<'_> {
    fn init(&mut self, w: usize, h: usize, option: Option<InitOptions>) -> Response {
        self.ready = false;
        if self.stopped {
            return Err(Box::new(Aborted));
        }
        #[cfg(feature = "image-buffer-ex")]
        {
            self.source = None;
        }
        let bytes = self.size(w, h)?;
        limits::check(bytes, self.limits.animation_bytes, "canvas storage")?;
        let result = self.drawer.init(w, h, option);
        let result = self.accept(result)?;
        if self.ready {
            self.canvas = (w, h);
            self.bytes = bytes;
            self.frames = 0;
            self.terminated = false;
        }
        Ok(result)
    }
    #[cfg(feature = "image-buffer-ex")]
    fn init_ex(&mut self, w: usize, h: usize, option: Option<InitOptionsEx>) -> Response {
        self.ready = false;
        if self.stopped {
            return Err(Box::new(Aborted));
        }
        self.source = None;
        let option = option.unwrap_or_default();
        let pixels = w
            .checked_mul(h)
            .ok_or_else(|| std::io::Error::other("pixel count overflow"))?;
        limits::check(pixels, self.limits.pixels, "EX pixels")?;
        let row = option.source_format.row_bytes(w)?;
        let stride = if let Some(stride) = option.source_stride {
            option.source_format.validate_stride(stride)?;
            // A tile declaration is legal; budget at least a full packed canvas,
            // and include all padding if the declared stride exceeds that row.
            stride.max(row)
        } else {
            row
        };
        if w == 0 || h == 0 {
            return Err(std::io::Error::other("EX dimensions must be nonzero").into());
        }
        let bytes = stride
            .checked_mul(h)
            .ok_or_else(|| std::io::Error::other("EX image size overflow"))?;
        limits::check(bytes, self.limits.expanded_bytes, "EX source canvas")?;
        limits::check(bytes, self.limits.animation_bytes, "EX canvas storage")?;
        let source = (option.source_format, option.source_stride);
        let result = self.drawer.init_ex(w, h, Some(option));
        let result = self.accept(result)?;
        if self.ready {
            self.canvas = (w, h);
            self.bytes = bytes;
            self.frames = 0;
            self.terminated = false;
            self.source = Some(source);
        }
        Ok(result)
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        data: &[u8],
        option: Option<DrawOptions>,
    ) -> Response {
        if !self.ready {
            return Err(std::io::Error::other(
                "draw before successful initialization or after Abort",
            )
            .into());
        }
        #[cfg(feature = "image-buffer-ex")]
        if let Some((format, stride)) = self.source {
            let (stride, bytes) = format.extent(w, h, stride)?;
            let pixels = w
                .checked_mul(h)
                .ok_or_else(|| std::io::Error::other("draw pixel count overflow"))?;
            limits::check(pixels, self.limits.pixels, "EX draw pixels")?;
            limits::check(bytes, self.limits.expanded_bytes, "EX draw bytes")?;
            crate::draw::image_buffer_ex::validate_bytes(data, w, h, format, stride, bytes)?;
            x.checked_add(w)
                .ok_or_else(|| std::io::Error::other("EX draw x overflow"))?;
            y.checked_add(h)
                .ok_or_else(|| std::io::Error::other("EX draw y overflow"))?;
        }
        let result = self.drawer.draw(x, y, w, h, data, option);
        self.forward(result)
    }
    fn next(&mut self, option: Option<NextOptions>) -> Response {
        if !self.ready {
            return Err(std::io::Error::other(
                "next before successful initialization or after Abort",
            )
            .into());
        }
        #[cfg(feature = "image-buffer-ex")]
        if self.source.is_some() {
            return Err(std::io::Error::other("EX animation is unsupported").into());
        }
        let (w, h) = option
            .as_ref()
            .and_then(|o| o.image_rect.as_ref())
            .map_or(self.canvas, |r| (r.width, r.height));
        let bytes = self.size(w, h)?;
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| std::io::Error::other("animation size overflow"))?;
        let frames = self
            .frames
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("frame count overflow"))?;
        limits::check(total, self.limits.animation_bytes, "animation storage")?;
        limits::check(frames, self.limits.frames, "frame count")?;
        let result = self.drawer.next(option);
        let result = self.forward(result)?;
        if !self.ready {
            return Ok(result);
        }
        self.bytes = total;
        self.frames = frames;
        Ok(result)
    }
    fn terminate(&mut self, option: Option<TerminateOptions>) -> Response {
        if self.handle_abort && self.terminated {
            return Ok(None);
        }
        self.terminated = true;
        self.ready = false;
        self.drawer.terminate(option)
    }
    fn verbose(&mut self, text: &str, option: Option<VerboseOptions>) -> Response {
        let result = self.drawer.verbose(text, option);
        self.forward(result)
    }
    fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
        let result = self.drawer.set_metadata(key, value);
        self.forward(result)
    }
}

pub(crate) fn run<B: BinaryReader>(
    reader: &mut B,
    options: &mut DecodeOptions,
    limits: DecodeLimits,
    decode: impl FnOnce(&mut B, &mut DecodeOptions) -> Result<Option<ImgWarnings>, Error>,
) -> Result<Option<ImgWarnings>, Error> {
    let offset = reader.offset()?;
    let end = reader.seek(std::io::SeekFrom::End(0))?;
    reader.seek(std::io::SeekFrom::Start(offset))?;
    let length = end
        .checked_sub(offset)
        .ok_or_else(|| std::io::Error::other("invalid input position"))?;
    limits::check_input_length(length, limits.input_bytes)?;
    let signature = reader.read_bytes_no_move(usize::try_from(length.min(8))?)?;
    let handle_abort =
        signature.starts_with(b"\x89PNG\r\n\x1a\n") || signature.starts_with(&[0xff, 0xd8]);
    limits::scope(limits, || {
        let mut guard = Guard {
            drawer: options.drawer,
            limits,
            canvas: (0, 0),
            frames: 0,
            bytes: 0,
            terminated: false,
            handle_abort,
            ready: false,
            stopped: false,
            #[cfg(feature = "image-buffer-ex")]
            source: None,
        };
        let result = decode(
            reader,
            &mut DecodeOptions {
                debug_flag: options.debug_flag,
                drawer: &mut guard,
            },
        );
        match result {
            Err(error) if error.is::<Aborted>() => {
                guard.terminate(None)?;
                Ok(None)
            }
            other => other,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Probe {
        abort: bool,
        fail: bool,
        initializations: usize,
        draws: usize,
    }
    impl DrawCallback for Probe {
        fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
            self.initializations += 1;
            if self.fail {
                return Err(std::io::Error::other("deliberate init failure").into());
            }
            Ok(self.abort.then(CallbackResponse::abort))
        }
        #[cfg(feature = "image-buffer-ex")]
        fn init_ex(&mut self, w: usize, h: usize, _: Option<InitOptionsEx>) -> Response {
            self.init(w, h, None)
        }
        fn draw(
            &mut self,
            _: usize,
            _: usize,
            _: usize,
            _: usize,
            _: &[u8],
            _: Option<DrawOptions>,
        ) -> Response {
            self.draws += 1;
            Ok(None)
        }
        fn next(&mut self, _: Option<NextOptions>) -> Response {
            Ok(None)
        }
        fn terminate(&mut self, _: Option<TerminateOptions>) -> Response {
            Ok(None)
        }
        fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
            Ok(None)
        }
        fn set_metadata(&mut self, _: &str, _: DataMap) -> Response {
            Ok(self.abort.then(CallbackResponse::abort))
        }
    }
    fn guard(probe: &mut Probe, handle_abort: bool, limits: DecodeLimits) -> Guard<'_> {
        Guard {
            drawer: probe,
            limits,
            canvas: (0, 0),
            bytes: 0,
            frames: 0,
            terminated: false,
            handle_abort,
            ready: false,
            stopped: false,
            #[cfg(feature = "image-buffer-ex")]
            source: None,
        }
    }
    #[test]
    fn initialization_failure_and_abort_never_establish_ready_or_accounting() {
        for handle_abort in [false, true] {
            for (abort, fail) in [(true, false), (false, true)] {
                let mut probe = Probe {
                    abort,
                    fail,
                    ..Default::default()
                };
                let mut guard = guard(&mut probe, handle_abort, DecodeLimits::unlimited());
                let _ = guard.init(2, 3, None);
                assert!(!guard.ready);
                assert_eq!(guard.bytes, 0);
                assert_eq!(guard.canvas, (0, 0));
                assert!(guard.draw(0, 0, 1, 1, &[0; 4], None).is_err());
                assert!(guard.next(None).is_err());
                drop(guard);
                assert_eq!(probe.draws, 0);
            }
        }
    }
    #[test]
    fn failed_reinitialization_invalidates_ready_preserves_committed_accounting() {
        let mut probe = Probe::default();
        let mut guard = guard(
            &mut probe,
            false,
            DecodeLimits {
                pixels: 1,
                ..DecodeLimits::unlimited()
            },
        );
        guard.init(1, 1, None).unwrap();
        assert!(guard.init(2, 1, None).is_err());
        assert!(!guard.ready);
        assert_eq!(guard.bytes, 4);
        assert_eq!(guard.canvas, (1, 1));
        assert!(guard.draw(0, 0, 1, 1, &[0; 4], None).is_err());
        guard.init(1, 1, None).unwrap();
        guard.draw(0, 0, 1, 1, &[0; 4], None).unwrap();
    }
    #[test]
    fn metadata_abort_cannot_be_reset_by_ignored_initialization() {
        let mut probe = Probe {
            abort: true,
            ..Default::default()
        };
        let mut guard = guard(&mut probe, false, DecodeLimits::unlimited());
        guard.set_metadata("abort", DataMap::UInt(1)).unwrap();
        assert!(guard.init(1, 1, None).is_err());
        assert!(guard.draw(0, 0, 1, 1, &[0; 4], None).is_err());
        drop(guard);
        assert_eq!(probe.initializations, 0);
        assert_eq!(probe.draws, 0);
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn ex_canvas_and_rectangle_budgets_use_actual_format_and_byte_stride() {
        let mut probe = Probe::default();
        let mut guard = guard(
            &mut probe,
            false,
            DecodeLimits {
                expanded_bytes: 10,
                ..DecodeLimits::unlimited()
            },
        );
        let opt = || {
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Gray12,
                source_stride: Some(5),
                legacy: None,
            })
        };
        guard.init_ex(2, 2, opt()).unwrap();
        assert_eq!(guard.bytes, 10);
        guard.draw(0, 0, 2, 2, &[0; 10], None).unwrap();
        assert!(guard.draw(0, 0, 3, 2, &[0; 10], None).is_err());
        assert!(guard.draw(0, 0, 2, 3, &[0; 15], None).is_err());
        let mut invalid = [0; 10];
        invalid[1] = 16;
        assert!(guard.draw(0, 0, 2, 2, &invalid, None).is_err());
        assert!(guard.init_ex(2, 3, opt()).is_err());
        assert!(!guard.ready);
        assert_eq!(guard.bytes, 10);
        drop(guard);
        assert_eq!(probe.initializations, 1);
        assert_eq!(probe.draws, 1);
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn ex_tile_stride_and_expanding_storage_have_independent_budgets() {
        let mut probe = Probe::default();
        let mut guard = guard(
            &mut probe,
            false,
            DecodeLimits {
                expanded_bytes: 16,
                ..DecodeLimits::unlimited()
            },
        );
        guard
            .init_ex(
                4,
                4,
                Some(InitOptionsEx {
                    source_format: PixelFormatEx::Gray8,
                    source_stride: Some(2),
                    legacy: None,
                }),
            )
            .unwrap();
        assert_eq!(guard.bytes, 16);
        guard.draw(0, 0, 2, 2, &[0; 4], None).unwrap();
        crate::limits::scope(
            DecodeLimits {
                expanded_bytes: 16,
                ..DecodeLimits::unlimited()
            },
            || {
                let mut ex =
                    ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
                assert!(
                    ex.init_ex(
                        4,
                        4,
                        Some(InitOptionsEx {
                            source_format: PixelFormatEx::Gray8,
                            ..Default::default()
                        })
                    )
                    .is_err()
                );
                assert!(ex.bytes().is_none());
            },
        );
    }
}
