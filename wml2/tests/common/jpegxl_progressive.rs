//! Trusted <=64KiB fixtures only; no production resource or publication API.
#![allow(dead_code)]
use jxl_codec::api::*;
use std::io::{self, IoSliceMut};
type Error = Box<dyn std::error::Error>;
struct Window {
    data: Vec<u8>,
    pos: usize,
    exposed: usize,
}
impl JxlBitstreamInput for Window {
    fn available_bytes(&mut self) -> io::Result<usize> {
        Ok(self.exposed - self.pos)
    }
    fn read(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        let start = self.pos;
        for buf in bufs {
            let n = buf.len().min(self.exposed - self.pos);
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
        }
        Ok(self.pos - start)
    }
}
enum State {
    Start(JxlDecoder<states::Initialized>),
    Image(JxlDecoder<states::WithImageInfo>),
    Frame(JxlDecoder<states::WithFrameInfo>),
    Empty,
}
struct Session {
    state: State,
    input: Window,
    pixels: Vec<u16>,
    w: usize,
    h: usize,
    done: bool,
    length: Option<u64>,
    need: usize,
    updates: Vec<(usize, Vec<u16>)>,
}
impl Session {
    fn new(data: &[u8]) -> Self {
        let mut opt = JxlDecoderOptions::default();
        opt.high_precision = true;
        opt.sample_limit = Some(32 * 24 * 4 + 1);
        opt.skip_preview = true;
        opt.render_spot_colors = false;
        Self {
            state: State::Start(JxlDecoder::new(opt)),
            input: Window {
                data: data.to_vec(),
                pos: 0,
                exposed: 0,
            },
            pixels: vec![],
            w: 0,
            h: 0,
            done: false,
            length: None,
            need: 0,
            updates: vec![],
        }
    }
    fn expose(&mut self, end: usize, paused: bool) -> Result<bool, Error> {
        if paused {
            return Ok(false);
        }
        if end < self.input.exposed || end > self.input.data.len() {
            return Err("exposure order".into());
        }
        self.input.exposed = end;
        self.poll()?;
        Ok(true)
    }
    fn poll(&mut self) -> Result<(), Error> {
        loop {
            let state = std::mem::replace(&mut self.state, State::Empty);
            match state {
                State::Start(d) => match d.process(&mut self.input, None)? {
                    ProcessingResult::Complete { result: mut d } => {
                        let (w, h) = d.basic_info().size;
                        if (w, h) != (32, 24) || d.basic_info().animation.is_some() {
                            return Err("trusted fixture canvas".into());
                        }
                        self.w = w;
                        self.h = h;
                        self.pixels = vec![0x6bad; w * h * 4];
                        let mut f = JxlPixelFormat::rgba16(d.basic_info().extra_channels.len());
                        f.color_data_format = Some(JxlDataFormat::U16 {
                            endianness: Endianness::LittleEndian,
                            bit_depth: 16,
                        });
                        d.set_pixel_format(f)?;
                        self.state = State::Image(d);
                    }
                    ProcessingResult::NeedsMoreInput {
                        fallback,
                        size_hint,
                    } => {
                        self.state = State::Start(fallback);
                        self.need = size_hint;
                        break;
                    }
                },
                State::Image(d) => {
                    if self.done {
                        self.state = State::Image(d);
                        break;
                    }
                    match d.process(&mut self.input, None)? {
                        ProcessingResult::Complete { result } => self.state = State::Frame(result),
                        ProcessingResult::NeedsMoreInput {
                            mut fallback,
                            size_hint,
                        } => {
                            let mut buffers = buffers(&mut self.pixels, self.w, self.h);
                            if fallback.flush_pixels(&mut buffers, None)? {
                                assert!(self.updates.len() < 16, "trusted snapshot cap");
                                self.updates.push((self.input.exposed, self.pixels.clone()));
                            }
                            self.state = State::Image(fallback);
                            self.need = size_hint;
                            break;
                        }
                    }
                }
                State::Frame(d) => {
                    let mut buffers = buffers(&mut self.pixels, self.w, self.h);
                    match d.process(&mut self.input, &mut buffers, None)? {
                        ProcessingResult::Complete { result } => {
                            if result.has_more_frames() {
                                return Err("fixture has multiple displayed frames".into());
                            }
                            self.done = true;
                            self.length = result.file_length();
                            self.state = State::Image(result);
                            break;
                        }
                        ProcessingResult::NeedsMoreInput {
                            mut fallback,
                            size_hint,
                        } => {
                            if fallback.flush_pixels(&mut buffers, None)? {
                                assert!(self.updates.len() < 16, "trusted snapshot cap");
                                self.updates.push((self.input.exposed, self.pixels.clone()));
                            }
                            self.state = State::Frame(fallback);
                            self.need = size_hint;
                            break;
                        }
                    }
                }
                State::Empty => unreachable!(),
            }
        }
        // Input ownership experiment: permanently erase only bytes actually consumed.
        self.input.data[..self.input.pos].fill(0xa5);
        Ok(())
    }
}
fn buffers(p: &mut [u16], w: usize, h: usize) -> [JxlOutputBuffer<'_>; 1] {
    // aligned initialized u16 output, little endian
    let bytes = unsafe { std::slice::from_raw_parts_mut(p.as_mut_ptr().cast::<u8>(), p.len() * 2) };
    [JxlOutputBuffer::new(bytes, h, w * 8)]
}
fn bytes(p: &[u16]) -> Vec<u8> {
    p.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn run(data: &[u8], ends: &[usize]) -> Result<Session, Error> {
    let mut s = Session::new(data);
    for &end in ends {
        s.input.exposed = end;
        s.poll()?;
    }
    Ok(s)
}
fn inspect(data: &[u8]) -> Result<(), Error> {
    use jxl_codec::bit_reader::BitReader;
    use jxl_codec::headers::encodings::UnconditionalCoder;
    use jxl_codec::headers::frame_header::FrameHeader;
    use jxl_codec::headers::toc::IncrementalTocReader;
    use jxl_codec::headers::{FileHeader, JxlHeader};
    let mut br = BitReader::new(data);
    let header = FileHeader::read(&mut br)?;
    if header.image_metadata.color_encoding.want_icc {
        return Err("trusted simple profile only".into());
    }
    br.jump_to_byte_boundary()?;
    let mut at = br.total_bits_read() / 8;
    for i in 0..8 {
        let mut br = BitReader::new(data.get(at..).ok_or("header offset")?);
        let non = header.frame_header_nonserialized();
        let mut frame = FrameHeader::read_unconditional(&(), &mut br, &non)?;
        frame.postprocess(&non);
        println!(
            "coded {i} offset={at} visible={} passes={} last={}",
            frame.is_visible(),
            frame.passes.num_passes,
            frame.is_last
        );
        let mut toc = IncrementalTocReader::new(frame.num_toc_entries().try_into()?, &mut br)?;
        while !toc.is_complete() {
            toc.read_step(&mut br)?;
        }
        let toc = toc.finalize();
        br.jump_to_byte_boundary()?;
        let payload = toc.entries.iter().try_fold(0usize, |n, v| {
            n.checked_add(*v as usize).ok_or("toc overflow")
        })?;
        at = at
            .checked_add(br.total_bits_read() / 8)
            .and_then(|n| n.checked_add(payload))
            .ok_or("offset overflow")?;
        if frame.is_last {
            assert_eq!(at, data.len());
            return Ok(());
        }
    }
    Err("trusted coded-frame cap".into())
}
fn trailing(mut s: Session) -> String {
    if !s.done {
        return "frame incomplete".into();
    }
    if let State::Image(d) = std::mem::replace(&mut s.state, State::Empty) {
        match d.process_trailing_data(&mut s.input) {
            Ok(ProcessingResult::Complete { result }) => format!(
                "complete remaining={} unbounded_box={}",
                s.input.exposed - s.input.pos,
                result.trailing_box().is_some()
            ),
            Ok(ProcessingResult::NeedsMoreInput { size_hint, .. }) => format!(
                "need_more hint={size_hint} remaining={}",
                s.input.exposed - s.input.pos
            ),
            Err(e) => format!("error {e}"),
        }
    } else {
        "unexpected state".into()
    }
}
#[cfg(not(test))]
pub fn cli() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    let data = std::fs::read(&args[1])?;
    if data.len() > 65536 {
        return Err("trusted input cap".into());
    }
    if args.get(3).is_some_and(|x| x == "--negative") {
        assert!(
            wml2::jpegxl::image_from_srgb(&data).is_err(),
            "invalid envelope accepted"
        );
        match run(&data, &[data.len()]) {
            Ok(s) => {
                println!(
                    "negative core_frame_complete={} file_length={:?} consumed={} WML2_rejected=true output_files=0",
                    s.done, s.length, s.input.pos
                );
                println!("trailing {}", trailing(s));
            }
            Err(e) => println!("negative core_error={e} WML2_rejected=true output_files=0"),
        };
        return Ok(());
    }
    if data.starts_with(&[255, 10]) {
        inspect(&data)?;
    }
    let whole = wml2::jpegxl::image_from_srgb(&data)?;
    let rgba = whole.buffer.ok_or("no WML2 output")?;
    assert_eq!(rgba.len(), 32 * 24 * 4);
    std::fs::write(format!("{}.srgb.rgba", args[2]), &rgba)?;
    let all = run(&data, &[data.len()])?;
    if !all.done {
        return Err("sealed incomplete".into());
    }
    println!("trailing {}", trailing(run(&data, &[data.len()])?));
    let expected = all.pixels.clone();
    std::fs::write(&args[2], bytes(&expected))?;
    let mut split_checks = 0;
    let mut early_complete = 0;
    let mut sealed_prefix_rejections = 0;
    let mut sealed_prefix_acceptances = 0;
    let mut backpressure_pauses = 0;
    let mut abort_drops = 0;
    for at in 0..=data.len() {
        let mut s = run(&data, &[at])?;
        // Caller policy experiment: no poll while downstream is paused. Withheld
        // input remains in the source; no decoder progress or extra snapshots occur.
        let paused = (
            s.input.pos,
            s.input.exposed,
            s.updates.len(),
            s.pixels.clone(),
        );
        assert!(!s.expose(data.len(), true)?);
        assert_eq!(
            paused,
            (
                s.input.pos,
                s.input.exposed,
                s.updates.len(),
                s.pixels.clone()
            )
        );
        backpressure_pauses += 1;
        // Independent temporary-versus-sealed observation. A valid complete prefix
        // of a container may itself be a valid file, so classify rather than assume.
        if at < data.len() {
            if wml2::jpegxl::image_from_srgb(&data[..at]).is_err() {
                sealed_prefix_rejections += 1;
            } else {
                sealed_prefix_acceptances += 1;
            }
        }
        // Abort between calls: dropping the actual core state releases it. This is
        // not an interruption test inside process(), nor a production Abort API.
        let abandoned = run(&data, &[at])?;
        drop(abandoned);
        abort_drops += 1;
        if s.done && at < data.len() {
            early_complete += 1;
        }
        let before = s.pixels.clone();
        let pos = s.input.pos;
        let done = s.done;
        s.poll()?;
        assert_eq!(s.input.pos, pos, "no input consumed on empty retry");
        assert_eq!(s.done, done);
        assert_eq!(s.pixels, before, "empty retry pixels changed");
        s.input.exposed = data.len();
        s.poll()?;
        assert!(s.done, "split {at}");
        assert_eq!(s.pixels, expected, "split {at}");
        assert_eq!(s.length, all.length, "boundary {at}");
        split_checks += 1;
    }
    println!(
        "policy sealed_prefix_rejections={sealed_prefix_rejections} sealed_prefix_acceptances={sealed_prefix_acceptances} backpressure_pauses={backpressure_pauses} abort_drops={abort_drops}"
    );
    // Arithmetic-only proposed admission ledger, independent of the core: dropping
    // consumed chunks must never replenish cumulative accepted-input allowance.
    let mut accepted = 0usize;
    for n in [4usize, 4, 2] {
        accepted = accepted.checked_add(n).ok_or("input overflow")?;
        assert!(accepted <= 10);
    }
    assert!(accepted.checked_add(1).filter(|n| *n <= 10).is_none());
    assert!(usize::MAX.checked_add(1).is_none());
    let ends: Vec<_> = (0..=data.len()).collect();
    let stream = run(&data, &ends)?;
    assert!(stream.done);
    assert_eq!(stream.pixels, expected);
    let different = stream
        .updates
        .iter()
        .filter(|(_, p)| *p != expected)
        .count();
    let untouched = stream
        .updates
        .iter()
        .filter(|(_, p)| p.contains(&0x6bad))
        .count();
    let unique = stream
        .updates
        .windows(2)
        .filter(|p| p[0].1 != p[1].1)
        .count();
    println!(
        "{{\"bytes\":{},\"all_two_chunk_boundaries\":{},\"bytewise_updates\":{},\"provisional_different_from_final\":{},\"successive_different_updates\":{},\"sentinel_present_updates\":{},\"frame_complete_before_input_end_boundaries\":{},\"file_length\":{:?},\"poison_consumed_input\":true}}",
        data.len(),
        split_checks,
        stream.updates.len(),
        different,
        unique,
        untouched,
        early_complete,
        all.length.unwrap_or(0)
    );
    for (i, (end, p)) in stream.updates.iter().enumerate() {
        std::fs::write(format!("{}.partial-{i}-{end}", args[2]), bytes(p))?;
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn frozen_incremental_boundaries_match_independent_c_pixels() {
    let cases = [
        (
            include_bytes!("../fixtures/review/jxl-progressive-ac.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-ac.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-ac-jxlc.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-ac.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-ac-jxlp.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-ac.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-ac-trailer.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-ac.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-dc.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-dc.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-dc-jxlc.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-dc.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-dc-jxlp.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-dc.rgba16le").as_slice(),
        ),
        (
            include_bytes!("../fixtures/review/jxl-progressive-dc-trailer.jxl").as_slice(),
            include_bytes!("../fixtures/review/jxl-progressive-dc.rgba16le").as_slice(),
        ),
    ];
    for (data, oracle) in cases {
        let full = run(data, &[data.len()]).unwrap();
        assert!(full.done);
        let actual = bytes(&full.pixels);
        assert_eq!(actual.len(), oracle.len());
        for (a, b) in actual.chunks_exact(2).zip(oracle.chunks_exact(2)) {
            assert!(
                u16::from_le_bytes([a[0], a[1]]).abs_diff(u16::from_le_bytes([b[0], b[1]])) <= 3
            );
        }
        // Exhaustive in the dedicated optimized CI step. Debug feature/platform
        // matrix keeps representative header/box/tail splits plus bytewise feed.
        let splits: Vec<usize> = if cfg!(debug_assertions) {
            vec![
                0,
                1,
                2,
                3,
                4,
                7,
                8,
                11,
                12,
                19,
                20,
                31,
                32,
                data.len() / 2,
                data.len() - 1,
                data.len(),
            ]
        } else {
            (0..=data.len()).collect()
        };
        for split in splits {
            let resumed = run(data, &[split, data.len()]).unwrap();
            assert!(resumed.done);
            assert_eq!(resumed.pixels, full.pixels, "split {split}");
            assert_eq!(resumed.length, full.length);
        }
        let ends: Vec<_> = (0..=data.len()).collect();
        let stream = run(data, &ends).unwrap();
        assert!(stream.done);
        assert_eq!(stream.pixels, full.pixels);
        assert!(stream.updates.len() >= 2);
        assert!(stream.updates.iter().any(|(_, p)| p != &full.pixels));
    }
}
