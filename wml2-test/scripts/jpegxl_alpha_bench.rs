use std::{hint::black_box, time::Instant};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = std::fs::read(&args[1]).unwrap();
    let managed = args[2] == "managed";
    let n: usize = args[3].parse().unwrap();
    println!("ns,hash,width,height");
    for i in 0..n + 3 {
        let begin = Instant::now();
        let image = if managed {
            wml2::jpegxl::image_from_srgb_with_limits(black_box(&input), Default::default())
                .unwrap()
        } else {
            wml2::draw::image_from_with_limits(black_box(&input), Default::default()).unwrap()
        };
        let ns = begin.elapsed().as_nanos();
        let pixels = image.buffer.unwrap();
        let hash = pixels
            .iter()
            .fold(0u64, |h, &b| h.wrapping_mul(31).wrapping_add(b as u64));
        if i >= 3 {
            println!("{ns},{hash},{},{}", image.width, image.height);
        }
        if i == 0 && args.len() > 4 {
            std::fs::write(&args[4], pixels).unwrap();
        }
    }
}
