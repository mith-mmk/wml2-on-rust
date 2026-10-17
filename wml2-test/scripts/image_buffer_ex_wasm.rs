use wml2::draw::*;
use wml2::highres::{ImageDescriptor, ImageFrame, PixelBuffer, Plane};
use wml2::limits::DecodeLimits;
use wml2::util::ImageFormat;
#[unsafe(no_mangle)]
pub extern "C" fn verify_ex() -> u32 {
    let mut count = 0;
    for fmt in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        let max = ((1u32 << fmt.bits()) - 1) as u16;
        let values: Vec<u16> = (0..fmt.channels() * 2)
            .map(|i| {
                if i % fmt.channels() == 3 {
                    0
                } else {
                    [1, max][i / fmt.channels()]
                }
            })
            .collect();
        let bytes: Vec<u8> = if fmt.bits() == 8 {
            values.iter().map(|v| *v as u8).collect()
        } else {
            values.iter().flat_map(|v| v.to_le_bytes()).collect()
        };
        let mut padded = bytes.clone();
        padded.push(0xee);
        let image = ImageBufferEx::from_bytes(2, 1, fmt, Some(bytes.len() + 1), padded).unwrap();
        let bp = fmt.bytes_per_pixel();
        assert_eq!(image.pixel_bytes(0, 0).unwrap(), &bytes[..bp]);
        assert_eq!(image.pixel_bytes(1, 0).unwrap(), &bytes[bp..]);
        assert!(image.pixel_bytes(2, 0).is_err());
        let typed = image.to_highres_pixels().unwrap();
        assert_eq!(
            ImageBufferEx::from_highres_pixels(&typed)
                .unwrap()
                .bytes()
                .unwrap(),
            bytes
        );
        assert_eq!(
            image
                .legacy_picker(PrecisionConversion::Round)
                .encode_pick(0, 0, 2, 1, None)
                .unwrap()
                .unwrap()
                .len(),
            8
        );
        count += 1;
    }
    let mut old = ImageBuffer::new();
    old.init(1, 1, None).unwrap();
    old.draw(0, 0, 1, 1, &[1, 2, 3, 0], None).unwrap();
    let ex = ImageBufferEx::try_from(old).unwrap();
    assert_eq!(ex.pixel_bytes(0, 0).unwrap(), &[1, 2, 3, 0]);
    count += 1;
    let image = ImageBufferEx::from_bytes(
        1,
        1,
        PixelFormatEx::Rgba16,
        None,
        [128u16, 129, 32768, 0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect(),
    )
    .unwrap();
    assert!(
        image
            .legacy_picker(PrecisionConversion::Exact)
            .encode_pick(0, 0, 1, 1, None)
            .is_err()
    );
    let encoded = image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image.legacy_picker(PrecisionConversion::Round),
            options: None,
        },
        ImageFormat::Png,
    )
    .unwrap();
    assert_eq!(
        image_from(&encoded).unwrap().buffer.unwrap(),
        [0, 1, 128, 0]
    );
    let widened =
        image_from_ex(&encoded, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
    assert_eq!(widened.source_format(), Some(PixelFormatEx::Rgba8));
    assert_eq!(widened.bytes().unwrap(), &[0, 0, 1, 1, 128, 128, 0, 0]);
    count += 1;
    let descriptor = ImageDescriptor::gray(2, 1, 16).unwrap();
    let frame = ImageFrame::new(
        descriptor.clone(),
        PixelBuffer::u16(vec![
            Plane::new(descriptor.planes()[0].layout().clone(), vec![1, 65535]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ImageBufferEx::from_highres_pixels(&frame)
            .unwrap()
            .bytes()
            .unwrap(),
        &[1, 0, 255, 255]
    );
    count += 1;
    for (input, bits) in [
        (
            include_bytes!(concat!(env!("WML2_EX_FIXTURES"), "/ex-gray8-lzw.tif")).as_slice(),
            8u8,
        ),
        (
            include_bytes!(concat!(env!("WML2_EX_FIXTURES"), "/ex-gray8-packbits.tif")).as_slice(),
            8,
        ),
        (
            include_bytes!(concat!(env!("WML2_EX_FIXTURES"), "/ex-gray16-lzw.tif")).as_slice(),
            16,
        ),
        (
            include_bytes!(concat!(env!("WML2_EX_FIXTURES"), "/ex-gray16-packbits.tif")).as_slice(),
            16,
        ),
    ] {
        let image =
            image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        let (w, h) = (image.width(), image.height());
        assert_eq!((w, h), (257, 3));
        assert_eq!(
            image.source_format(),
            Some(if bits == 8 {
                PixelFormatEx::Gray8
            } else {
                PixelFormatEx::Gray16
            })
        );
        assert!(
            image_from_ex_with_limits(
                input,
                PixelFormatEx::Rgba16,
                PrecisionConversion::Exact,
                DecodeLimits {
                    pixels: 1,
                    ..DecodeLimits::unlimited()
                }
            )
            .is_err()
        );
        for y in 0..h {
            for x in 0..w {
                let v = if bits == 8 {
                    ((x * 37 + y * 83 + (x / 7) * 11) % 256) as u16 * 257
                } else {
                    ((x * 257 + y * 4660 + (x / 17) * 31) % 65536) as u16
                };
                let b = v.to_le_bytes();
                assert_eq!(
                    image.pixel_bytes(x, y).unwrap(),
                    &[b[0], b[1], b[0], b[1], b[0], b[1], 255, 255]
                );
            }
        }
        count += 1;
    }
    for (input, format, expected) in REVIEW_FILL {
        let image = image_from_ex(input, *format, PrecisionConversion::Exact).unwrap();
        let bytes: Vec<u8> = if format.bits() == 8 {
            expected.iter().map(|v| *v as u8).collect()
        } else {
            expected.iter().flat_map(|v| v.to_le_bytes()).collect()
        };
        assert_eq!(image.bytes().unwrap(), bytes);
        if format.bits() == 16 {
            let native =
                wml2::highres::tiff::decode_native(input, &DecodeLimits::default()).unwrap();
            assert_eq!(
                native.pixels().u16_planes().unwrap()[0].samples(),
                *expected
            );
        }
        count += 1;
    }
    for (input, accept, expected) in REVIEW_SAMPLE_FORMAT {
        let result = image_from_ex(input, PixelFormatEx::Gray16, PrecisionConversion::Exact);
        if *accept {
            let bytes: Vec<u8> = expected.iter().flat_map(|v| v.to_le_bytes()).collect();
            assert_eq!(result.unwrap().bytes().unwrap(), bytes);
        } else {
            assert!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("non-unsigned SampleFormat")
            );
        }
        count += 1;
    }
    for encoder in [
        wml2::encoder::lzw::encode_tiff_libtiff_compat,
        wml2::encoder::lzw::encode_tiff_wml2_lsb,
    ] {
        for depth in [8u8, 16] {
            for be in [false, true] {
                let input = REVIEW_FILL
                    .iter()
                    .find(|(bytes, format, _)| {
                        format.bits() == depth
                            && bytes[0] == if be { b'M' } else { b'I' }
                            && field_value(bytes, 259, be) == 5
                            && field_value(bytes, 266, be) == 2
                            && field_value(bytes, 317, be) == 0
                    })
                    .unwrap();
                let raw: Vec<u8> = if depth == 8 {
                    input.2.iter().map(|v| *v as u8).collect()
                } else {
                    input
                        .2
                        .iter()
                        .flat_map(|v| if be { v.to_be_bytes() } else { v.to_le_bytes() })
                        .collect()
                };
                let compressed = encoder(&raw).unwrap();
                let mut modified = input.0[..field_value(input.0, 273, be) as usize].to_vec();
                let offset = field_offset(&modified, 279, be).unwrap();
                let size = compressed.len() as u32;
                modified[offset + 8..offset + 12].copy_from_slice(&if be {
                    size.to_be_bytes()
                } else {
                    size.to_le_bytes()
                });
                modified.extend(compressed);
                let image = image_from_ex(&modified, input.1, PrecisionConversion::Exact).unwrap();
                let expected: Vec<u8> = if depth == 8 {
                    input.2.iter().map(|v| *v as u8).collect()
                } else {
                    input.2.iter().flat_map(|v| v.to_le_bytes()).collect()
                };
                assert_eq!(image.bytes().unwrap(), expected);
                count += 1;
            }
        }
    }
    let collision = REVIEW_FILL
        .iter()
        .find(|(_, _, v)| *v == [64, 2, 128, 255])
        .unwrap();
    assert!(
        image_from_ex_with_limits(
            collision.0,
            PixelFormatEx::Gray8,
            PrecisionConversion::Exact,
            DecodeLimits {
                expanded_bytes: 8,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
    assert_eq!(
        image_from_ex(
            collision.0,
            PixelFormatEx::Gray8,
            PrecisionConversion::Exact
        )
        .unwrap()
        .bytes()
        .unwrap(),
        [64, 2, 128, 255]
    );
    count += 1;
    count
}

fn word16(data: &[u8], be: bool) -> u16 {
    if be {
        u16::from_be_bytes(data.try_into().unwrap())
    } else {
        u16::from_le_bytes(data.try_into().unwrap())
    }
}
fn field_offset(data: &[u8], tag: u16, be: bool) -> Option<usize> {
    (0..word16(&data[8..10], be) as usize)
        .map(|i| 10 + i * 12)
        .find(|i| word16(&data[*i..*i + 2], be) == tag)
}
fn field_value(data: &[u8], tag: u16, be: bool) -> u32 {
    let Some(i) = field_offset(data, tag, be) else {
        return 0;
    };
    if word16(&data[i + 2..i + 4], be) == 3 {
        word16(&data[i + 8..i + 10], be) as u32
    } else if be {
        u32::from_be_bytes(data[i + 8..i + 12].try_into().unwrap())
    } else {
        u32::from_le_bytes(data[i + 8..i + 12].try_into().unwrap())
    }
}

const REVIEW_FILL: &[(&[u8], PixelFormatEx, &[u16])] = &[
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-raw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-raw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-packbits-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-packbits-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-lzw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-lzw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-lzw-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-lzw-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-deflate-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-deflate-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-le-deflate-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-deflate-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-raw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-raw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-packbits-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-packbits-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-lzw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-lzw-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-lzw-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-lzw-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-deflate-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-deflate-p1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray8-be-deflate-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-be-deflate-p2.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-raw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-raw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-packbits-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-packbits-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-lzw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-lzw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-lzw-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-lzw-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-deflate-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-deflate-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-le-deflate-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-le-deflate-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-raw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-raw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-packbits-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-packbits-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-lzw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-lzw-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-lzw-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-lzw-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-deflate-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-deflate-p1.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill1-gray16-be-deflate-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray16-be-deflate-p2.tif"
        )),
        PixelFormatEx::Gray16,
        &[1, 4660, 32769, 65535],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first1.tif"
        )),
        PixelFormatEx::Gray8,
        &[1, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first64.tif"
        )),
        PixelFormatEx::Gray8,
        &[64, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first65.tif"
        )),
        PixelFormatEx::Gray8,
        &[65, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first127.tif"
        )),
        PixelFormatEx::Gray8,
        &[127, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first128.tif"
        )),
        PixelFormatEx::Gray8,
        &[128, 2, 128, 255],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-fill2-gray8-le-old-lsb-first255.tif"
        )),
        PixelFormatEx::Gray8,
        &[255, 2, 128, 255],
    ),
];
const REVIEW_SAMPLE_FORMAT: &[(&[u8], bool, &[u16])] = &[
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-signed8.tif"
        )),
        false,
        &[],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-signed16.tif"
        )),
        false,
        &[],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-float16.tif"
        )),
        false,
        &[],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-float32.tif"
        )),
        false,
        &[],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-unsigned16le.tif"
        )),
        true,
        &[32768, 65535, 0, 32767],
    ),
    (
        include_bytes!(concat!(
            env!("WML2_EX_FIXTURES"),
            "/ex-review-sampleformat-unsigned16be.tif"
        )),
        true,
        &[32768, 65535, 0, 32767],
    ),
];
