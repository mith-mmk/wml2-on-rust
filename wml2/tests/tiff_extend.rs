#![cfg(feature = "tiff")]

//! Focused TIFF extension fixtures.  These are deliberately generated here so
//! the decoder tests do not depend on a third party TIFF writer.

use std::io::{self, BufReader, Read, Seek, SeekFrom};

use bin_rs::reader::StreamReader;
use miniz_oxide::deflate::compress_to_vec_zlib;
use wml2::draw::{image_decoder_with_limits, image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;

#[derive(Clone, Copy)]
enum Variant {
    Classic,
    Big,
}

#[derive(Clone)]
struct Page {
    width: u32,
    height: u32,
    bits: Vec<u16>,
    samples: u16,
    photo: u16,
    rows: u32,
    planar: u16,
    predictor: u16,
    compression: u16,
    tile: Option<(u32, u32)>,
    blocks: Vec<Vec<u8>>,
    extra: Vec<u16>,
    orientation: u16,
    icc: Vec<u8>,
}

#[derive(Clone)]
struct Field {
    tag: u16,
    ty: u16,
    payload: Vec<u8>,
    at: Option<usize>,
}

fn put_u16(out: &mut Vec<u8>, value: u16, be: bool) {
    out.extend(if be {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    });
}
fn put_u32(out: &mut Vec<u8>, value: u32, be: bool) {
    out.extend(if be {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    });
}
fn put_u64(out: &mut Vec<u8>, value: u64, be: bool) {
    out.extend(if be {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    });
}
fn bytes_u16(values: &[u16], be: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 2);
    for &value in values {
        put_u16(&mut out, value, be);
    }
    out
}
fn bytes_u32(values: &[u32], be: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for &value in values {
        put_u32(&mut out, value, be);
    }
    out
}

fn compression(raw: &[u8], mode: u16) -> Vec<u8> {
    match mode {
        1 => raw.to_vec(),
        5 => wml2::encoder::lzw::encode_tiff(raw, false).unwrap(),
        8 | 32946 => compress_to_vec_zlib(raw, 6),
        32773 => {
            let mut out = Vec::with_capacity(raw.len() + raw.len() / 128 + 1);
            for chunk in raw.chunks(128) {
                out.push((chunk.len() - 1) as u8);
                out.extend_from_slice(chunk);
            }
            out
        }
        _ => panic!("fixture compression {mode} is not implemented"),
    }
}

fn field(tag: u16, ty: u16, payload: Vec<u8>) -> Field {
    Field {
        tag,
        ty,
        payload,
        at: None,
    }
}

fn page_fields(page: &Page, be: bool, variant: Variant) -> Vec<Field> {
    let big = matches!(variant, Variant::Big);
    let offset_ty = if big { 16 } else { 4 };
    let offsets_len = page.blocks.len();
    let mut fields = vec![
        field(256, 4, bytes_u32(&[page.width], be)),
        field(257, 4, bytes_u32(&[page.height], be)),
        field(258, 3, bytes_u16(&page.bits, be)),
        field(259, 3, bytes_u16(&[page.compression], be)),
        field(262, 3, bytes_u16(&[page.photo], be)),
        field(274, 3, bytes_u16(&[page.orientation], be)),
        field(
            273,
            offset_ty,
            vec![0; offsets_len * if big { 8 } else { 4 }],
        ),
        field(277, 3, bytes_u16(&[page.samples], be)),
        field(
            278,
            4,
            bytes_u32(&[page.tile.map_or(page.rows, |_| page.tile.unwrap().1)], be),
        ),
        field(
            279,
            offset_ty,
            vec![0; offsets_len * if big { 8 } else { 4 }],
        ),
        field(284, 3, bytes_u16(&[page.planar], be)),
        field(317, 3, bytes_u16(&[page.predictor], be)),
    ];
    if let Some((tile_width, tile_height)) = page.tile {
        fields.retain(|f| f.tag != 273 && f.tag != 278 && f.tag != 279);
        fields.push(field(322, 4, bytes_u32(&[tile_width], be)));
        fields.push(field(323, 4, bytes_u32(&[tile_height], be)));
        fields.push(field(
            324,
            offset_ty,
            vec![0; offsets_len * if big { 8 } else { 4 }],
        ));
        fields.push(field(
            325,
            offset_ty,
            vec![0; offsets_len * if big { 8 } else { 4 }],
        ));
    }
    if !page.extra.is_empty() {
        fields.push(field(338, 3, bytes_u16(&page.extra, be)));
    }
    if !page.icc.is_empty() {
        fields.push(field(34675, 7, page.icc.clone()));
    }
    fields
}

fn write_field(out: &mut Vec<u8>, f: &Field, slot: usize, be: bool) {
    put_u16(out, f.tag, be);
    put_u16(out, f.ty, be);
    let count = match f.ty {
        1 | 2 | 6 | 7 => f.payload.len(),
        3 | 8 => f.payload.len() / 2,
        4 | 9 | 11 | 13 => f.payload.len() / 4,
        5 | 10 | 12 | 16 | 17 | 18 => f.payload.len() / 8,
        _ => 0,
    } as u64;
    if slot == 4 {
        put_u32(out, count as u32, be);
    } else {
        put_u64(out, count, be);
    }
    if f.payload.len() <= slot {
        out.extend_from_slice(&f.payload);
        out.resize(out.len() + slot - f.payload.len(), 0);
    } else if slot == 4 {
        put_u32(out, f.at.unwrap() as u32, be);
    } else {
        put_u64(out, f.at.unwrap() as u64, be);
    }
}

fn build_pages(pages: &[Page], variant: Variant, be: bool) -> Vec<u8> {
    let (header_len, entry_len, count_len, next_len, slot) = match variant {
        Variant::Classic => (8usize, 12usize, 2usize, 4usize, 4usize),
        Variant::Big => (16usize, 20usize, 8usize, 8usize, 8usize),
    };
    let mut ifd_offsets = Vec::with_capacity(pages.len());
    let mut cursor = header_len;
    for page in pages {
        let fields = page_fields(page, be, variant);
        ifd_offsets.push(cursor);
        cursor += count_len + fields.len() * entry_len + next_len;
    }
    let mut out = vec![0; cursor];
    for (index, page) in pages.iter().enumerate() {
        let ifd_offset = ifd_offsets[index];
        let next = ifd_offsets.get(index + 1).copied().unwrap_or(0);
        let mut fields = page_fields(page, be, variant);
        let mut data_cursor = out.len();
        for f in &mut fields {
            if f.payload.len() > slot {
                if data_cursor & 1 != 0 {
                    data_cursor += 1;
                }
                f.at = Some(data_cursor);
                data_cursor += f.payload.len();
            }
        }
        let block_offsets: Vec<usize> = page
            .blocks
            .iter()
            .map(|block| {
                let at = data_cursor;
                data_cursor += block.len();
                at
            })
            .collect();
        let offsets_tag = if page.tile.is_some() { 324 } else { 273 };
        let counts_tag = if page.tile.is_some() { 325 } else { 279 };
        let big = matches!(variant, Variant::Big);
        let mut offsets = Vec::new();
        let mut counts = Vec::new();
        for (at, block) in block_offsets.iter().zip(&page.blocks) {
            if big {
                put_u64(&mut offsets, *at as u64, be);
            } else {
                put_u32(&mut offsets, *at as u32, be);
            }
            if big {
                put_u64(&mut counts, block.len() as u64, be);
            } else {
                put_u32(&mut counts, block.len() as u32, be);
            }
        }
        for f in &mut fields {
            if f.tag == offsets_tag {
                f.payload = offsets.clone();
            }
            if f.tag == counts_tag {
                f.payload = counts.clone();
            }
        }
        if out.len() < data_cursor {
            out.resize(data_cursor, 0);
        }
        for f in &fields {
            if let Some(at) = f.at {
                out[at..at + f.payload.len()].copy_from_slice(&f.payload);
            }
        }
        let mut ifd = Vec::with_capacity(count_len + fields.len() * entry_len + next_len);
        if slot == 4 {
            put_u16(&mut ifd, fields.len() as u16, be);
        } else {
            put_u64(&mut ifd, fields.len() as u64, be);
        }
        for f in &fields {
            write_field(&mut ifd, f, slot, be);
        }
        if slot == 4 {
            put_u32(&mut ifd, next as u32, be);
        } else {
            put_u64(&mut ifd, next as u64, be);
        }
        out[ifd_offset..ifd_offset + ifd.len()].copy_from_slice(&ifd);
        for (at, block) in block_offsets.iter().zip(&page.blocks) {
            out[*at..*at + block.len()].copy_from_slice(block);
        }
    }
    let mut header = Vec::new();
    header.extend_from_slice(if be { b"MM" } else { b"II" });
    put_u16(
        &mut header,
        if matches!(variant, Variant::Big) {
            43
        } else {
            42
        },
        be,
    );
    if matches!(variant, Variant::Big) {
        put_u16(&mut header, 8, be);
        put_u16(&mut header, 0, be);
        put_u64(&mut header, ifd_offsets[0] as u64, be);
    } else {
        put_u32(&mut header, ifd_offsets[0] as u32, be);
    }
    out[..header.len()].copy_from_slice(&header);
    out
}

fn page(
    width: u32,
    height: u32,
    bits: &[u16],
    samples: u16,
    photo: u16,
    raw: Vec<Vec<u8>>,
) -> Page {
    Page {
        width,
        height,
        bits: bits.to_vec(),
        samples,
        photo,
        rows: height,
        planar: 1,
        predictor: 1,
        compression: 1,
        tile: None,
        blocks: raw,
        extra: Vec::new(),
        orientation: 1,
        icc: Vec::new(),
    }
}
fn rgba(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
fn decode(bytes: &[u8]) -> Vec<u8> {
    image_load(bytes).unwrap().buffer.unwrap()
}

#[test]
fn associated_alpha_is_converted_to_straight_in_legacy_output() {
    for be in [false, true] {
        for bits in [8, 16] {
            for photo in [1, 2] {
                let channels = if photo == 1 { 2 } else { 4 };
                let values: Vec<u16> = if photo == 1 {
                    vec![10, 85, 0, 0]
                } else {
                    vec![10, 20, 30, 85, 0, 0, 0, 0]
                };
                let raw = if bits == 8 {
                    values.iter().map(|v| *v as u8).collect()
                } else {
                    bytes_u16(&values.iter().map(|v| v * 257).collect::<Vec<_>>(), be)
                };
                let mut p = page(
                    2,
                    1,
                    &vec![bits; channels],
                    channels as u16,
                    photo,
                    vec![raw],
                );
                p.extra = vec![1];
                let bytes = build_pages(&[p], Variant::Big, be);
                let expected = if photo == 1 {
                    vec![30, 30, 30, 85, 0, 0, 0, 0]
                } else {
                    vec![30, 60, 90, 85, 0, 0, 0, 0]
                };
                assert_eq!(
                    decode(&bytes),
                    expected,
                    "bits={bits} photo={photo} be={be}"
                );
                #[cfg(feature = "high-bit-depth")]
                if bits == 16 {
                    let frame =
                        wml2::highres::tiff::decode_native(&bytes, &DecodeLimits::default())
                            .unwrap();
                    assert_eq!(
                        frame.descriptor().alpha(),
                        wml2::highres::AlphaAssociation::Premultiplied
                    );
                    assert_eq!(
                        frame.pixels().u16_planes().unwrap()[0].samples(),
                        values.iter().map(|v| v * 257).collect::<Vec<_>>()
                    );
                }
            }
        }
    }
}

#[test]
fn legacy_gray16_predictor_preserves_carry_endian_and_polarity() {
    for be in [false, true] {
        for photo in [0, 1] {
            let mut p = page(
                3,
                1,
                &[16],
                1,
                photo,
                vec![bytes_u16(&[0x00ff, 1, 0xfeff], be)],
            );
            p.predictor = 2;
            let gray = if photo == 1 {
                [0, 1, 255]
            } else {
                [255, 254, 0]
            };
            let expected: Vec<u8> = gray.into_iter().flat_map(|v| [v, v, v, 255]).collect();
            assert_eq!(decode(&build_pages(&[p], Variant::Classic, be)), expected);
        }
    }
}

#[test]
fn classic_strip_formats_and_cmyk_k_are_decoded() {
    let raw = rgba(&[0, 0, 0, 255, 255, 0, 0, 0]);
    for mode in [1, 5, 8, 32946, 32773] {
        let mut p = page(2, 1, &[8, 8, 8, 8], 4, 5, vec![compression(&raw, mode)]);
        p.compression = mode;
        let out = decode(&build_pages(&[p], Variant::Classic, false));
        assert_eq!(&out[..4], &[0, 0, 0, 255], "mode {mode}");
        assert_eq!(&out[4..8], &[0, 255, 255, 255], "mode {mode}");
    }
}

#[test]
fn deflate_streams_are_inflated_independently_per_strip() {
    let mut p = page(
        1,
        2,
        &[8, 8, 8],
        3,
        2,
        vec![compression(&[1, 2, 3], 8), compression(&[4, 5, 6], 8)],
    );
    p.rows = 1;
    p.compression = 8;
    let out = decode(&build_pages(&[p], Variant::Classic, false));
    assert_eq!(out, vec![1, 2, 3, 255, 4, 5, 6, 255]);
}

#[test]
fn packed_gray_and_rgba_samples_are_decoded() {
    let gray = page(8, 1, &[1], 1, 1, vec![vec![0b1010_0000]]);
    let out = decode(&build_pages(&[gray], Variant::Classic, false));
    assert_eq!(&out[0..4], &[255, 255, 255, 255]);
    assert_eq!(&out[4..8], &[0, 0, 0, 255]);
    assert_eq!(&out[8..12], &[255, 255, 255, 255]);
    assert_eq!(&out[12..16], &[0, 0, 0, 255]);

    let mut rgba_page = page(1, 1, &[8, 8, 8, 8], 4, 2, vec![vec![9, 8, 7, 6]]);
    rgba_page.extra = vec![2];
    assert_eq!(
        decode(&build_pages(&[rgba_page], Variant::Classic, false)),
        vec![9, 8, 7, 6]
    );
}

#[test]
fn classic_and_bigtiff_both_endians_decode_rgb_strip() {
    for variant in [Variant::Classic, Variant::Big] {
        for be in [false, true] {
            let p = page(2, 1, &[8, 8, 8], 3, 2, vec![vec![1, 2, 3, 4, 5, 6]]);
            let out = decode(&build_pages(&[p], variant, be));
            assert_eq!(out, vec![1, 2, 3, 255, 4, 5, 6, 255]);
        }
    }
}

#[test]
fn tiled_right_edge_and_tile_length_at_least_height_are_drawn() {
    let mut p = page(
        3,
        2,
        &[8, 8, 8],
        3,
        2,
        vec![
            vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0],
            vec![10, 20, 30, 0, 0, 0, 40, 50, 60, 0, 0, 0],
        ],
    );
    p.tile = Some((2, 2));
    let out = decode(&build_pages(&[p], Variant::Classic, false));
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);
    assert_eq!(&out[8..12], &[10, 20, 30, 255]);
    assert_eq!(&out[16..20], &[255, 255, 0, 255]);
}

#[test]
fn planar_rgb16_predictor_is_applied_before_quantization() {
    let mut p = page(
        2,
        1,
        &[16, 16, 16],
        3,
        2,
        vec![
            vec![0xff, 0x00, 0x01, 0x00],
            vec![0x00, 0x10, 0x10, 0x00],
            vec![0x00, 0x80, 0x80, 0x00],
        ],
    );
    p.planar = 2;
    p.predictor = 2;
    let out = decode(&build_pages(&[p], Variant::Classic, false));
    assert_eq!(&out[..4], &[0, 16, 128, 255]);
    assert_eq!(&out[4..8], &[1, 16, 128, 255]);
}

#[test]
fn multipage_ifds_are_independent_of_tag_order() {
    let first = page(1, 1, &[8, 8, 8], 3, 2, vec![vec![9, 8, 7]]);
    let second = page(1, 1, &[8], 1, 1, vec![vec![42]]);
    let bytes = build_pages(&[first, second], Variant::Classic, false);
    let image = image_load(&bytes).unwrap();
    assert_eq!(image.width, 1);
    assert_eq!(image.height, 1);
    assert_eq!(
        image
            .metadata
            .unwrap()
            .get("image pages")
            .unwrap()
            .to_string(),
        "2"
    );
}

#[test]
fn malformed_block_arrays_dimensions_and_limits_are_rejected() {
    let mut p = page(1, 1, &[8, 8, 8], 3, 2, vec![vec![1, 2, 3]]);
    let mut bytes = build_pages(&[p.clone()], Variant::Classic, false);
    // StripByteCounts points one byte past the file.
    let entry_count = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    for index in 0..entry_count {
        let entry = 8 + 2 + index * 12;
        if u16::from_le_bytes([bytes[entry], bytes[entry + 1]]) == 279 {
            bytes[entry + 8..entry + 12].copy_from_slice(&[0xff, 0xff, 0xff, 0x7f]);
        }
    }
    assert!(image_load(&bytes).is_err());
    p.width = 100;
    p.height = 100;
    p.blocks = vec![vec![1, 2, 3]];
    let bytes = build_pages(&[p], Variant::Classic, false);
    assert!(
        image_from_with_limits(
            &bytes,
            DecodeLimits {
                pixels: 1,
                ..DecodeLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn truncated_block_is_rejected() {
    let p = page(2, 1, &[8, 8, 8], 3, 2, vec![vec![1, 2, 3, 4, 5]]);
    let mut bytes = build_pages(&[p], Variant::Classic, false);
    bytes.truncate(bytes.len() - 1);
    assert!(image_load(&bytes).is_err());
}

#[test]
fn short_last_strips_and_rows_larger_than_image_work_for_all_generic_codecs() {
    for mode in [1, 5, 8, 32946, 32773] {
        for variant in [Variant::Classic, Variant::Big] {
            for be in [false, true] {
                let mut p = page(
                    2,
                    3,
                    &[8],
                    1,
                    1,
                    vec![
                        compression(&[10, 20, 30, 40], mode),
                        compression(&[50, 60], mode),
                    ],
                );
                p.rows = 2;
                p.compression = mode;
                let decoded = decode(&build_pages(&[p], variant, be));
                let expected: Vec<_> = [10u8, 20, 30, 40, 50, 60]
                    .into_iter()
                    .flat_map(|v| [v, v, v, 255])
                    .collect();
                assert_eq!(decoded, expected, "short final strip, mode {mode}");
                let mut p = page(2, 1, &[8], 1, 1, vec![compression(&[90, 100], mode)]);
                p.rows = u32::MAX;
                p.compression = mode;
                assert_eq!(
                    decode(&build_pages(&[p], variant, be)),
                    [90, 90, 90, 255, 100, 100, 100, 255]
                );
            }
        }
    }
}

#[test]
fn tile_boundary_matrix_covers_bottom_and_right_padding_for_all_generic_codecs() {
    for mode in [1, 5, 8, 32946, 32773] {
        for variant in [Variant::Classic, Variant::Big] {
            for (width, height) in [(1, 1), (3, 1), (1, 3), (3, 3), (4, 4)] {
                let mut blocks = Vec::new();
                for ty in (0..height).step_by(2) {
                    for tx in (0..width).step_by(2) {
                        let mut raw = Vec::new();
                        for y in ty..ty + 2 {
                            for x in tx..tx + 2 {
                                let value = if x < width && y < height {
                                    (y * width + x) as u8
                                } else {
                                    200
                                };
                                raw.extend_from_slice(&[value, value + 10, value + 20]);
                            }
                        }
                        blocks.push(compression(&raw, mode));
                    }
                }
                let mut p = page(width, height, &[8, 8, 8], 3, 2, blocks);
                p.tile = Some((2, 2));
                p.compression = mode;
                let expected: Vec<_> = (0..width * height)
                    .flat_map(|v| [v as u8, v as u8 + 10, v as u8 + 20, 255])
                    .collect();
                assert_eq!(
                    decode(&build_pages(&[p], variant, false)),
                    expected,
                    "mode {mode} size {width}x{height}"
                );
            }
        }
    }
}

#[test]
fn packed_gray_rows_restart_after_padding_bits() {
    for bits in [1, 2, 4] {
        let width = 3;
        let mask = (1u8 << bits) - 1;
        let mut raw = vec![0u8; (width * usize::from(bits)).div_ceil(8) * 2];
        for y in 0..2 {
            for x in 0..width {
                let bit = x * usize::from(bits);
                let value = if (x + y) % 2 == 0 { mask } else { 0 };
                raw[y * (width * usize::from(bits)).div_ceil(8) + bit / 8] |=
                    value << (8 - usize::from(bits) - bit % 8);
            }
        }
        let p = page(3, 2, &[bits], 1, 1, vec![raw]);
        let expected: Vec<_> = [255u8, 0, 255, 0, 255, 0]
            .into_iter()
            .flat_map(|v| [v, v, v, 255])
            .collect();
        assert_eq!(
            decode(&build_pages(&[p], Variant::Classic, false)),
            expected,
            "depth {bits}"
        );
    }
}

#[test]
fn extra_sample_counts_and_values_are_validated() {
    for extra in [vec![2, 1], vec![3]] {
        let mut p = page(1, 1, &[8, 8, 8, 8], 4, 2, vec![vec![1, 2, 3, 4]]);
        p.extra = extra;
        assert!(image_load(&build_pages(&[p], Variant::Classic, false)).is_err());
    }
    let mut p = page(1, 1, &[8, 8, 8], 3, 2, vec![vec![1, 2, 3]]);
    p.extra = vec![2];
    assert!(image_load(&build_pages(&[p], Variant::Classic, false)).is_err());
}

#[test]
fn sparse_bigtiff_high_offsets_are_read_without_large_allocation() {
    let ifd = 0x1_0000_0100u64;
    let pixels = 0x1_0000_0200u64;
    let mut header = Vec::new();
    header.extend_from_slice(b"II");
    put_u16(&mut header, 43, false);
    put_u16(&mut header, 8, false);
    put_u16(&mut header, 0, false);
    put_u64(&mut header, ifd, false);
    let mut directory = Vec::new();
    put_u64(&mut directory, 9, false);
    let values = [
        (256u16, 4u16, 1u64),
        (257, 4, 1),
        (258, 3, 8),
        (259, 3, 1),
        (262, 3, 1),
        (273, 16, pixels),
        (277, 3, 1),
        (278, 4, 1),
        (279, 16, 1),
    ];
    for (tag, ty, value) in values {
        put_u16(&mut directory, tag, false);
        put_u16(&mut directory, ty, false);
        put_u64(&mut directory, 1, false);
        put_u64(&mut directory, value, false);
    }
    put_u64(&mut directory, 0, false);
    let mut sparse = SparseReader::new(ifd.max(pixels + 1) + directory.len() as u64);
    sparse.segment(0, header);
    sparse.segment(ifd, directory);
    sparse.segment(pixels, vec![77]);
    let mut reader = StreamReader::new(BufReader::new(sparse));
    let mut image = wml2::draw::ImageBuffer::new();
    let mut options = wml2::draw::DecodeOptions {
        debug_flag: 0,
        drawer: &mut image,
    };
    image_decoder_with_limits(
        &mut reader,
        &mut options,
        DecodeLimits {
            input_bytes: usize::MAX,
            ..DecodeLimits::default()
        },
    )
    .unwrap();
    assert_eq!(image.buffer.unwrap(), vec![77, 77, 77, 255]);
}

#[test]
fn malformed_tiff_layouts_are_rejected() {
    // A zero tile dimension is invalid even when the tile arrays are present.
    let mut zero_tile = page(1, 1, &[8], 1, 1, vec![vec![1]]);
    zero_tile.tile = Some((0, 1));
    assert!(image_load(&build_pages(&[zero_tile], Variant::Classic, false)).is_err());

    // An IFD chain must not be allowed to loop back to itself.
    let mut cycle = build_pages(
        &[page(1, 1, &[8], 1, 1, vec![vec![1]])],
        Variant::Classic,
        false,
    );
    let entries = u16::from_le_bytes([cycle[8], cycle[9]]) as usize;
    let next = 8 + 2 + entries * 12;
    cycle[next..next + 4].copy_from_slice(&8u32.to_le_bytes());
    assert!(image_load(&cycle).is_err());

    // A huge entry count is rejected before allocation/read of entries.
    let mut huge = build_pages(
        &[page(1, 1, &[8], 1, 1, vec![vec![1]])],
        Variant::Classic,
        false,
    );
    huge[8..10].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(image_load(&huge).is_err());

    // Offset and byte-count arrays must have the same number of entries.
    let mut mismatch = page(1, 2, &[8, 8, 8], 3, 2, vec![vec![1, 2, 3], vec![4, 5, 6]]);
    mismatch.rows = 1;
    let mut mismatch = build_pages(&[mismatch], Variant::Classic, false);
    let entries = u16::from_le_bytes([mismatch[8], mismatch[9]]) as usize;
    for index in 0..entries {
        let entry = 8 + 2 + index * 12;
        if u16::from_le_bytes([mismatch[entry], mismatch[entry + 1]]) == 279 {
            mismatch[entry + 4..entry + 8].copy_from_slice(&1u32.to_le_bytes());
        }
    }
    assert!(image_load(&mismatch).is_err());
}

fn bigtiff_directory(pixel: u64, tile: bool, next: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let count = if tile { 10 } else { 9 };
    put_u64(&mut out, count, false);
    let mut entry = |tag: u16, ty: u16, value: u64| {
        put_u16(&mut out, tag, false);
        put_u16(&mut out, ty, false);
        put_u64(&mut out, 1, false);
        put_u64(&mut out, value, false);
    };
    entry(256, 4, 1);
    entry(257, 4, 1);
    entry(258, 3, 8);
    entry(259, 3, 1);
    entry(262, 3, 1);
    entry(if tile { 324 } else { 273 }, 16, pixel);
    entry(277, 3, 1);
    if tile {
        entry(322, 4, 1);
        entry(323, 4, 1);
    } else {
        entry(278, 4, 1);
    }
    entry(if tile { 325 } else { 279 }, 16, 1);
    put_u64(&mut out, next, false);
    out
}

#[test]
fn sparse_bigtiff_tile_offset_and_next_ifd_can_exceed_u32() {
    let second_ifd = 0x1_0000_1000u64;
    let second_pixel = second_ifd + 0x200;
    let first_pixel = 0x200u64;
    let mut header = Vec::new();
    header.extend_from_slice(b"II");
    put_u16(&mut header, 43, false);
    put_u16(&mut header, 8, false);
    put_u16(&mut header, 0, false);
    put_u64(&mut header, 16, false);
    let first = bigtiff_directory(first_pixel, false, second_ifd);
    let second = bigtiff_directory(second_pixel, true, 0);
    let mut sparse = SparseReader::new(second_pixel + 1);
    sparse.segment(0, header);
    sparse.segment(16, first);
    sparse.segment(second_ifd, second);
    sparse.segment(first_pixel, vec![11]);
    sparse.segment(second_pixel, vec![22]);
    let mut reader = StreamReader::new(BufReader::new(sparse));
    let mut image = wml2::draw::ImageBuffer::new();
    let mut options = wml2::draw::DecodeOptions {
        debug_flag: 0,
        drawer: &mut image,
    };
    image_decoder_with_limits(
        &mut reader,
        &mut options,
        DecodeLimits {
            input_bytes: usize::MAX,
            ..DecodeLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        image
            .metadata
            .unwrap()
            .get("image pages")
            .unwrap()
            .to_string(),
        "2"
    );
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn native_rgb16_preserves_samples_and_tiff_metadata() {
    let mut p = page(
        2,
        1,
        &[16, 16, 16],
        3,
        2,
        vec![bytes_u16(
            &[0x1234, 0xabcd, 0x8000, 0xffff, 0x0001, 0x007f],
            false,
        )],
    );
    p.orientation = 6;
    p.icc = b"tiny-icc".to_vec();
    let bytes = build_pages(&[p], Variant::Classic, false);
    let frame = wml2::highres::tiff::decode_native(&bytes, &DecodeLimits::default()).unwrap();
    assert_eq!(frame.descriptor().width(), 2);
    assert_eq!(frame.descriptor().height(), 1);
    assert_eq!(frame.descriptor().model(), wml2::highres::ChannelModel::RGB);
    assert_eq!(
        frame.pixels().u16_planes().unwrap()[0].samples(),
        &[0x1234, 0xabcd, 0x8000, 0xffff, 0x0001, 0x007f]
    );
    assert_eq!(
        frame
            .metadata()
            .tags()
            .get("Orientation")
            .unwrap()
            .to_string(),
        "6"
    );
    assert_eq!(
        frame
            .metadata()
            .tags()
            .get("ICC Profile")
            .unwrap()
            .to_string(),
        format!("{:?}", b"tiny-icc")
    );
    assert_eq!(
        frame.metadata().source_color().icc_profile(),
        Some(&b"tiny-icc"[..])
    );

    let p = page(
        1,
        1,
        &[16, 16, 16],
        3,
        2,
        vec![bytes_u16(&[0x0102, 0x0304, 0x0506], true)],
    );
    let frame = wml2::highres::tiff::decode_native(
        &build_pages(&[p], Variant::Classic, true),
        &DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(
        frame.pixels().u16_planes().unwrap()[0].samples(),
        &[0x0102, 0x0304, 0x0506]
    );
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn native_gray_white_is_zero_and_rgba_alpha_association_are_preserved() {
    let gray = page(2, 1, &[16], 1, 0, vec![vec![0x00, 0x00, 0xff, 0xff]]);
    let frame = wml2::highres::tiff::decode_native(
        &build_pages(&[gray], Variant::Classic, false),
        &DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(
        frame.pixels().u16_planes().unwrap()[0].samples(),
        &[u16::MAX, 0]
    );

    let mut rgba_page = page(
        1,
        1,
        &[16, 16, 16, 16],
        4,
        2,
        vec![vec![0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44]],
    );
    rgba_page.extra = vec![2];
    let frame = wml2::highres::tiff::decode_native(
        &build_pages(&[rgba_page], Variant::Classic, false),
        &DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(
        frame.descriptor().alpha(),
        wml2::highres::AlphaAssociation::Straight
    );
    assert_eq!(
        frame.pixels().u16_planes().unwrap()[0].samples(),
        &[0x1111, 0x2222, 0x3333, 0x4444]
    );
}

#[cfg(feature = "high-bit-depth")]
#[test]
fn native_pages_keep_each_16bit_page_independent() {
    let first = page(1, 1, &[16], 1, 1, vec![vec![0x34, 0x12]]);
    let second = page(1, 1, &[16], 1, 1, vec![vec![0xcd, 0xab]]);
    let frames = wml2::highres::tiff::decode_native_pages(
        &build_pages(&[first, second], Variant::Classic, false),
        &DecodeLimits::default(),
    )
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0].pixels().u16_planes().unwrap()[0].samples(),
        &[0x1234]
    );
    assert_eq!(
        frames[1].pixels().u16_planes().unwrap()[0].samples(),
        &[0xabcd]
    );
}

struct SparseReader {
    len: u64,
    pos: u64,
    segments: Vec<(u64, Vec<u8>)>,
}
impl SparseReader {
    fn new(len: u64) -> Self {
        Self {
            len,
            pos: 0,
            segments: Vec::new(),
        }
    }
    fn segment(&mut self, at: u64, data: Vec<u8>) {
        self.segments.push((at, data));
    }
}
impl Read for SparseReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let amount = (self.len - self.pos).min(buf.len() as u64) as usize;
        buf[..amount].fill(0);
        for (at, data) in &self.segments {
            let start = self.pos.max(*at);
            let end = (self.pos + amount as u64).min(*at + data.len() as u64);
            if start < end {
                let dst = (start - self.pos) as usize;
                let src = (start - *at) as usize;
                buf[dst..dst + (end - start) as usize]
                    .copy_from_slice(&data[src..src + (end - start) as usize]);
            }
        }
        self.pos += amount as u64;
        Ok(amount)
    }
}
impl Seek for SparseReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let next = match from {
            SeekFrom::Start(v) => v as i128,
            SeekFrom::Current(v) => self.pos as i128 + v as i128,
            SeekFrom::End(v) => self.len as i128 + v as i128,
        };
        if next < 0 || next > u64::MAX as i128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "sparse seek out of range",
            ));
        }
        self.pos = next as u64;
        Ok(self.pos)
    }
}
