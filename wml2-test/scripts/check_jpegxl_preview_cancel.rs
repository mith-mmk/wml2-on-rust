use std::{cell::Cell, path::PathBuf};
use wml2::{
    jpegxl::{DecodeCancelled, image_from_srgb_with_limits_and_cancel as decode},
    limits::DecodeLimits,
};
fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap());
    for name in ["rgba8", "oriented-icc16", "lossy"] {
        let input = std::fs::read(root.join(format!("jxl-preview-{name}.jxl"))).unwrap();
        let expected = miniz_oxide::inflate::decompress_to_vec_zlib(
            &std::fs::read(root.join(format!("jxl-preview-{name}.srgb.rgba.zlib"))).unwrap(),
        )
        .unwrap();
        let calls = Cell::new(0usize);
        let image = decode(&input, DecodeLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        let data = image.buffer.unwrap();
        assert_eq!(data.len(), expected.len());
        for (a, b) in data.chunks_exact(4).zip(expected.chunks_exact(4)) {
            for c in 0..3 {
                assert!(a[c].abs_diff(b[c]) <= 2);
            }
            assert_eq!(a[3], b[3]);
        }
        let total = calls.get();
        assert!(total > 3);
        for stop in 1..=total {
            let n = Cell::new(0);
            let e = match decode(&input, DecodeLimits::default(), &|| {
                n.set(n.get() + 1);
                n.get() >= stop
            }) {
                Ok(_) => panic!("published cancelled output"),
                Err(e) => e,
            };
            assert!(e.downcast_ref::<DecodeCancelled>().is_some(), "{e}");
        }
        let limits = DecodeLimits {
            pixels: 0,
            ..Default::default()
        };
        let e = decode(&input, limits, &|| true).err().unwrap();
        assert!(e.downcast_ref::<DecodeCancelled>().is_some());
        let e = decode(&input, limits, &|| false).err().unwrap();
        assert!(e.downcast_ref::<DecodeCancelled>().is_none());
        println!(
            "{name}: independent CMS oracle passed; {total} deterministic cancellation points and precedence passed"
        );
    }
}
