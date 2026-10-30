use super::*;
struct Case {
    input: &'static [u8],
    rgba: &'static [u8],
    #[cfg(feature = "image-buffer-ex")]
    native: &'static [u8],
    #[cfg(feature = "color-management")]
    managed: &'static [u8],
    size: (usize, usize),
    #[cfg(feature = "image-buffer-ex")]
    bits: u8,
    #[cfg(feature = "image-buffer-ex")]
    gray: bool,
    tolerance: u8,
    preview_start: usize,
    main_start: usize,
}
macro_rules! case {
    ($name:literal,$w:literal,$h:literal,$bits:literal,$gray:literal,$tol:literal,$preview:literal,$main:literal) => {
        Case {
            input: include_bytes!(concat!("../fixtures/review/jxl-preview-", $name, ".jxl")),
            rgba: include_bytes!(concat!(
                "../fixtures/review/jxl-preview-",
                $name,
                ".rgba.zlib"
            )),
            #[cfg(feature = "image-buffer-ex")]
            native: include_bytes!(concat!(
                "../fixtures/review/jxl-preview-",
                $name,
                ".rgba16le.zlib"
            )),
            #[cfg(feature = "color-management")]
            managed: include_bytes!(concat!(
                "../fixtures/review/jxl-preview-",
                $name,
                ".srgb.rgba.zlib"
            )),
            size: ($w, $h),
            #[cfg(feature = "image-buffer-ex")]
            bits: $bits,
            #[cfg(feature = "image-buffer-ex")]
            gray: $gray,
            tolerance: $tol,
            preview_start: $preview,
            main_start: $main,
        }
    };
}
const CASES: &[Case] = &[
    case!("rgba8", 7, 5, 8, false, 0, 9, 64),
    case!("icc16", 7, 5, 16, false, 0, 246, 324),
    case!("oriented-icc16", 5, 7, 16, false, 0, 246, 324),
    case!("grayalpha8", 7, 5, 8, true, 0, 11, 47),
    case!("larger", 2, 3, 8, false, 0, 11, 784),
    case!("lossy", 64, 48, 8, false, 2, 9, 169),
];
fn inflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(data).unwrap()
}
fn pixels(actual: &[u8], expected: &[u8], tolerance: u8) {
    assert_eq!(actual.len(), expected.len());
    for (a, b) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
        for c in 0..3 {
            assert!(a[c].abs_diff(b[c]) <= tolerance, "{a:?} != {b:?}");
        }
        assert_eq!(a[3], b[3]);
    }
}
#[test]
fn main_pixels_match_independent_libjxl_even_when_preview_content_size_and_alpha_differ() {
    for case in CASES {
        let image = image_from(case.input).unwrap();
        assert_eq!((image.width, image.height), case.size);
        pixels(
            image.buffer.as_deref().unwrap(),
            &inflate(case.rgba),
            case.tolerance,
        );
        assert!(matches!(
            image.metadata.as_ref().unwrap()["orientation_applied"],
            DataMap::Bool(true)
        ));
        let mut p = Probe::default();
        probe(case.input, &mut p, DecodeLimits::default(), false).unwrap();
        assert_eq!(p.events.iter().filter(|&&e| e == "init").count(), 1);
        assert_eq!(p.events.iter().filter(|&&e| e == "draw").count(), 1);
    }
    let plain = include_bytes!("../fixtures/review/jxl-preview-lossy-main-only.jxl");
    let image = image_from(plain).unwrap();
    pixels(image.buffer.as_deref().unwrap(), &inflate(CASES[5].rgba), 2);
}
#[test]
#[cfg(feature = "image-buffer-ex")]
fn native_preview_skip_preserves_main_depth_gray_alpha_and_orientation() {
    for case in CASES {
        let format = if case.bits == 16 {
            PixelFormatEx::Rgba16
        } else {
            PixelFormatEx::Rgba8
        };
        let image = image_from_ex(case.input, format, PrecisionConversion::Exact).unwrap();
        assert_eq!((image.width(), image.height()), case.size);
        assert_eq!(image.source_format(), Some(format));
        if case.bits == 16 {
            assert_eq!(image.bytes().unwrap(), inflate(case.native));
        } else {
            pixels(image.bytes().unwrap(), &inflate(case.rgba), case.tolerance);
        }
        if case.gray {
            assert!(
                image
                    .bytes()
                    .unwrap()
                    .chunks_exact(4)
                    .all(|p| p[0] == p[1] && p[1] == p[2])
            );
        }
        let mut p = Probe::default();
        probe(case.input, &mut p, DecodeLimits::default(), true).unwrap();
        assert_eq!(p.events.iter().filter(|&&e| e == "init").count(), 1);
        assert_eq!(p.events.iter().filter(|&&e| e == "draw").count(), 1);
    }
}
#[test]
#[cfg(feature = "color-management")]
fn preview_with_embedded_icc_and_orientation_keeps_independent_srgb_main_output() {
    for case in CASES {
        let image = wml2::jpegxl::image_from_srgb(case.input).unwrap();
        assert_eq!((image.width, image.height), case.size);
        pixels(image.buffer.as_deref().unwrap(), &inflate(case.managed), 2);
    }
}
fn rejected(data: &[u8], limits: DecodeLimits) -> String {
    let mut p = Probe::default();
    let e = probe(data, &mut p, limits, false).unwrap_err().to_string();
    assert!(p.events.is_empty(), "{e}: {:?}", p.events);
    e
}
#[test]
fn preview_skip_keeps_canvas_limits_independent_and_never_publishes_rejected_input() {
    let case = &CASES[4]; // 32x32 preview, 2x3 main; never allocate preview pixels.
    let limits = DecodeLimits {
        pixels: 6,
        frames: 1,
        animation_bytes: 24,
        expanded_bytes: 1_120_000,
        ..Default::default()
    };
    let image = image_from_with_limits(case.input, limits).unwrap();
    assert_eq!((image.width, image.height), (2, 3));
    pixels(image.buffer.as_deref().unwrap(), &inflate(case.rgba), 0);
    for limits in [
        DecodeLimits {
            pixels: 5,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            animation_bytes: 23,
            ..Default::default()
        },
        DecodeLimits {
            input_bytes: case.input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1_090_000,
            ..Default::default()
        },
    ] {
        rejected(case.input, limits);
    }
    let oversized = include_bytes!("../fixtures/review/jxl-preview-oversized-header.jxl");
    assert!(rejected(oversized, DecodeLimits::unlimited()).contains("preview"));
}
#[test]
fn preview_headers_toc_payload_extents_and_main_presence_are_checked_before_callbacks() {
    for case in [&CASES[0], &CASES[5]] {
        for n in 0..=case.main_start {
            rejected(&case.input[..n], DecodeLimits::default());
        }
        let mut trailing = case.input.to_vec();
        trailing.push(0);
        rejected(&trailing, DecodeLimits::default());
    }
    let case = &CASES[0];
    let mut oversized_toc = case.input.to_vec();
    // Single-entry TOC: no permutation, byte-aligned u2 selector0 + 10bits1023.
    oversized_toc[case.preview_start + 4..case.preview_start + 6].copy_from_slice(&[0xfc, 0x0f]);
    rejected(&oversized_toc, DecodeLimits::default());
    let mut nonterminal = case.input.to_vec();
    // Toggle this C-encoded Modular frame's is_last field. The parsed regular
    // header then takes the unsafe upstream SkipOutput path, so reject it early.
    nonterminal[case.preview_start + 2] ^= 0x10;
    assert!(rejected(&nonterminal, DecodeLimits::default()).contains("terminal visible"));
}
#[test]
fn skipped_preview_entropy_is_opaque_and_abort_does_not_publish_preview_or_metadata() {
    let case = &CASES[0];
    let mut data = case.input.to_vec();
    data[case.preview_start + 6..case.main_start].fill(0xff);
    let image = image_from(&data).unwrap();
    pixels(image.buffer.as_deref().unwrap(), &inflate(case.rgba), 0);
    for native in [false, true] {
        let mut p = Probe {
            abort: Some("init"),
            ..Default::default()
        };
        probe(case.input, &mut p, DecodeLimits::default(), native).unwrap();
        assert_eq!(p.events, ["init", "end"]);
    }
}
