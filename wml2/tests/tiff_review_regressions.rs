#![cfg(feature = "tiff")]

//! Reproductions for the a08bf058 review.  Every byte fixture is built in the
//! test so CI does not rely on a downloaded corpus.

use bin_rs::Endian;
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::collections::HashMap;
use wml2::draw::{EncodeOptions, ImageBuffer, image_encoder, image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
use wml2::tiff::header::{DataPack, TiffHeader, TiffHeaders};
use wml2::util::ImageFormat;

#[derive(Clone)]
struct Spec {
    width: u32,
    height: u32,
    bits: Vec<u16>,
    samples: u16,
    photo: u16,
    compression: u16,
    rows: u32,
    fill_order: u16,
    planar: u16,
    tile: Option<(u32, u32)>,
    blocks: Vec<Vec<u8>>,
    extra: Vec<u16>,
    new_subfile: u32,
    subfile: u16,
    ink_set: Option<u16>,
    number_inks: Option<u16>,
    ink_names: Option<Vec<u8>>,
    color_map: Option<Vec<u16>>,
}

#[derive(Clone)]
struct F {
    tag: u16,
    ty: u16,
    payload: Vec<u8>,
    at: Option<usize>,
}
fn u16b(v: u16, be: bool) -> [u8; 2] {
    if be { v.to_be_bytes() } else { v.to_le_bytes() }
}
fn u32b(v: u32, be: bool) -> [u8; 4] {
    if be { v.to_be_bytes() } else { v.to_le_bytes() }
}
fn s16(v: &[u16], be: bool) -> Vec<u8> {
    v.iter().flat_map(|x| u16b(*x, be)).collect()
}
fn s32(v: &[u32], be: bool) -> Vec<u8> {
    v.iter()
        .flat_map(|x| if be { x.to_be_bytes() } else { x.to_le_bytes() })
        .collect()
}
fn f(tag: u16, ty: u16, payload: Vec<u8>) -> F {
    F {
        tag,
        ty,
        payload,
        at: None,
    }
}
fn type_size(ty: u16) -> usize {
    match ty {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        _ => 8,
    }
}

fn fields(s: &Spec, be: bool, big: bool) -> Vec<F> {
    let ot = if big { 16 } else { 4 };
    let n = s.blocks.len();
    let mut v = vec![
        f(254, 4, u32b(s.new_subfile, be).to_vec()),
        f(255, 3, s16(&[s.subfile], be)),
        f(256, 4, u32b(s.width, be).to_vec()),
        f(257, 4, u32b(s.height, be).to_vec()),
        f(258, 3, s16(&s.bits, be)),
        f(259, 3, s16(&[s.compression], be)),
        f(262, 3, s16(&[s.photo], be)),
        f(266, 3, s16(&[s.fill_order], be)),
        f(273, ot, vec![0; n * if big { 8 } else { 4 }]),
        f(277, 3, s16(&[s.samples], be)),
        f(278, 4, u32b(s.rows, be).to_vec()),
        f(279, ot, vec![0; n * if big { 8 } else { 4 }]),
        f(284, 3, s16(&[s.planar], be)),
        f(317, 3, s16(&[1], be)),
    ];
    if let Some((tw, th)) = s.tile {
        v.retain(|x| !matches!(x.tag, 273 | 278 | 279));
        v.push(f(322, 4, u32b(tw, be).to_vec()));
        v.push(f(323, 4, u32b(th, be).to_vec()));
        v.push(f(324, ot, vec![0; n * if big { 8 } else { 4 }]));
        v.push(f(325, ot, vec![0; n * if big { 8 } else { 4 }]));
    }
    if !s.extra.is_empty() {
        v.push(f(338, 3, s16(&s.extra, be)));
    }
    if let Some(vv) = s.ink_set {
        v.push(f(332, 3, s16(&[vv], be)));
    }
    if let Some(vv) = s.number_inks {
        v.push(f(334, 3, s16(&[vv], be)));
    }
    if let Some(vv) = &s.ink_names {
        v.push(f(333, 2, vv.clone()));
    }
    if let Some(vv) = &s.color_map {
        v.push(f(320, 3, s16(vv, be)));
    }
    v
}

fn write_f(out: &mut Vec<u8>, x: &F, slot: usize, be: bool) {
    out.extend_from_slice(&u16b(x.tag, be));
    out.extend_from_slice(&u16b(x.ty, be));
    let count = (x.payload.len() / type_size(x.ty)) as u32;
    out.extend_from_slice(&u32b(count, be));
    if x.payload.len() <= slot {
        out.extend_from_slice(&x.payload);
        out.resize(out.len() + slot - x.payload.len(), 0);
    } else {
        out.extend_from_slice(&u32b(x.at.unwrap() as u32, be));
    }
}

fn build(specs: &[Spec], big: bool, be: bool) -> Vec<u8> {
    assert!(
        !big,
        "review fixtures use Classic TIFF; BigTIFF coverage is in tiff_extend"
    );
    let mut v = vec![0; 8];
    let mut offsets = Vec::new();
    let mut cursor = 8;
    for s in specs {
        let count = fields(s, be, big).len();
        offsets.push(cursor);
        cursor += 2 + count * 12 + 4;
    }
    v.resize(cursor, 0);
    for (page, &ifd) in specs.iter().zip(&offsets) {
        let mut fs = fields(page, be, big);
        let mut data = v.len();
        for x in &mut fs {
            if x.payload.len() > 4 {
                if data & 1 != 0 {
                    data += 1;
                }
                x.at = Some(data);
                data += x.payload.len();
            }
        }
        let image_at = data;
        let ot = if page.tile.is_some() { 324 } else { 273 };
        let ct = if page.tile.is_some() { 325 } else { 279 };
        let mut offs = Vec::new();
        let mut counts = Vec::new();
        let mut block_at = image_at;
        for block in &page.blocks {
            offs.extend_from_slice(&u32b(block_at as u32, be));
            counts.extend_from_slice(&u32b(block.len() as u32, be));
            block_at += block.len();
        }
        for x in &mut fs {
            if x.tag == ot {
                x.payload = offs.clone();
            }
            if x.tag == ct {
                x.payload = counts.clone();
            }
        }
        if v.len() < block_at {
            v.resize(block_at, 0);
        }
        for x in &fs {
            if let Some(at) = x.at {
                v[at..at + x.payload.len()].copy_from_slice(&x.payload);
            }
        }
        let mut dir = Vec::new();
        dir.extend_from_slice(&u16b(fs.len() as u16, be));
        for x in &fs {
            write_f(&mut dir, x, 4, be);
        }
        let next = offsets
            .get(offsets.iter().position(|&q| q == ifd).unwrap() + 1)
            .copied()
            .unwrap_or(0);
        dir.extend_from_slice(&u32b(next as u32, be));
        v[ifd..ifd + dir.len()].copy_from_slice(&dir);
        let mut at = image_at;
        for block in &page.blocks {
            v[at..at + block.len()].copy_from_slice(block);
            at += block.len();
        }
    }
    v[0..2].copy_from_slice(if be { b"MM" } else { b"II" });
    v[2..4].copy_from_slice(&u16b(42, be));
    v[4..8].copy_from_slice(&u32b(8, be));
    v
}

fn spec(
    width: u32,
    height: u32,
    bits: &[u16],
    samples: u16,
    photo: u16,
    blocks: Vec<Vec<u8>>,
) -> Spec {
    Spec {
        width,
        height,
        bits: bits.to_vec(),
        samples,
        photo,
        compression: 1,
        rows: height,
        fill_order: 1,
        planar: 1,
        tile: None,
        blocks,
        extra: Vec::new(),
        new_subfile: 0,
        subfile: 0,
        ink_set: None,
        number_inks: None,
        ink_names: None,
        color_map: None,
    }
}
fn raw(width: usize, height: usize, channels: usize, value: u8) -> Vec<u8> {
    vec![value; width * height * channels]
}
fn lzw(raw: &[u8]) -> Vec<u8> {
    wml2::encoder::lzw::encode_tiff(raw, false).unwrap()
}
fn packbits(raw: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    for c in raw.chunks(128) {
        v.push((c.len() - 1) as u8);
        v.extend_from_slice(c);
    }
    v
}

#[test]
fn r1_storage_block_expansion_is_limited_for_all_compressions() {
    let data = raw(16, 16, 1, 7);
    for (compression, block) in [
        (1, data.clone()),
        (5, lzw(&data)),
        (32773, packbits(&data)),
        (8, compress_to_vec_zlib(&data, 6)),
        (32946, compress_to_vec_zlib(&data, 6)),
    ] {
        let mut p = spec(1, 1, &[8], 1, 1, vec![block]);
        p.compression = compression;
        p.tile = Some((16, 16));
        let bytes = build(&[p], false, false);
        assert_eq!(image_load(&bytes).unwrap().buffer.unwrap(), [7, 7, 7, 255]);
        let error = image_from_with_limits(
            &bytes,
            DecodeLimits {
                expanded_bytes: 4,
                ..DecodeLimits::default()
            },
        )
        .err()
        .expect("oversized block must be rejected");
        assert!(
            error
                .to_string()
                .contains("expanded TIFF block exceeds decode limit"),
            "compression {compression}: {error}"
        );
    }
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn r1_native_u16_storage_block_expansion_is_limited() {
    let raw = vec![0; 16 * 16 * 2];
    for (compression, block) in [
        (1, raw.clone()),
        (5, lzw(&raw)),
        (32773, packbits(&raw)),
        (8, compress_to_vec_zlib(&raw, 6)),
        (32946, compress_to_vec_zlib(&raw, 6)),
    ] {
        let mut p = spec(1, 1, &[16], 1, 1, vec![block]);
        p.compression = compression;
        p.tile = Some((16, 16));
        let bytes = build(&[p], false, false);
        let frame = wml2::highres::tiff::decode_native(&bytes, &DecodeLimits::default()).unwrap();
        assert_eq!(frame.pixels().u16_planes().unwrap()[0].samples(), &[0]);
        let error = wml2::highres::tiff::decode_native(
            &bytes,
            &DecodeLimits {
                expanded_bytes: 4,
                ..DecodeLimits::default()
            },
        )
        .err()
        .expect("oversized block must be rejected");
        assert!(
            error
                .to_string()
                .contains("expanded TIFF block exceeds decode limit"),
            "compression {compression}: {error}"
        );
    }
}

#[test]
fn r2_opaque_rgba_reencodes_without_stale_four_sample_format() {
    for bigtiff in [false, true] {
        for compression in ["none", "lzw", "deflate"] {
            let mut image = ImageBuffer::from_buffer(1, 1, vec![10, 20, 30, 255]);
            let mut headers = TiffHeaders::empty(Endian::LittleEndian);
            headers.headers.push(TiffHeader {
                tagid: 0x0153,
                data: DataPack::Short(vec![1, 1, 1, 1]),
                length: 4,
            });
            image.metadata = Some(HashMap::from([(
                "Tiff headers".to_string(),
                DataMap::Exif(headers),
            )]));
            let mut options = HashMap::from([(
                "compression".to_string(),
                DataMap::Ascii(compression.to_string()),
            )]);
            if bigtiff {
                options.insert("bigtiff".to_string(), DataMap::UInt(1));
            }
            let mut encode = EncodeOptions {
                debug_flag: 0,
                drawer: &mut image,
                options: Some(options),
            };
            let bytes = image_encoder(&mut encode, ImageFormat::Tiff).unwrap();
            let decoded = image_load(&bytes).unwrap();
            assert_eq!(decoded.buffer.unwrap(), vec![10, 20, 30, 255]);
        }
    }
}

#[test]
fn r3_subfiletype_and_new_subfiletype_pages_are_retained() {
    for (new_subfile, subfile) in [(0, 1), (2, 0)] {
        let first = spec(1, 1, &[8], 1, 1, vec![vec![1]]);
        let mut second = spec(1, 1, &[8], 1, 1, vec![vec![2]]);
        second.subfile = subfile;
        second.new_subfile = new_subfile;
        let image = image_load(&build(&[first, second], false, false)).unwrap();
        assert_eq!(
            image
                .metadata
                .unwrap()
                .get("image pages")
                .unwrap()
                .to_string(),
            "2"
        );
        assert_eq!(image.buffer.as_ref().unwrap(), &vec![1, 1, 1, 255]);
        let frames = image.animation.as_ref().expect("second TIFF page");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].buffer, vec![2, 2, 2, 255]);
    }
}

#[test]
fn r3_first_thumbnail_is_skipped_before_normal_old_subfile_page() {
    let mut thumbnail = spec(1, 1, &[8], 1, 1, vec![vec![9]]);
    thumbnail.new_subfile = 1;
    let normal = spec(1, 1, &[8], 1, 1, vec![vec![2]]);
    let mut old_full = spec(1, 1, &[8], 1, 1, vec![vec![3]]);
    old_full.subfile = 3;
    let image = image_load(&build(&[thumbnail, normal, old_full], false, false)).unwrap();
    assert_eq!(
        image
            .metadata
            .as_ref()
            .unwrap()
            .get("image pages")
            .unwrap()
            .to_string(),
        "2"
    );
    assert_eq!(image.buffer.as_ref().unwrap(), &vec![2, 2, 2, 255]);
    assert_eq!(
        image.animation.as_ref().unwrap()[0].buffer,
        vec![3, 3, 3, 255]
    );
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn r3_native_first_thumbnail_is_skipped_and_two_16bit_pages_are_kept() {
    let mut thumbnail = spec(1, 1, &[16], 1, 1, vec![vec![0x11, 0x11]]);
    thumbnail.new_subfile = 1;
    let normal = spec(1, 1, &[16], 1, 1, vec![vec![0x22, 0x22]]);
    let mut old_full = spec(1, 1, &[16], 1, 1, vec![vec![0x33, 0x33]]);
    old_full.subfile = 3;
    let frames = wml2::highres::tiff::decode_native_pages(
        &build(&[thumbnail, normal, old_full], false, false),
        &DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0].pixels().u16_planes().unwrap()[0].samples(),
        &[0x2222]
    );
    assert_eq!(
        frames[1].pixels().u16_planes().unwrap()[0].samples(),
        &[0x3333]
    );
}

#[test]
fn r4_extra_samples_zero_is_not_transparency() {
    let mut p = spec(1, 1, &[8, 8], 2, 1, vec![vec![128, 0]]);
    p.extra = vec![0];
    let image = image_load(&build(&[p], false, false)).unwrap();
    assert_eq!(image.buffer.unwrap(), vec![128, 128, 128, 255]);
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn r4_native_unknown_extra_channels_are_rejected() {
    let mut gray = spec(1, 1, &[16, 16], 2, 1, vec![vec![0x12, 0x34, 0x00, 0x01]]);
    gray.extra = vec![0];
    assert!(
        wml2::highres::tiff::decode_native(&build(&[gray], false, false), &DecodeLimits::default())
            .is_err()
    );

    let rgb = spec(
        1,
        1,
        &[16, 16, 16, 16],
        4,
        2,
        vec![vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]],
    );
    assert!(
        wml2::highres::tiff::decode_native(&build(&[rgb], false, false), &DecodeLimits::default())
            .is_err()
    );
}

#[test]
fn r5_fill_order_two_four_bit_palette_matches_libtiff_order() {
    let mut cmap = vec![0u16; 16 * 3];
    for (index, value) in [(1usize, 17u16), (2, 34)] {
        cmap[index] = value << 8;
        cmap[16 + index] = value << 8;
        cmap[32 + index] = value << 8;
    }
    let mut p = spec(2, 1, &[4], 1, 3, vec![vec![0x48]]);
    p.fill_order = 2;
    let bytes = build_with_colormap(p, cmap, false);
    assert_eq!(
        image_load(&bytes).unwrap().buffer.unwrap(),
        vec![17, 17, 17, 255, 34, 34, 34, 255]
    );
}

fn build_with_colormap(mut p: Spec, cmap: Vec<u16>, be: bool) -> Vec<u8> {
    p.color_map = Some(cmap);
    build(&[p], false, be)
}

#[test]
fn lzw_code_order_is_independent_of_packed_sample_fill_order() {
    let encoders = [
        wml2::encoder::lzw::encode_tiff_standard,
        wml2::encoder::lzw::encode_tiff_libtiff_compat,
        wml2::encoder::lzw::encode_tiff_wml2_lsb,
    ];
    for encoder in encoders {
        for (fill_order, packed) in [(1, 0x12), (2, 0x48)] {
            let mut cmap = vec![0; 16 * 3];
            cmap[1] = u16::MAX;
            cmap[16 + 2] = u16::MAX;
            let mut p = spec(2, 1, &[4], 1, 3, vec![encoder(&[packed]).unwrap()]);
            p.compression = 5;
            p.fill_order = fill_order;
            let image = image_load(&build_with_colormap(p, cmap, false)).unwrap();
            assert_eq!(image.buffer.unwrap(), [255, 0, 0, 255, 0, 255, 0, 255]);
        }
    }
}

#[test]
fn associated_alpha_is_unassociated_before_high_depth_quantization() {
    let alpha32 = 0x0100_0000u32;
    let rgb32 = [alpha32 / 3, alpha32 * 2 / 3, alpha32, alpha32];
    let gray32 = [alpha32 / 3, alpha32];
    for be in [false, true] {
        for (bits, rgb, gray, expected) in [
            (
                16,
                s16(&[100, 200, 300, 300], be),
                s16(&[100, 300], be),
                [85, 170, 255, 1],
            ),
            (32, s32(&rgb32, be), s32(&gray32, be), [85, 170, 255, 1]),
        ] {
            let mut rgb_page = spec(1, 1, &[bits; 4], 4, 2, vec![rgb]);
            rgb_page.extra = vec![1];
            assert_eq!(
                image_load(&build(&[rgb_page], false, be))
                    .unwrap()
                    .buffer
                    .unwrap(),
                expected,
                "RGB{bits} {:?}",
                if be { "BE" } else { "LE" }
            );

            let mut gray_page = spec(1, 1, &[bits; 2], 2, 1, vec![gray]);
            gray_page.extra = vec![1];
            assert_eq!(
                image_load(&build(&[gray_page], false, be))
                    .unwrap()
                    .buffer
                    .unwrap(),
                [expected[0], expected[0], expected[0], expected[3]],
                "Gray{bits} {:?}",
                if be { "BE" } else { "LE" }
            );
        }
    }
}

#[test]
fn r6_rows_per_strip_u32_max_is_valid_for_small_image() {
    let mut p = spec(1, 2, &[8], 1, 1, vec![vec![4, 5]]);
    p.rows = u32::MAX;
    assert_eq!(
        image_load(&build(&[p], false, false))
            .unwrap()
            .buffer
            .unwrap(),
        vec![4, 4, 4, 255, 5, 5, 5, 255]
    );
}

#[test]
fn inkset_two_is_rejected_as_non_cmyk_separation() {
    let mut p = spec(1, 1, &[8, 8, 8, 8], 4, 5, vec![vec![0, 0, 0, 0]]);
    p.ink_set = Some(2);
    p.number_inks = Some(4);
    p.ink_names = Some(b"Cyan\0Magenta\0Yellow\0Black\0".to_vec());
    assert!(image_load(&build(&[p], false, false)).is_err());
}

#[test]
fn unsupported_color_alpha_is_rejected_before_drawing() {
    for (photo, alpha, channels) in [(3, 1, 2usize), (3, 2, 2), (5, 1, 5)] {
        for bits in [8, 16] {
            for be in [false, true] {
                for planar in [false, true] {
                    for tiled in [false, true] {
                        let pixels = if tiled { 256 } else { 1 };
                        let bytes = pixels * usize::from(bits / 8);
                        let blocks = if planar {
                            vec![vec![0; bytes]; channels]
                        } else {
                            vec![vec![0; bytes * channels]]
                        };
                        let mut p =
                            spec(1, 1, &vec![bits; channels], channels as u16, photo, blocks);
                        p.extra = vec![alpha];
                        p.planar = if planar { 2 } else { 1 };
                        if tiled {
                            p.tile = Some((16, 16));
                        }
                        if photo == 3 {
                            p.color_map = Some(vec![0; 3 * (1usize << bits)]);
                        }
                        let error = image_load(&build(&[p], false, be))
                            .err()
                            .expect("unsupported alpha");
                        assert!(error.to_string().contains("No Support format"), "{error}");
                        assert!(
                            error
                                .to_string()
                                .contains("Palette alpha and associated CMYK alpha"),
                            "{error}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cmyk_unassociated_alpha_survives_strip_tile_and_planar_decoding() {
    let pixels = [
        [255u16, 0, 0, 0, 85],
        [0, 0, 0, 255, 255],
        [0, 0, 0, 0, 0],
        [85, 0, 0, 85, 170],
    ];
    for bits in [8, 16] {
        for be in [false, true] {
            for planar in [false, true] {
                for tiled in [false, true] {
                    let stored = if tiled { 256 } else { pixels.len() };
                    let mut planes = vec![Vec::new(); if planar { 5 } else { 1 }];
                    for index in 0..stored {
                        for channel in 0..5 {
                            let value = pixels.get(index).map_or(0, |p| p[channel]);
                            let output = &mut planes[if planar { channel } else { 0 }];
                            if bits == 16 {
                                output.extend_from_slice(&u16b(value * 257, be));
                            } else {
                                output.push(value as u8);
                            }
                        }
                    }
                    let mut p = spec(4, 1, &[bits; 5], 5, 5, planes);
                    p.extra = vec![2];
                    p.planar = if planar { 2 } else { 1 };
                    if tiled {
                        p.tile = Some((16, 16));
                    }
                    assert_eq!(
                        image_load(&build(&[p], false, be)).unwrap().buffer.unwrap(),
                        [
                            0, 255, 255, 85, 0, 0, 0, 255, 255, 255, 255, 0, 113, 170, 170, 170
                        ]
                    );
                }
            }
        }
    }
}

#[test]
fn palette_unspecified_extra_sample_is_opaque_and_does_not_resize_colormap() {
    for extra in [vec![], vec![0]] {
        let mut p = spec(2, 1, &[8, 8], 2, 3, vec![vec![1, 0, 2, 255]]);
        let mut palette = vec![0; 3 * 256];
        palette[1] = u16::MAX;
        palette[256 + 2] = u16::MAX;
        p.color_map = Some(palette);
        p.extra = extra;
        assert_eq!(
            image_load(&build(&[p], false, false))
                .unwrap()
                .buffer
                .unwrap(),
            [255, 0, 0, 255, 0, 255, 0, 255]
        );
    }
}

#[test]
fn borrowed_raw_drawing_restores_predictor_without_modifying_caller_bytes() {
    for planar in [false, true] {
        let samples = if planar {
            [0xff, 1, 0x1ff, 1, 0x2ff, 1]
        } else {
            [0xff, 0x1ff, 0x2ff, 1, 1, 1]
        };
        let data = s16(&samples, false);
        let original = data.clone();
        let mut header = wml2::tiff::header::Tiff::empty();
        header.width = 2;
        header.height = 1;
        header.samples_per_pixel = 3;
        header.bitspersamples = vec![16; 3];
        header.bitspersample = 48;
        header.photometric_interpretation = 2;
        header.predictor = 2;
        header.planar_config = if planar { 2 } else { 1 };
        header.tiff_headers.endian = Endian::LittleEndian;
        let mut image = ImageBuffer::from_buffer(2, 1, vec![0; 8]);
        let mut options = wml2::draw::DecodeOptions {
            debug_flag: 0,
            drawer: &mut image,
        };
        wml2::tiff::decoder::draw_tile(&data, 0, 1, 0, 2, &mut options, &header).unwrap();
        assert_eq!(data, original);
        assert_eq!(header.predictor, 2);
        assert_eq!(image.buffer.unwrap(), [0, 1, 2, 255, 1, 2, 3, 255]);
    }
}
