//! Developer probe: same source can link against either side of format admission.
use std::{hint::black_box, time::Instant};
use wml2::{jpegxl::*, limits::DecodeLimits};
enum Pixels {
    Bytes(Vec<u8>),
    Words(Vec<u16>),
    Floats(Vec<f32>),
}
fn decode(input: &[u8], mode: &str, quota: usize) -> Result<Pixels, Box<dyn std::error::Error>> {
    let limits = DecodeLimits::default();
    let policy = JxlResourceLimits {
        workspace_bytes: quota,
        ..Default::default()
    };
    Ok(match mode {
        "legacy" => Pixels::Bytes(
            wml2::draw::image_from_with_limits(input, limits)?
                .buffer
                .unwrap(),
        ),
        "ordinary" => Pixels::Bytes(
            image_from_with_resource_limits(input, limits, policy)?
                .buffer
                .unwrap(),
        ),
        "managed" => Pixels::Bytes(
            image_from_srgb_with_resource_limits(input, limits, policy)?
                .buffer
                .unwrap(),
        ),
        "f32" => Pixels::Floats(
            image_f32_from_with_resource_limits_and_cancel(input, limits, policy, &|| false)?.rgba,
        ),
        "hdr" => Pixels::Floats(
            image_hdr_encoded_from_with_resource_limits_and_cancel(input, limits, policy, &|| {
                false
            })?
            .rgba_encoded,
        ),
        "preview" => Pixels::Words(
            preview_from_with_resource_limits(input, limits, policy)?
                .unwrap()
                .rgba16,
        ),
        _ => panic!("unknown mode"),
    })
}
fn main() {
    let a: Vec<_> = std::env::args().collect();
    let input = std::fs::read(&a[1]).unwrap();
    let mode = &a[2];
    if a[3] == "floor" {
        assert!(mode != "legacy");
        let (mut lo, mut hi) = (0, 512 << 20);
        decode(&input, mode, hi).unwrap_or_else(|e| panic!("baseline input must succeed: {e}"));
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if decode(&input, mode, mid).is_ok() {
                hi = mid
            } else {
                lo = mid + 1
            }
        }
        assert!(decode(&input, mode, lo).is_ok());
        assert!(lo > 0 && decode(&input, mode, lo - 1).is_err());
        println!("{{\"workspace_bytes\":{lo}}}");
        return;
    }
    let n: usize = a[3].parse().unwrap();
    println!("ns,hash");
    for i in 0..n + 3 {
        let t = Instant::now();
        let pixels = decode(black_box(&input), mode, 512 << 20).unwrap();
        let ns = t.elapsed().as_nanos();
        let bytes = match pixels {
            Pixels::Bytes(v) => v,
            Pixels::Words(v) => v.iter().flat_map(|v| v.to_le_bytes()).collect(),
            Pixels::Floats(v) => v.iter().flat_map(|v| v.to_le_bytes()).collect(),
        };
        let hash = bytes
            .iter()
            .fold(0u64, |h, &b| h.wrapping_mul(31).wrapping_add(b as u64));
        if i >= 3 {
            println!("{ns},{hash}");
        }
        if i == 0 && a.len() > 4 {
            std::fs::write(&a[4], bytes).unwrap();
        }
    }
}
