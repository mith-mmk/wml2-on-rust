//! Lossy reference comparison is separate from exact storage/callback checks.
use wml2::draw::*;

pub fn format(bits: u8, components: usize) -> PixelFormatEx {
    if components == 1 {
        crate::checks::format(bits)
    } else {
        crate::color::format(bits)
    }
}

pub fn pixels(
    data: &[u8],
    reference: &[u8],
    w: usize,
    h: usize,
    bits: u8,
    components: usize,
) -> Vec<u8> {
    let fmt = format(bits, components);
    let mut image = image_from_ex(data, fmt, PrecisionConversion::Exact).unwrap();
    let channels = if components == 1 { 1 } else { 4 };
    assert_eq!(
        (image.width(), image.height(), image.stride()),
        (w, h, w * channels * 2)
    );
    assert_eq!(image.source_format(), Some(fmt));
    assert_eq!(image.source_stride(), None);
    let actual = image.bytes().unwrap().to_vec();
    assert_eq!(actual.len(), reference.len());
    // OpenJPEG's historical high-band normalization differs from Table F.4.
    // These source-domain caps are independently checked against FFmpeg too.
    // They compare decoders, not the lossy decoded image with its source.
    let cap = if bits == 12 { 1 } else { 4 };
    let mut total = 0usize;
    for (a, b) in actual.chunks_exact(2).zip(reference.chunks_exact(2)) {
        let a = u16::from_le_bytes([a[0], a[1]]);
        let b = u16::from_le_bytes([b[0], b[1]]);
        assert!(u32::from(a) < 1u32 << bits);
        let error = a.abs_diff(b);
        assert!(error <= cap, "decoder difference {error} exceeds {cap}");
        total += usize::from(error);
    }
    let mean = total as f64 / (actual.len() / 2) as f64;
    assert!(
        mean <= if bits == 12 { 0.1 } else { 1.25 },
        "mean decoder difference {mean}"
    );
    if components == 3 {
        for pixel in actual.chunks_exact(8) {
            assert_eq!(
                u16::from_le_bytes([pixel[6], pixel[7]]),
                ((1u32 << bits) - 1) as u16
            );
        }
    }
    // Exercise odd byte strides independently of tolerated numerical error.
    let row_bytes = w * channels * 2;
    let stride = row_bytes + 3;
    let picked = image
        .encode_pick_ex(
            0,
            0,
            w,
            h,
            PixelOutputEx {
                format: fmt,
                conversion: PrecisionConversion::Exact,
                stride: Some(stride),
            },
            None,
        )
        .unwrap()
        .unwrap();
    for y in 0..h {
        assert_eq!(
            &picked.bytes()[y * stride..y * stride + row_bytes],
            &actual[y * row_bytes..(y + 1) * row_bytes]
        );
        assert_eq!(
            &picked.bytes()[y * stride + row_bytes..(y + 1) * stride],
            &[0; 3]
        );
    }
    assert_eq!(image.bytes().unwrap(), actual);
    if w > 1 {
        if components == 1 {
            crate::checks::pixels(data, &actual, w, h, bits);
        } else {
            crate::color::pixels(data, &actual, w, h, bits);
        }
    }
    assert!(image_load(data).is_err());
    actual
}

pub fn extra(data: &[u8], actual: &[u8], w: usize, h: usize, bits: u8, components: usize) {
    if components == 1 {
        crate::checks::callbacks_and_limits(data, actual, w, h, bits);
        crate::checks::geometry_boundaries(data, actual, w, h, bits);
        crate::checks::large_x_origin_pixels(data, actual, w, h, bits);
    } else {
        crate::color::callbacks_and_limits(data, actual, w, h, bits);
        crate::color::origins_and_geometry(data, actual, w, h, bits);
    }
    let fmt = if components == 1 {
        PixelFormatEx::Gray8
    } else {
        PixelFormatEx::Rgba8
    };
    assert!(image_from_ex(data, fmt, PrecisionConversion::Exact).is_err());
    let reduced = image_from_ex(data, fmt, PrecisionConversion::Round).unwrap();
    let max = (1u32 << bits) - 1;
    let expected: Vec<_> = actual
        .chunks_exact(2)
        .map(|v| ((u32::from(u16::from_le_bytes([v[0], v[1]])) * 255 + max / 2) / max) as u8)
        .collect();
    assert_eq!(reduced.bytes().unwrap(), expected);
}
