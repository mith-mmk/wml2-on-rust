#![cfg(feature = "tiff-jpeg")]

use wml2::draw::{image_from_with_limits, image_load};
use wml2::limits::DecodeLimits;

fn marker(out: &mut Vec<u8>, code: u8, payload: &[u8]) {
    out.extend_from_slice(&[0xff, code]);
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
}

fn jpeg(ids: [u8; 3], tables_separate: bool) -> (Vec<u8>, Vec<u8>) {
    jpeg_with_components(ids, tables_separate, [200, 20, 50], 8)
}

// Constant-component JPEG using DC category 10 for the first MCU and category
// 0 for subsequent MCUs. All AC coefficients are zero; expected samples are
// exact and do not depend on another JPEG writer.
fn jpeg_with_components(
    ids: [u8; 3],
    tables_separate: bool,
    samples: [u8; 3],
    side: u16,
) -> (Vec<u8>, Vec<u8>) {
    assert!(side == 8 || side == 16);
    let mut tables = vec![0xff, 0xd8];
    let mut quantization = vec![1; 65];
    quantization[0] = 0;
    marker(&mut tables, 0xdb, &quantization);
    for (class, symbol) in [(0, 10), (0x10, 0)] {
        let mut huffman = vec![class, 1];
        huffman.push(if class == 0 { 1 } else { 0 });
        huffman.extend_from_slice(&[0; 14]);
        huffman.push(symbol);
        if class == 0 {
            huffman.push(0); // DC category 0 has the two-bit code 10.
        }
        marker(&mut tables, 0xc4, &huffman);
    }
    let mut data = if tables_separate {
        vec![0xff, 0xd8]
    } else {
        tables.clone()
    };
    let mut frame = vec![8];
    frame.extend_from_slice(&side.to_be_bytes());
    frame.extend_from_slice(&side.to_be_bytes());
    frame.push(3);
    for id in ids {
        frame.extend_from_slice(&[id, 0x11, 0]);
    }
    marker(&mut data, 0xc0, &frame);
    let mut scan = vec![3];
    for id in ids {
        scan.extend_from_slice(&[id, 0]);
    }
    scan.extend_from_slice(&[0, 63, 0]);
    marker(&mut data, 0xda, &scan);
    let mut entropy = String::new();
    for mcu in 0..(side / 8).pow(2) {
        for sample in samples {
            if mcu == 0 {
                let coefficient = (i32::from(sample) - 128) * 8;
                assert!((512..1024).contains(&coefficient.abs()));
                let encoded = if coefficient >= 0 {
                    coefficient
                } else {
                    1023 + coefficient
                };
                entropy.push_str(&format!("0{encoded:010b}0"));
            } else {
                entropy.push_str("100"); // Zero DC difference, then AC EOB.
            }
        }
    }
    while entropy.len() % 8 != 0 {
        entropy.push('1');
    }
    for bits in entropy.as_bytes().chunks(8) {
        let byte = bits
            .iter()
            .fold(0, |value, &bit| (value << 1) | (bit - b'0'));
        data.push(byte);
        if byte == 255 {
            data.push(0);
        }
    }
    data.extend_from_slice(&[0xff, 0xd9]);
    tables.extend_from_slice(&[0xff, 0xd9]);
    (data, if tables_separate { tables } else { vec![] })
}

fn tiff(photo: u16, jpeg: &[u8], tables: &[u8], dimensions: (u32, u32)) -> Vec<u8> {
    tiff_blocks(photo, &[jpeg.to_vec()], tables, dimensions, None)
}

fn tiff_blocks(
    photo: u16,
    jpegs: &[Vec<u8>],
    tables: &[u8],
    dimensions: (u32, u32),
    tile: Option<(u32, u32)>,
) -> Vec<u8> {
    let mut fields = vec![
        (256u16, 4u16, 1u32, dimensions.0.to_le_bytes().to_vec()),
        (257, 4, 1, dimensions.1.to_le_bytes().to_vec()),
        (258, 3, 3, vec![8, 0, 8, 0, 8, 0]),
        (259, 3, 1, vec![7, 0]),
        (262, 3, 1, photo.to_le_bytes().to_vec()),
        (273, 4, jpegs.len() as u32, vec![0; 4 * jpegs.len()]),
        (277, 3, 1, vec![3, 0]),
        (278, 4, 1, dimensions.1.to_le_bytes().to_vec()),
        (
            279,
            4,
            jpegs.len() as u32,
            jpegs
                .iter()
                .flat_map(|jpeg| (jpeg.len() as u32).to_le_bytes())
                .collect(),
        ),
        (284, 3, 1, vec![1, 0]),
        (530, 3, 2, vec![1, 0, 1, 0]),
    ];
    if let Some((width, height)) = tile {
        fields.retain(|field| field.0 != 278);
        fields.iter_mut().find(|field| field.0 == 273).unwrap().0 = 324;
        fields.iter_mut().find(|field| field.0 == 279).unwrap().0 = 325;
        fields.push((322, 4, 1, width.to_le_bytes().to_vec()));
        fields.push((323, 4, 1, height.to_le_bytes().to_vec()));
    }
    if !tables.is_empty() {
        fields.push((347, 7, tables.len() as u32, tables.to_vec()));
    }
    fields.sort_by_key(|field| field.0);
    let base = 8 + 2 + 12 * fields.len() + 4;
    let jpeg_offset = base
        + fields
            .iter()
            .filter(|field| field.3.len() > 4)
            .map(|field| field.3.len())
            .sum::<usize>();
    let mut offset = jpeg_offset as u32;
    let offsets = jpegs
        .iter()
        .flat_map(|jpeg| {
            let at = offset;
            offset += jpeg.len() as u32;
            at.to_le_bytes()
        })
        .collect();
    let offset_tag = if tile.is_some() { 324 } else { 273 };
    fields
        .iter_mut()
        .find(|field| field.0 == offset_tag)
        .unwrap()
        .3 = offsets;
    let mut data = b"II\x2a\0\x08\0\0\0".to_vec();
    data.extend_from_slice(&(fields.len() as u16).to_le_bytes());
    let mut extra = Vec::new();
    for (tag, kind, count, payload) in fields {
        data.extend_from_slice(&tag.to_le_bytes());
        data.extend_from_slice(&kind.to_le_bytes());
        data.extend_from_slice(&count.to_le_bytes());
        if payload.len() > 4 {
            data.extend_from_slice(&((base + extra.len()) as u32).to_le_bytes());
            extra.extend_from_slice(&payload);
        } else {
            data.extend_from_slice(&payload);
            data.resize(data.len() + 4 - payload.len(), 0);
        }
    }
    data.extend_from_slice(&[0; 4]);
    data.extend_from_slice(&extra);
    for jpeg in jpegs {
        data.extend_from_slice(jpeg);
    }
    data
}

fn assert_color(data: &[u8], expected: [u8; 4]) {
    let image = image_load(data).unwrap();
    assert_eq!((image.width, image.height), (8, 8));
    for pixel in image.buffer.unwrap().chunks_exact(4) {
        for (&actual, expected) in pixel.iter().zip(expected) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "pixel {pixel:?}, expected {expected}"
            );
        }
    }
}

#[test]
fn rgb_tiff_uses_photometric_with_numeric_component_ids() {
    for separate in [false, true] {
        let (jpeg, tables) = jpeg([1, 2, 3], separate);
        assert_color(&tiff(2, &jpeg, &tables, (8, 8)), [200, 20, 50, 255]);
    }
}

#[test]
fn ycbcr_tiff_uses_photometric_with_rgb_component_ids() {
    for separate in [false, true] {
        let (jpeg, tables) = jpeg(*b"RGB", separate);
        assert_color(&tiff(6, &jpeg, &tables, (8, 8)), [91, 255, 9, 255]);
    }
}

#[test]
fn ordinary_jpeg_keeps_component_id_color_selection() {
    let (rgb, _) = jpeg(*b"RGB", false);
    assert_color(&rgb, [200, 20, 50, 255]);
    let (ycbcr, _) = jpeg([1, 2, 3], false);
    assert_color(&ycbcr, [91, 255, 9, 255]);
}

#[test]
fn embedded_jpeg_keeps_parent_pixel_limit() {
    let (jpeg, tables) = jpeg([1, 2, 3], false);
    let data = tiff(6, &jpeg, &tables, (1, 1));
    let error = image_from_with_limits(
        &data,
        DecodeLimits {
            pixels: 4,
            ..DecodeLimits::default()
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("limit"), "{error}");
}

#[test]
fn rejects_jpeg_tables_without_soi_or_eoi() {
    let (jpeg, tables) = jpeg([1, 2, 3], true);
    for position in [0, tables.len() - 1] {
        let mut invalid = tables.clone();
        invalid[position] = 0;
        let error = image_load(&tiff(6, &jpeg, &invalid, (8, 8))).err().unwrap();
        assert!(error.to_string().contains("JPEG tables"), "{error}");
    }
}

#[test]
fn rejects_jpeg_payload_without_soi_or_eoi() {
    let (jpeg, tables) = jpeg([1, 2, 3], true);
    for position in [0, jpeg.len() - 1] {
        let mut invalid = jpeg.clone();
        invalid[position] = 0;
        let error = image_load(&tiff(6, &invalid, &tables, (8, 8)))
            .err()
            .unwrap();
        assert!(error.to_string().contains("JPEG tile payload"), "{error}");
    }
}

fn assert_tiled_pixels(width: u32, height: u32) {
    const TILE: u32 = 16;
    let colors = [[200, 20, 50], [50, 200, 20], [20, 50, 200], [220, 230, 240]];
    let across = width.div_ceil(TILE);
    let count = across * height.div_ceil(TILE);
    for separate in [false, true] {
        let mut tables = Vec::new();
        let jpegs: Vec<_> = colors[..count as usize]
            .iter()
            .map(|&color| {
                let (jpeg, shared) = jpeg_with_components(*b"RGB", separate, color, TILE as u16);
                tables = shared;
                jpeg
            })
            .collect();
        let data = tiff_blocks(2, &jpegs, &tables, (width, height), Some((TILE, TILE)));
        let image = image_load(&data).unwrap();
        assert_eq!(
            (image.width, image.height),
            (width as usize, height as usize)
        );
        let pixels = image.buffer.unwrap();
        assert_eq!(pixels.len(), width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                let [red, green, blue] = colors[(y / TILE * across + x / TILE) as usize];
                let position = ((y * width + x) * 4) as usize;
                assert_eq!(
                    &pixels[position..position + 4],
                    &[red, green, blue, 255],
                    "image {width}x{height}, pixel ({x}, {y}), shared tables {separate}"
                );
            }
        }
    }
}

fn jpeg_marker_payload(data: &[u8], code: u8) -> Vec<u8> {
    let marker = [0xff, code];
    let start = data.windows(2).position(|window| window == marker).unwrap();
    let length = u16::from_be_bytes([data[start + 2], data[start + 3]]) as usize;
    data[start + 4..start + 2 + length].to_vec()
}

fn old_jpeg_fixture_parts(jpeg: &[u8], tables: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let quantization = jpeg_marker_payload(tables, 0xdb);
    let dc = jpeg_marker_payload(tables, 0xc4);
    let ac_start = tables
        .windows(2)
        .enumerate()
        .filter_map(|(index, window)| (window == [0xff, 0xc4]).then_some(index))
        .nth(1)
        .unwrap();
    let ac = {
        let length = u16::from_be_bytes([tables[ac_start + 2], tables[ac_start + 3]]) as usize;
        tables[ac_start + 5..ac_start + 2 + length].to_vec()
    };
    let scan = jpeg
        .windows(2)
        .position(|window| window == [0xff, 0xda])
        .unwrap();
    let scan_length = u16::from_be_bytes([jpeg[scan + 2], jpeg[scan + 3]]) as usize;
    let entropy_start = scan + 2 + scan_length;
    let entropy_end = jpeg.len() - 2;
    (
        quantization[1..].to_vec(),
        dc[1..].to_vec(),
        ac[1..].to_vec(),
        jpeg[entropy_start..entropy_end].to_vec(),
    )
}

fn old_jpeg_tiff(
    entropy: &[u8],
    quantization: &[u8],
    dc: &[u8],
    ac: &[u8],
    tile: bool,
    omit_q_tables: bool,
) -> Vec<u8> {
    #[derive(Clone)]
    struct Field {
        tag: u16,
        kind: u16,
        count: u32,
        data: Vec<u8>,
    }
    let u16v = |value: u16| value.to_le_bytes().to_vec();
    let u32v = |value: u32| value.to_le_bytes().to_vec();
    let mut fields = vec![
        Field {
            tag: 256,
            kind: 4,
            count: 1,
            data: u32v(8),
        },
        Field {
            tag: 257,
            kind: 4,
            count: 1,
            data: u32v(8),
        },
        Field {
            tag: 258,
            kind: 3,
            count: 3,
            data: [u16v(8), u16v(8), u16v(8)].concat(),
        },
        Field {
            tag: 259,
            kind: 3,
            count: 1,
            data: u16v(6),
        },
        Field {
            tag: 262,
            kind: 3,
            count: 1,
            data: u16v(6),
        },
        Field {
            tag: if tile { 324 } else { 273 },
            kind: 4,
            count: 1,
            data: vec![0; 4],
        },
        Field {
            tag: 277,
            kind: 3,
            count: 1,
            data: u16v(3),
        },
        Field {
            tag: 279,
            kind: 4,
            count: 1,
            data: u32v(entropy.len() as u32),
        },
        Field {
            tag: 284,
            kind: 3,
            count: 1,
            data: u16v(1),
        },
        Field {
            tag: 512,
            kind: 3,
            count: 1,
            data: u16v(1),
        },
        Field {
            tag: 513,
            kind: 4,
            count: 1,
            data: u32v(0),
        },
        Field {
            tag: 530,
            kind: 3,
            count: 2,
            data: [u16v(1), u16v(1)].concat(),
        },
        Field {
            tag: 519,
            kind: 4,
            count: 3,
            data: vec![0; 12],
        },
        Field {
            tag: 520,
            kind: 4,
            count: 3,
            data: vec![0; 12],
        },
        Field {
            tag: 521,
            kind: 4,
            count: 3,
            data: vec![0; 12],
        },
    ];
    if omit_q_tables {
        fields.retain(|field| field.tag != 519);
    }
    if tile {
        fields.push(Field {
            tag: 322,
            kind: 4,
            count: 1,
            data: u32v(8),
        });
        fields.push(Field {
            tag: 323,
            kind: 4,
            count: 1,
            data: u32v(8),
        });
        fields.push(Field {
            tag: 325,
            kind: 4,
            count: 1,
            data: u32v(entropy.len() as u32),
        });
        fields.retain(|field| field.tag != 279);
    } else {
        fields.push(Field {
            tag: 278,
            kind: 4,
            count: 1,
            data: u32v(8),
        });
    }
    fields.sort_by_key(|field| field.tag);

    let base = 8 + 2 + fields.len() * 12 + 4;
    let extra_len: usize = fields
        .iter()
        .filter(|field| field.data.len() > 4)
        .map(|field| field.data.len())
        .sum();
    let blob_start = base + extra_len;
    let table_bytes = quantization.len() + dc.len() + ac.len();
    let image_offset = blob_start + table_bytes;
    let q_offset = blob_start;
    let dc_offset = q_offset + quantization.len();
    let ac_offset = dc_offset + dc.len();
    let image_tag = if tile { 324 } else { 273 };
    let q_offsets = [q_offset as u32; 3]
        .map(|value| value.to_le_bytes())
        .concat();
    let dc_offsets = [dc_offset as u32; 3]
        .map(|value| value.to_le_bytes())
        .concat();
    let ac_offsets = [ac_offset as u32; 3]
        .map(|value| value.to_le_bytes())
        .concat();
    for field in &mut fields {
        match field.tag {
            519 => field.data = q_offsets.clone(),
            520 => field.data = dc_offsets.clone(),
            521 => field.data = ac_offsets.clone(),
            tag if tag == image_tag => field.data = u32v(image_offset as u32),
            _ => {}
        }
    }
    let mut output = b"II\x2a\0\x08\0\0\0".to_vec();
    output.extend_from_slice(&(fields.len() as u16).to_le_bytes());
    let mut extra = Vec::new();
    for field in &fields {
        output.extend_from_slice(&field.tag.to_le_bytes());
        output.extend_from_slice(&field.kind.to_le_bytes());
        output.extend_from_slice(&field.count.to_le_bytes());
        if field.data.len() > 4 {
            output.extend_from_slice(&((base + extra.len()) as u32).to_le_bytes());
            extra.extend_from_slice(&field.data);
        } else {
            output.extend_from_slice(&field.data);
            output.resize(output.len() + 4 - field.data.len(), 0);
        }
    }
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&extra);
    output.extend_from_slice(quantization);
    output.extend_from_slice(dc);
    output.extend_from_slice(ac);
    output.extend_from_slice(entropy);
    output
}

#[test]
fn old_style_jpeg_rebuilds_generated_table_form_strip_and_tile() {
    let (jpeg, tables) = jpeg_with_components(*b"RGB", true, [200, 20, 50], 8);
    let (quantization, dc, ac, entropy) = old_jpeg_fixture_parts(&jpeg, &tables);
    for tile in [false, true] {
        let data = old_jpeg_tiff(&entropy, &quantization, &dc, &ac, tile, false);
        let image = image_load(&data).unwrap();
        assert_eq!((image.width, image.height), (8, 8));
        assert_eq!(image.buffer.unwrap().len(), 8 * 8 * 4);
    }
}

#[test]
fn old_style_jpeg_rejects_missing_table_offsets() {
    let (jpeg, tables) = jpeg_with_components(*b"RGB", true, [200, 20, 50], 8);
    let (quantization, dc, ac, entropy) = old_jpeg_fixture_parts(&jpeg, &tables);
    let data = old_jpeg_tiff(&entropy, &quantization, &dc, &ac, false, true);
    let error = match image_load(&data) {
        Ok(_) => panic!("missing JPEG table offsets must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("missing"), "{error}");
}

#[test]
fn old_style_jpeg_rejects_out_of_range_table_pointer() {
    let (jpeg, tables) = jpeg_with_components(*b"RGB", true, [200, 20, 50], 8);
    let (quantization, dc, ac, entropy) = old_jpeg_fixture_parts(&jpeg, &tables);
    let mut data = old_jpeg_tiff(&entropy, &quantization, &dc, &ac, false, false);
    let ifd = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let count = u16::from_le_bytes(data[ifd..ifd + 2].try_into().unwrap()) as usize;
    for index in 0..count {
        let entry = ifd + 2 + index * 12;
        let tag = u16::from_le_bytes(data[entry..entry + 2].try_into().unwrap());
        if tag == 519 {
            let values =
                u32::from_le_bytes(data[entry + 8..entry + 12].try_into().unwrap()) as usize;
            data[values..values + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            break;
        }
    }
    let error = match image_load(&data) {
        Ok(_) => panic!("out-of-range JPEG table pointers must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("outside"), "{error}");
}

#[test]
fn jpeg_tiles_continue_across_when_tile_height_equals_image_height() {
    assert_tiled_pixels(32, 16);
}

#[test]
fn jpeg_tiles_continue_across_and_clip_when_tile_height_exceeds_image_height() {
    assert_tiled_pixels(29, 11);
}

#[test]
fn jpeg_tiles_clip_partial_bottom_row() {
    assert_tiled_pixels(16, 29);
}

#[test]
fn jpeg_tiles_preserve_two_by_two_order_and_partial_edges() {
    assert_tiled_pixels(32, 32);
    assert_tiled_pixels(29, 27);
}
