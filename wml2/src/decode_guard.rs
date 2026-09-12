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
}
impl Guard<'_> {
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
        let bytes = self.size(w, h)?;
        limits::check(bytes, self.limits.animation_bytes, "canvas storage")?;
        self.canvas = (w, h);
        self.bytes = bytes;
        response(self.drawer.init(w, h, option), self.handle_abort)
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
        response(
            self.drawer.draw(x, y, w, h, data, option),
            self.handle_abort,
        )
    }
    fn next(&mut self, option: Option<NextOptions>) -> Response {
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
        let result = response(self.drawer.next(option), self.handle_abort)?;
        self.bytes = total;
        self.frames = frames;
        Ok(result)
    }
    fn terminate(&mut self, option: Option<TerminateOptions>) -> Response {
        if self.handle_abort && self.terminated {
            return Ok(None);
        }
        self.terminated = true;
        self.drawer.terminate(option)
    }
    fn verbose(&mut self, text: &str, option: Option<VerboseOptions>) -> Response {
        response(self.drawer.verbose(text, option), self.handle_abort)
    }
    fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
        response(self.drawer.set_metadata(key, value), self.handle_abort)
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
