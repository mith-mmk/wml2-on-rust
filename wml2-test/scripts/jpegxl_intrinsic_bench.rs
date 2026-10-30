use std::{hint::black_box, time::Instant};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = std::fs::read(&args[1]).unwrap();
    let managed = args[2] == "managed";
    let n: usize = args[3].parse().unwrap();
    if args[2] == "gap" {
        println!("info={:?}", wml2::image_info::image_info(&input));
        println!(
            "decode={:?}",
            wml2::draw::image_from(&input).map(|v| (v.width, v.height))
        );
        return;
    }
    if args[2] == "info" {
        println!("ns,bytes_read,width,height");
        for i in 0..n + 3 {
            let begin = Instant::now();
            let v = wml2::image_info::image_info_with_limits(
                black_box(&input),
                wml2::limits::DecodeLimits {
                    pixels: 0,
                    animation_bytes: 0,
                    expanded_bytes: 8192,
                    metadata_bytes: 4096,
                    ..Default::default()
                },
            )
            .unwrap();
            let ns = begin.elapsed().as_nanos();
            if i >= 3 {
                println!(
                    "{ns},{},{},{}",
                    v.bytes_read, v.dimensions.coded.width, v.dimensions.coded.height
                );
            }
        }
        return;
    }
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
