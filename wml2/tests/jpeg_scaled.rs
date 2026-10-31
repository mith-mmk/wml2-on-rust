#![cfg(feature = "jpeg")]
use std::path::PathBuf;
use wml2::jpeg::{JpegScale, image_from_scaled, image_from_scaled_with_limits};
use wml2::limits::DecodeLimits;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review")
}
fn source(name: &str) -> Vec<u8> {
    std::fs::read(root().join(format!("{name}.jpg"))).unwrap()
}
const SCALES: [JpegScale; 3] = [JpegScale::Half, JpegScale::Quarter, JpegScale::Eighth];
#[test]
fn reduced_pixels_match_independent_libjpeg_islow_no_fancy() {
    for name in [
        "jpeg-scaled-RGB-129x75-s0",
        "jpeg-scaled-RGB-129x75-s1",
        "jpeg-scaled-RGB-129x75-s2",
        "jpeg-scaled-RGB-9x7-s0",
        "jpeg-scaled-L-17x13-s0",
        "jpeg-scaled-L-1x1-s0",
        "jpeg-scaled-direct-rgb",
        "restart-minimal-i0",
        "restart-bgray-i0",
        "restart-b444-i0",
        "restart-b420-i0",
        "restart-odd420-i0",
    ] {
        let bytes = source(name);
        for scale in SCALES {
            let d = scale.denominator();
            let got =
                image_from_scaled(&bytes, scale).unwrap_or_else(|e| panic!("{name} /{d}: {e}"));
            let dims = got.dimensions;
            assert_eq!(got.image.width, (dims.coded.width as usize).div_ceil(d));
            assert_eq!(got.image.height, (dims.coded.height as usize).div_ceil(d));
            assert_eq!(dims.output.unwrap().width as usize, got.image.width);
            assert_eq!(dims.output.unwrap().height as usize, got.image.height);
            assert_eq!(dims.intrinsic_display, None);
            let raw = got.image.buffer.unwrap();
            let oracle = miniz_oxide::inflate::decompress_to_vec_zlib(
                &std::fs::read(root().join(format!("{name}-d{d}.rgba.zlib"))).unwrap(),
            )
            .unwrap();
            assert_eq!(raw.len(), oracle.len());
            let max = raw
                .iter()
                .zip(&oracle)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            let mae = raw
                .iter()
                .zip(&oracle)
                .map(|(a, b)| a.abs_diff(*b) as usize)
                .sum::<usize>() as f64
                / raw.len() as f64;
            eprintln!("{name} /{d}: max={max}, MAE={mae:.6}");
            assert!(max <= 3, "{name} /{d}: max {max}");
            assert!(raw.chunks_exact(4).all(|p| p[3] == 255));
        }
    }
}
#[test]
fn restart_intervals_wrap_and_edge_mcus_preserve_reduced_pixels() {
    for prefix in ["minimal", "bgray", "b444", "b420", "odd420"] {
        for scale in SCALES {
            let plain = image_from_scaled(&source(&format!("restart-{prefix}-i0")), scale).unwrap();
            for suffix in ["i1", "i2", "i3", "i8", "i100", "row"] {
                let name = format!("restart-{prefix}-{suffix}");
                let got = image_from_scaled(&source(&name), scale)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(got.dimensions, plain.dimensions);
                assert_eq!(got.image.buffer, plain.image.buffer, "{name} {scale:?}");
            }
        }
    }
}
#[test]
fn reduced_limits_admit_coded_pixels_but_only_reduced_raster() {
    let bytes = source("jpeg-scaled-RGB-129x75-s2");
    let budget = DecodeLimits::default();
    for scale in SCALES {
        for limited in [
            DecodeLimits {
                input_bytes: bytes.len() - 1,
                ..budget
            },
            DecodeLimits {
                pixels: 129 * 75 - 1,
                ..budget
            },
            DecodeLimits {
                expanded_bytes: 1,
                ..budget
            },
            DecodeLimits {
                metadata_bytes: 1,
                ..budget
            },
            DecodeLimits {
                frames: 0,
                ..budget
            },
            DecodeLimits {
                animation_bytes: 1,
                ..budget
            },
        ] {
            assert!(image_from_scaled_with_limits(&bytes, scale, limited).is_err());
        }
        assert!(
            image_from_scaled_with_limits(
                &bytes,
                scale,
                DecodeLimits {
                    pixels: 129 * 75,
                    ..budget
                }
            )
            .is_ok()
        );
    }
}
#[test]
fn unsupported_reduced_processes_reject_without_changing_ordinary_decode() {
    for name in [
        "restart-p420-i1",
        "restart-pgray-i0",
        "jpeg-icc-cmyk",
        "jpeg-icc-ycck",
    ] {
        let bytes = source(name);
        assert!(
            image_from_scaled(&bytes, JpegScale::Eighth).is_err(),
            "{name}"
        );
        assert!(wml2::draw::image_from(&bytes).is_ok(), "legacy {name}");
    }
}

fn segment(bytes: &[u8], marker: u8) -> usize {
    bytes.windows(2).position(|p| p == [0xff, marker]).unwrap()
}
fn insert_app(bytes: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    let len = (payload.len() + 2) as u16;
    bytes.splice(
        2..2,
        [0xff, marker, (len >> 8) as u8, len as u8]
            .into_iter()
            .chain(payload.iter().copied()),
    );
}
#[test]
fn malformed_headers_entropy_tables_and_restarts_return_errors() {
    let original = source("restart-b420-i1");
    let sof = segment(&original, 0xc0);
    let sos = segment(&original, 0xda);
    let dqt = segment(&original, 0xdb);
    let dht = segment(&original, 0xc4);
    for (at, value) in [
        (sof + 4, 12),
        (sof + 6, 0),
        (sof + 11, 0x18),
        (sof + 11, 0x01),
        (sof + 12, 4),
        (sof + 13, original[sof + 10]),
        (sos + 5, 255),
        (sos + 6, 0x40),
        (dqt + 4, 0x10),
        (dqt + 5, 0),
        (dht + 4, 0x20),
        (dht + 5, 255),
    ] {
        let mut bad = original.clone();
        bad[at] = value;
        assert!(
            image_from_scaled(&bad, JpegScale::Eighth).is_err(),
            "byte {at}={value}"
        );
    }
    let rst = original.windows(2).position(|p| p == [255, 208]).unwrap();
    let mut bad = original.clone();
    bad[rst + 1] = 211;
    assert!(image_from_scaled(&bad, JpegScale::Half).is_err());
    let mut missing = original.clone();
    missing.drain(rst..rst + 2);
    assert!(image_from_scaled(&missing, JpegScale::Quarter).is_err());
    for end in [
        0,
        1,
        2,
        3,
        sof,
        sof + 4,
        sos + 4,
        rst,
        rst + 1,
        original.len() - 2,
        original.len() - 1,
    ] {
        assert!(
            image_from_scaled(&original[..end], JpegScale::Eighth).is_err(),
            "length {end}"
        );
    }
    for marker in [0xe0, 0xee] {
        let mut bad = original.clone();
        insert_app(
            &mut bad,
            marker,
            if marker == 0xe0 { b"JFIF\0" } else { b"Adobe" },
        );
        assert!(image_from_scaled(&bad, JpegScale::Eighth).is_err());
    }
}
#[test]
fn all_short_prefixes_and_header_mutations_are_bounded_without_panics() {
    let bytes = source("jpeg-scaled-L-1x1-s0");
    for end in 0..bytes.len() {
        assert!(
            image_from_scaled(&bytes[..end], JpegScale::Eighth).is_err(),
            "prefix {end}"
        );
    }
    for at in 0..bytes.len() {
        for bit in [1, 16, 128] {
            let mut changed = bytes.clone();
            changed[at] ^= bit;
            // Valid mutations may decode; malformed ones must never index/panic.
            let _ = image_from_scaled_with_limits(
                &changed,
                JpegScale::Eighth,
                DecodeLimits {
                    pixels: 1_000_000,
                    expanded_bytes: 1_000_000,
                    ..Default::default()
                },
            );
        }
    }
}
#[test]
fn opaque_exif_orientation_and_icc_association_are_preserved_with_budget_checks() {
    use wml2::metadata::DataMap;
    let mut bytes = source("jpeg-scaled-L-17x13-s0");
    let exif = b"Exif\0\0II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    insert_app(&mut bytes, 0xe1, exif);
    let mut profile = vec![0; 128];
    profile[16..20].copy_from_slice(b"GRAY");
    let mut app = b"ICC_PROFILE\0\x01\x01".to_vec();
    app.extend_from_slice(&profile);
    insert_app(&mut bytes, 0xe2, &app);
    let got = image_from_scaled(&bytes, JpegScale::Half).unwrap();
    assert_eq!((got.image.width, got.image.height), (9, 7));
    let map = got.image.metadata.unwrap();
    assert!(matches!(&map["EXIF Raw"],DataMap::Raw(v) if v==exif));
    assert!(matches!(&map["Source ICC Profile"],DataMap::ICCProfile(v) if v==&profile));
    assert!(!map.contains_key("ICC Profile"));
    // Invalid/duplicate ICC sequence discards the association as a whole.
    insert_app(&mut bytes, 0xe2, &app);
    assert!(
        !image_from_scaled(&bytes, JpegScale::Half)
            .unwrap()
            .image
            .metadata
            .unwrap()
            .contains_key("Source ICC Profile")
    );
    let mut rgb = source("jpeg-scaled-direct-rgb");
    profile[16..20].copy_from_slice(b"RGB ");
    let mut second = b"ICC_PROFILE\0\x02\x02".to_vec();
    second.extend_from_slice(&profile[64..]);
    let mut first = b"ICC_PROFILE\0\x01\x02".to_vec();
    first.extend_from_slice(&profile[..64]);
    insert_app(&mut rgb, 0xe2, &first);
    insert_app(&mut rgb, 0xe2, &second);
    assert!(
        matches!(&image_from_scaled(&rgb,JpegScale::Quarter).unwrap().image.metadata.unwrap()["ICC Profile"],DataMap::ICCProfile(v) if v==&profile)
    );
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
struct AllocationProbe;
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static LARGEST:Cell<usize>=const{Cell::new(0)};}
unsafe impl GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        TRACK.with(|t| {
            if t.get() {
                LARGEST.with(|m| m.set(m.get().max(l.size())));
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        TRACK.with(|t| {
            if t.get() {
                LARGEST.with(|m| m.set(m.get().max(n)));
            }
        });
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationProbe = AllocationProbe;
fn largest_allocation<T>(body: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TRACK.set(false);
        }
    }
    LARGEST.set(0);
    TRACK.set(true);
    let _reset = Reset;
    let result = body();
    (result, LARGEST.get())
}
#[test]
fn small_reduced_budget_has_no_coded_raster_or_coefficient_allocation() {
    let bytes = source("jpeg-scaled-bench");
    let (got, largest) = largest_allocation(|| {
        image_from_scaled_with_limits(
            &bytes,
            JpegScale::Eighth,
            DecodeLimits {
                expanded_bytes: 140_000,
                ..Default::default()
            },
        )
    });
    let got = got.unwrap();
    assert_eq!((got.image.width, got.image.height), (144, 112));
    assert_eq!(largest, 144 * 112 * 4);
    // No 1152x896x4 raster or image-wide coefficients fit this budget.
    let mut metadata = vec![b'x'; 60_000];
    metadata[..6].copy_from_slice(b"Exif\0\0");
    let mut large = bytes.clone();
    insert_app(&mut large, 0xe1, &metadata);
    for budget in [
        DecodeLimits {
            metadata_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 140_000,
            ..Default::default()
        },
    ] {
        let (result, largest) =
            largest_allocation(|| image_from_scaled_with_limits(&large, JpegScale::Eighth, budget));
        assert!(result.is_err());
        assert!(largest < 1024, "pre-admission allocation {largest}");
    }
}

fn marker(out: &mut Vec<u8>, kind: u8, payload: &[u8]) {
    out.extend_from_slice(&[255, kind]);
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
}
#[test]
fn component_order_and_noninterleaved_gray_sampling_use_scan_mcu_geometry() {
    // One DC-only RGB MCU, encoded in B,R,G scan order. DC category 10 has
    // code 0; AC EOB has code 0. No color conversion should reorder channels.
    let mut bytes = vec![255, 216];
    let mut q = vec![0];
    q.extend_from_slice(&[1; 64]);
    marker(&mut bytes, 0xdb, &q);
    marker(
        &mut bytes,
        0xc0,
        &[
            8, 0, 8, 0, 8, 3, b'R', 0x11, 0, b'G', 0x11, 0, b'B', 0x11, 0,
        ],
    );
    let mut huff = vec![0, 1];
    huff.extend_from_slice(&[0; 15]);
    huff.push(10);
    huff.extend_from_slice(&[0x10, 1]);
    huff.extend_from_slice(&[0; 15]);
    huff.push(0);
    marker(&mut bytes, 0xc4, &huff);
    marker(&mut bytes, 0xda, &[3, b'B', 0, b'R', 0, b'G', 0, 0, 63, 0]);
    let mut bits = String::new();
    for dc in [-640i32, -896, -768] {
        bits.push_str(&format!("0{:010b}0", dc + 1023));
    }
    while !bits.len().is_multiple_of(8) {
        bits.push('1');
    }
    for chunk in bits.as_bytes().chunks_exact(8) {
        let b = chunk.iter().fold(0u8, |v, b| v * 2 + (b - b'0'));
        bytes.push(b);
        if b == 255 {
            bytes.push(0);
        }
    }
    bytes.extend_from_slice(&[255, 217]);
    for scale in SCALES {
        let image = image_from_scaled(&bytes, scale).unwrap().image;
        assert!(
            image
                .buffer
                .unwrap()
                .chunks_exact(4)
                .all(|p| p == [16, 32, 48, 255])
        );
    }
    // In a single-component scan, SOF H/V=4 does not make 16 blocks per MCU.
    let mut gray = vec![255, 216];
    marker(&mut gray, 0xdb, &q);
    marker(&mut gray, 0xc0, &[8, 0, 17, 0, 9, 1, 1, 0x44, 0]);
    huff[17] = 0;
    marker(&mut gray, 0xc4, &huff);
    marker(&mut gray, 0xda, &[1, 1, 0, 0, 63, 0]);
    gray.extend_from_slice(&[0, 15, 255, 217]);
    for scale in SCALES {
        let got = image_from_scaled(&gray, scale).unwrap();
        assert!(
            got.image
                .buffer
                .unwrap()
                .chunks_exact(4)
                .all(|p| p == [128, 128, 128, 255])
        );
    }
}
