//! Owned JPEG2000 output compared with the unchanged public callback path.
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
type Error = Box<dyn std::error::Error>;

pub fn format(bits: u8, components: usize) -> PixelFormatEx {
    match (bits, components) {
        (12, 1) => PixelFormatEx::Gray12,
        (16, 1) => PixelFormatEx::Gray16,
        (12, _) => PixelFormatEx::Rgba12,
        (16, _) => PixelFormatEx::Rgba16,
        _ => panic!("fixture depth"),
    }
}

pub fn callback(
    data: &[u8],
    storage: PixelFormatEx,
    policy: PrecisionConversion,
    limits: DecodeLimits,
) -> Result<ImageBufferEx, Error> {
    let mut image = ImageBufferEx::with_storage(storage, policy);
    image_decoder_ex_with_limits(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        },
        limits,
    )?;
    Ok(image)
}

pub fn compare(
    data: &[u8],
    storage: PixelFormatEx,
    policy: PrecisionConversion,
    limits: DecodeLimits,
) {
    let owned = image_from_ex_with_limits(data, storage, policy, limits);
    let reference = callback(data, storage, policy, limits);
    match (owned, reference) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                (a.width(), a.height(), a.format(), a.stride()),
                (b.width(), b.height(), b.format(), b.stride())
            );
            assert_eq!(a.bytes(), b.bytes());
            assert_eq!(a.source_format(), b.source_format());
            assert_eq!(a.source_stride(), b.source_stride());
            assert_eq!(a.precision_conversion(), policy);
            assert_eq!(a.precision_conversion(), b.precision_conversion());
            assert!(a.metadata.is_none() && b.metadata.is_none());
        }
        (Err(a), Err(b)) => assert_eq!(a.to_string(), b.to_string()),
        (a, b) => panic!(
            "owned/callback acceptance differs: owned={}, callback={}",
            a.is_ok(),
            b.is_ok()
        ),
    }
}

pub fn native(data: &[u8], bits: u8, components: usize) {
    for policy in [PrecisionConversion::Exact, PrecisionConversion::Round] {
        compare(
            data,
            format(bits, components),
            policy,
            DecodeLimits::default(),
        );
    }
}

pub fn conversions_and_limits(data: &[u8], bits: u8, components: usize, w: usize, h: usize) {
    for storage in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        for policy in [PrecisionConversion::Exact, PrecisionConversion::Round] {
            compare(data, storage, policy, DecodeLimits::default());
            for field in 0..6 {
                let mut limits = DecodeLimits::default();
                match field {
                    0 => limits.input_bytes = data.len() - 1,
                    1 => limits.pixels = w * h - 1,
                    2 => {
                        limits.expanded_bytes =
                            w * h * format(bits, components).bytes_per_pixel() - 1
                    }
                    3 => limits.frames = 0,
                    4 => limits.metadata_bytes = 0,
                    _ => {
                        limits.animation_bytes =
                            w * h * format(bits, components).bytes_per_pixel() - 1
                    }
                }
                compare(data, storage, policy, limits);
            }
        }
    }
    let storage = format(bits, components);
    let (mut lo, mut hi) = (0, 16 * 1024 * 1024);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if callback(
            data,
            storage,
            PrecisionConversion::Exact,
            DecodeLimits {
                expanded_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    for n in [lo - 1, lo] {
        let limits = DecodeLimits {
            expanded_bytes: n,
            ..Default::default()
        };
        let result = image_from_ex_with_limits(data, storage, PrecisionConversion::Exact, limits);
        assert_eq!(result.is_ok(), n == lo);
        compare(data, storage, PrecisionConversion::Exact, limits);
    }
    // A failed inner metadata scope must not poison a subsequent decode.
    native(data, bits, components);
}

pub fn origins(data: &[u8], bits: u8, components: usize) {
    let storage = format(bits, components);
    let expected = image_from_ex(data, storage, PrecisionConversion::Exact).unwrap();
    let siz = data.windows(2).position(|v| v == [255, 0x51]).unwrap();
    for origin in [0x8000_0000u32, 0xffff_ff80] {
        let mut shifted = data.to_vec();
        for offset in [siz + 6, siz + 14, siz + 30] {
            let value = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
            shifted[offset..offset + 4]
                .copy_from_slice(&value.checked_add(origin).unwrap().to_be_bytes());
        }
        let got = image_from_ex(&shifted, storage, PrecisionConversion::Exact).unwrap();
        assert_eq!(got.bytes(), expected.bytes());
        native(&shifted, bits, components);
    }
    let mut huge = data.to_vec();
    for offset in [siz + 6, siz + 10] {
        huge[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    }
    assert!(
        image_from_ex_with_limits(
            &huge,
            storage,
            PrecisionConversion::Exact,
            DecodeLimits::unlimited()
        )
        .is_err()
    );
    compare(
        &huge,
        storage,
        PrecisionConversion::Exact,
        DecodeLimits::unlimited(),
    );
}
