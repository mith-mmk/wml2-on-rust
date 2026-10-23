//! In-memory JPEG decode timing; file I/O, hashing and optional RGBA dumps are untimed.
//! Usage: jpeg_bench <repeats> <warmups> <jpeg>...
use std::time::Instant;
use wml2::draw::*;

struct Sink {
    image: ImageBuffer,
    start: Instant,
    first: u128,
}
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
    fn set_metadata(&mut self, k: &str, v: wml2::metadata::DataMap) -> Response {
        self.image.set_metadata(k, v)
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
}
fn main() {
    let mut args = std::env::args().skip(1);
    let repeats: usize = args.next().expect("repeats").parse().unwrap();
    let warmups: usize = args.next().expect("warmups").parse().unwrap();
    assert!(repeats > 0);
    println!("file,iteration,decode_ns,first_draw_ns,hash");
    for file in args {
        let encoded = std::fs::read(&file).unwrap();
        let mut expected = None;
        for i in 0..warmups + repeats {
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
            let ns = sink.start.elapsed().as_nanos();
            let pixels = sink.image.buffer.as_ref().unwrap();
            let digest = pixels.iter().fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
            });
            assert_eq!(*expected.get_or_insert(digest), digest);
            let name = std::path::Path::new(&file)
                .file_name()
                .unwrap()
                .to_string_lossy();
            if i >= warmups {
                println!("{name},{},{ns},{},{digest}", i - warmups, sink.first);
            }
            if i == 0 {
                if let Some(dir) = std::env::var_os("JPEG_BENCH_RGBA_DIR") {
                    std::fs::write(
                        std::path::PathBuf::from(dir).join(format!("{name}.rgba")),
                        pixels,
                    )
                    .unwrap();
                }
            }
        }
    }
}
