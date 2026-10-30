#![cfg(feature = "jpeg-xl")]
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
type Error = Box<dyn std::error::Error>;
#[derive(Clone, Copy, Debug, Default)]
struct Stats {
    count: usize,
    total: usize,
    largest: usize,
}
struct Probe;
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static STATS:Cell<Stats>=const{Cell::new(Stats{count:0,total:0,largest:0})};}
fn allocated(n: usize) {
    TRACK.with(|t| {
        if t.get() {
            STATS.with(|s| {
                let mut v = s.get();
                v.count += 1;
                v.total += n;
                v.largest = v.largest.max(n);
                s.set(v);
            });
        }
    });
}
// SAFETY: forward exact layouts/pointers to System; counters do not allocate.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            allocated(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            allocated(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, l, n) };
        if !next.is_null() {
            allocated(n);
        }
        next
    }
}
#[global_allocator]
static ALLOC: Probe = Probe;
fn measure<T>(body: impl FnOnce() -> T) -> (T, Stats) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TRACK.set(false);
        }
    }
    STATS.set(Stats::default());
    TRACK.set(true);
    let _reset = Reset;
    let result = body();
    (result, STATS.get())
}
fn oversized(container: bool) -> Vec<u8> {
    let mut bytes = vec![0; 64 * 1024 * 1024 + 1];
    let signature: &[u8] = if container {
        b"\0\0\0\x0cJXL \r\n\x87\n"
    } else {
        &[255, 10]
    };
    bytes[..signature.len()].copy_from_slice(signature);
    bytes
}
fn bounded_refusal<T>(name: &str, body: impl FnOnce() -> Result<T, Error>) {
    let (result, stats) = measure(body);
    let error = result.err().expect("oversized input must reject");
    assert_eq!(
        error.to_string(),
        "Out of memory: JPEG XL input exceeds decode limit",
        "{name}"
    );
    eprintln!("{name}: {stats:?}");
    assert!(
        stats.largest < 4096 && stats.total < 8192 && stats.count < 32,
        "{name} allocated before admission: {stats:?}"
    );
}
#[test]
fn ordinary_slice_entrypoints_reject_without_input_sized_allocation() {
    for container in [false, true] {
        let bytes = oversized(container);
        bounded_refusal("ordinary with limits", || {
            image_from_with_limits(&bytes, DecodeLimits::default())
        });
        bounded_refusal("image_from", || image_from(&bytes));
        bounded_refusal("image_load", || image_load(&bytes));
        let mut image = ImageBuffer::new();
        bounded_refusal("image_loader", || {
            image_loader(
                &bytes,
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,
                },
            )
        });
        assert_eq!((image.width, image.height), (0, 0));
        assert!(image.buffer.is_none() && image.metadata.is_none());
    }
}
#[cfg(feature = "image-buffer-ex")]
#[test]
fn ex_slice_entrypoints_reject_without_input_sized_allocation() {
    for container in [false, true] {
        let bytes = oversized(container);
        bounded_refusal("EX with limits", || {
            image_from_ex_with_limits(
                &bytes,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Round,
                DecodeLimits::default(),
            )
        });
        bounded_refusal("EX default", || {
            image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Round)
        });
    }
}
#[cfg(feature = "color-management")]
#[test]
fn managed_slice_entrypoints_reject_without_input_sized_allocation() {
    for container in [false, true] {
        let bytes = oversized(container);
        bounded_refusal("managed with limits", || {
            wml2::jpegxl::image_from_srgb_with_limits(&bytes, DecodeLimits::default())
        });
        bounded_refusal("managed default", || wml2::jpegxl::image_from_srgb(&bytes));
    }
}

fn old_owned(bytes: &[u8], limits: DecodeLimits) -> Result<ImageBuffer, Error> {
    let mut image = ImageBuffer::new();
    image_decoder_with_limits(
        &mut bin_rs::reader::BytesReader::new(bytes),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
    )?;
    Ok(image)
}
fn outcome<T>(result: Result<T, Error>) -> Option<String> {
    result.err().map(|e| e.to_string())
}
fn metadata(image: &ImageBuffer) -> Vec<(String, String)> {
    let mut entries: Vec<_> = image
        .metadata
        .as_ref()
        .into_iter()
        .flat_map(|map| map.iter())
        .map(|(k, v)| (k.clone(), format!("{v:?}")))
        .collect();
    entries.sort();
    entries
}
const RGB: &[u8] = include_bytes!("fixtures/review/jxl-rgb8.jxl");
#[test]
fn valid_pixels_metadata_orientation_and_legacy_loader_match_owned_reader() {
    for bytes in [
        RGB,
        include_bytes!("fixtures/review/jxl-rgba16.jxl"),
        include_bytes!("fixtures/review/jxl-icc-lossless.jxl"),
        include_bytes!("fixtures/review/jxl-orientation6.jxl"),
        include_bytes!("fixtures/review/jxl-sunset-logo.jxl"),
    ] {
        let old = old_owned(bytes, DecodeLimits::default()).unwrap();
        let got = image_from(bytes).unwrap();
        assert_eq!((got.width, got.height), (old.width, old.height));
        assert_eq!(got.buffer, old.buffer);
        assert_eq!(metadata(&got), metadata(&old));
        let mut loaded = ImageBuffer::new();
        image_loader(
            bytes,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut loaded,
            },
        )
        .unwrap();
        assert_eq!(loaded.buffer, old.buffer);
        assert_eq!(metadata(&loaded), metadata(&old));
        #[cfg(feature = "image-buffer-ex")]
        {
            let mut old =
                ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
            image_decoder_ex_with_limits(
                &mut bin_rs::reader::BytesReader::new(bytes),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut old,
                },
                DecodeLimits::default(),
            )
            .unwrap();
            let got =
                image_from_ex(bytes, PixelFormatEx::Rgba16, PrecisionConversion::Round).unwrap();
            assert_eq!(got.bytes(), old.bytes());
            assert_eq!(got.source_format(), old.source_format());
            assert_eq!((got.width(), got.height()), (old.width(), old.height()));
        }
    }
}
#[test]
fn malformed_signatures_truncation_and_all_budget_errors_preserve_precedence() {
    let base = DecodeLimits::default();
    let budgets = [
        base,
        DecodeLimits {
            input_bytes: 1,
            ..base
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..base
        },
        DecodeLimits { pixels: 0, ..base },
        DecodeLimits { frames: 0, ..base },
        DecodeLimits {
            metadata_bytes: 1,
            ..base
        },
        DecodeLimits {
            animation_bytes: 1,
            ..base
        },
    ];
    for end in 0..RGB.len() {
        let bytes = &RGB[..end];
        for budget in budgets {
            assert_eq!(
                outcome(image_from_with_limits(bytes, budget)),
                outcome(old_owned(bytes, budget)),
                "prefix {end}"
            );
        }
    }
    let signature = b"\0\0\0\x0cJXL \r\n\x87\n";
    for end in 0..=signature.len() {
        let bytes = &signature[..end];
        for budget in budgets {
            assert_eq!(
                outcome(image_from_with_limits(bytes, budget)),
                outcome(old_owned(bytes, budget)),
                "container prefix {end}"
            );
        }
    }
    for i in 0..signature.len() {
        let mut bytes = signature.to_vec();
        bytes[i] ^= 1;
        assert_eq!(
            outcome(image_from(&bytes)),
            outcome(old_owned(&bytes, base))
        );
    }
    #[cfg(feature = "image-buffer-ex")]
    for end in 0..RGB.len() {
        let bytes = &RGB[..end];
        let mut old =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
        let result = image_decoder_ex_with_limits(
            &mut bin_rs::reader::BytesReader::new(bytes),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut old,
            },
            base,
        );
        assert_eq!(
            outcome(image_from_ex_with_limits(
                bytes,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Round,
                base
            )),
            outcome(result)
        );
    }
    #[cfg(feature = "color-management")]
    for (budget, message) in [
        (budgets[1], "input"),
        (budgets[2], "JPEG XL input assembly workspace"),
        (budgets[3], "JPEG XL pixels"),
        (budgets[4], "JPEG XL still frames"),
        (budgets[5], "JPEG XL source metadata"),
        (budgets[6], "JPEG XL canvas"),
    ] {
        assert_eq!(
            outcome(wml2::jpegxl::image_from_srgb_with_limits(RGB, budget)),
            Some(format!("Out of memory: {message} exceeds decode limit"))
        );
    }
    // Gray ICC refusal and normalized Exact precision behavior remain intact.
    assert!(
        outcome(image_from(include_bytes!(
            "fixtures/review/jxl-icc-gray16.jxl"
        )))
        .unwrap()
        .contains("Gray ICC expansion")
    );
    #[cfg(feature = "image-buffer-ex")]
    assert!(
        image_from_ex(
            include_bytes!("fixtures/review/jxl-rgba12.jxl"),
            PixelFormatEx::Rgba16,
            PrecisionConversion::Exact
        )
        .is_err()
    );
}
#[test]
fn caller_input_and_workspace_refusals_also_precede_input_sized_allocations() {
    let mut bytes = vec![0; 8 * 1024 * 1024];
    bytes[..2].copy_from_slice(&[255, 10]);
    for (budget, message) in [
        (
            DecodeLimits {
                input_bytes: 1,
                ..Default::default()
            },
            "input",
        ),
        (
            DecodeLimits {
                expanded_bytes: 1,
                ..Default::default()
            },
            "JPEG XL input assembly workspace",
        ),
    ] {
        let (result, stats) = measure(|| image_from_with_limits(&bytes, budget));
        assert_eq!(
            outcome(result),
            Some(format!("Out of memory: {message} exceeds decode limit"))
        );
        assert!(stats.largest < 4096 && stats.total < 8192, "{stats:?}");
        #[cfg(feature = "image-buffer-ex")]
        {
            let (result, stats) = measure(|| {
                image_from_ex_with_limits(
                    &bytes,
                    PixelFormatEx::Rgba8,
                    PrecisionConversion::Round,
                    budget,
                )
            });
            assert_eq!(
                outcome(result),
                Some(format!("Out of memory: {message} exceeds decode limit"))
            );
            assert!(stats.largest < 4096 && stats.total < 8192, "{stats:?}");
        }
        #[cfg(feature = "color-management")]
        {
            let (result, stats) =
                measure(|| wml2::jpegxl::image_from_srgb_with_limits(&bytes, budget));
            assert_eq!(
                outcome(result),
                Some(format!("Out of memory: {message} exceeds decode limit"))
            );
            assert!(stats.largest < 4096 && stats.total < 8192, "{stats:?}");
        }
    }
}
#[cfg(feature = "png")]
#[test]
fn non_jxl_slice_paths_match_existing_owned_reader() {
    let mut source = ImageBuffer::from_buffer(2, 1, vec![1, 2, 3, 255, 77, 88, 99, 128]);
    let bytes = image_to(&mut source, wml2::util::ImageFormat::Png, None).unwrap();
    for limits in [
        DecodeLimits::default(),
        DecodeLimits {
            input_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 1,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..Default::default()
        },
    ] {
        let old = old_owned(&bytes, limits);
        let got = image_from_with_limits(&bytes, limits);
        match (old, got) {
            (Ok(old), Ok(got)) => assert_eq!(old.buffer, got.buffer),
            (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
            _ => panic!("non-JXL outcome changed"),
        }
    }
}

#[cfg(feature = "color-management")]
#[test]
fn managed_oversized_invalid_signature_keeps_limit_precedence_without_copy() {
    let bytes = vec![0; 64 * 1024 * 1024 + 1];
    bounded_refusal("managed oversized invalid signature", || {
        wml2::jpegxl::image_from_srgb(&bytes)
    });
}
#[test]
fn borrowed_input_is_not_retained_by_returned_image() {
    let mut bytes = RGB.to_vec();
    let image = image_from(&bytes).unwrap();
    let expected = old_owned(RGB, DecodeLimits::default()).unwrap();
    bytes.fill(0);
    drop(bytes);
    assert_eq!(image.buffer, expected.buffer);
    assert_eq!(metadata(&image), metadata(&expected));
}

#[cfg(feature = "color-management")]
#[test]
fn composition_cancellation_stays_before_borrowed_input_admission() {
    let input = oversized(true);
    let limits = DecodeLimits {
        input_bytes: 0,
        pixels: 0,
        expanded_bytes: 0,
        metadata_bytes: 0,
        frames: 0,
        animation_bytes: 0,
    };
    let (result, stats) =
        measure(|| wml2::jpegxl::image_from_srgb_with_limits_and_cancel(&input, limits, &|| true));
    assert!(result.err().unwrap().is::<wml2::jpegxl::DecodeCancelled>());
    assert_eq!(stats.count, 0, "pre-cancel allocations: {stats:?}");
    bounded_refusal("composed managed non-cancel", || {
        wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
            &input,
            DecodeLimits::default(),
            &|| false,
        )
    });
    let mixed = include_bytes!("fixtures/review/jxl-alpha-icc-c8a16o6.jxl");
    let image = wml2::jpegxl::image_from_srgb_with_limits_and_cancel(
        mixed,
        DecodeLimits::default(),
        &|| false,
    )
    .unwrap();
    let expected = include_bytes!("fixtures/review/jxl-alpha-icc-c8a16o6.srgb.rgba");
    for (actual, reference) in image
        .buffer
        .unwrap()
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
    {
        for c in 0..3 {
            assert!(actual[c].abs_diff(reference[c]) <= 2);
        }
        assert_eq!(actual[3], reference[3]);
    }
}
