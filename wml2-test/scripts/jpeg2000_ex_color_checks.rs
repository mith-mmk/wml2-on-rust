//! Public-API colour assertions shared by native tests and actual WASM.
use crate::checks::{Probe, decode};
use wml2::draw::*;
use wml2::limits::DecodeLimits;

pub fn format(bits: u8) -> PixelFormatEx {
    match bits {
        12 => PixelFormatEx::Rgba12,
        16 => PixelFormatEx::Rgba16,
        _ => panic!("test depth"),
    }
}

pub fn pixels(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    let fmt = format(bits);
    let mut image = image_from_ex(data, fmt, PrecisionConversion::Exact).unwrap();
    assert_eq!(
        (image.width(), image.height(), image.stride()),
        (w, h, w * 8)
    );
    assert_eq!(image.source_format(), Some(fmt));
    assert_eq!(image.source_stride(), None);
    assert_eq!(image.bytes().unwrap(), expected);
    let stride = (w - 1) * 8 + 3;
    let pick = image
        .encode_pick_ex(
            1,
            0,
            w - 1,
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
        let row = &pick.bytes()[y * stride..(y + 1) * stride];
        assert_eq!(
            &row[..(w - 1) * 8],
            &expected[(y * w + 1) * 8..(y + 1) * w * 8]
        );
        assert_eq!(&row[(w - 1) * 8..], &[0; 3]);
    }
    assert_eq!(image.bytes().unwrap(), expected);
    assert!(image_load(data).is_err());
}

pub fn callbacks_and_limits(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    for abort in [0, 1, 2, 3] {
        let mut p = Probe::with_format(format(bits));
        p.abort = abort;
        decode(data, &mut p, DecodeLimits::default()).unwrap();
        assert_eq!(
            p.events,
            if abort == 1 {
                vec![1, 3]
            } else {
                vec![1, 2, 3]
            }
        );
        if abort != 1 {
            assert_eq!(p.image.bytes().unwrap(), expected);
        }
    }
    for fail in [1, 2, 3] {
        let mut p = Probe::with_format(format(bits));
        p.fail = fail;
        assert!(
            decode(data, &mut p, DecodeLimits::default())
                .unwrap_err()
                .to_string()
                .contains("probe callback error")
        );
        assert_eq!(p.events, (1..=fail).collect::<Vec<_>>());
    }
    for field in 0..6 {
        let mut limits = DecodeLimits::default();
        match field {
            0 => limits.input_bytes = data.len() - 1,
            1 => limits.pixels = w * h - 1,
            2 => limits.expanded_bytes = expected.len() - 1,
            3 => limits.frames = 0,
            4 => limits.metadata_bytes = 0,
            _ => limits.animation_bytes = expected.len() - 1,
        }
        let mut p = Probe::with_format(format(bits));
        assert!(decode(data, &mut p, limits).is_err(), "budget {field}");
        assert!(p.events.is_empty());
    }
    let (mut lo, mut hi) = (0, 16 * 1024 * 1024);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let mut p = Probe::with_format(format(bits));
        if decode(
            data,
            &mut p,
            DecodeLimits {
                expanded_bytes: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            hi = mid;
        } else {
            assert!(p.events.is_empty());
            lo = mid + 1;
        }
    }
    assert!(lo > expected.len());
    for budget in [lo - 1, lo] {
        let mut p = Probe::with_format(format(bits));
        let result = decode(
            data,
            &mut p,
            DecodeLimits {
                expanded_bytes: budget,
                ..Default::default()
            },
        );
        if budget == lo {
            result.unwrap();
            assert_eq!(p.image.bytes().unwrap(), expected);
        } else {
            assert!(result.is_err());
            assert!(p.events.is_empty());
        }
    }
    pixels(data, expected, w, h, bits);
}

pub fn origins_and_geometry(data: &[u8], expected: &[u8], w: usize, h: usize, bits: u8) {
    let siz = data.windows(2).position(|v| v == [255, 0x51]).unwrap();
    for origin in [0x8000_0000u32, 0xffff_ff80] {
        let mut shifted = data.to_vec();
        for offset in [siz + 6, siz + 14, siz + 30] {
            let v = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
            shifted[offset..offset + 4]
                .copy_from_slice(&v.checked_add(origin).unwrap().to_be_bytes());
        }
        pixels(&shifted, expected, w, h, bits);
    }
    let mut huge = data.to_vec();
    for offset in [siz + 6, siz + 10] {
        huge[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
    }
    let mut p = Probe::with_format(format(bits));
    assert!(decode(&huge, &mut p, DecodeLimits::unlimited()).is_err());
    assert!(p.events.is_empty());
}
