#![cfg(feature = "tiff")]

//! Small, deterministic TIFF regression vectors for palette and fax pages.

use wml2::draw::image_load;

#[derive(Clone)]
struct Entry {
    tag: u16,
    ty: u16,
    count: u32,
    payload: Vec<u8>,
    offset: Option<usize>,
}

fn u16v(value: u16, be: bool) -> [u8; 2] {
    if be {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    }
}
fn u32v(value: u32, be: bool) -> [u8; 4] {
    if be {
        value.to_be_bytes()
    } else {
        value.to_le_bytes()
    }
}
fn shorts(values: &[u16], be: bool) -> Vec<u8> {
    values.iter().flat_map(|v| u16v(*v, be)).collect()
}
fn entry(tag: u16, ty: u16, count: u32, payload: Vec<u8>) -> Entry {
    Entry {
        tag,
        ty,
        count,
        payload,
        offset: None,
    }
}

fn write_entry(out: &mut Vec<u8>, e: &Entry, be: bool) {
    out.extend_from_slice(&u16v(e.tag, be));
    out.extend_from_slice(&u16v(e.ty, be));
    out.extend_from_slice(&u32v(e.count, be));
    if e.payload.len() <= 4 {
        out.extend_from_slice(&e.payload);
        out.resize(out.len() + 4 - e.payload.len(), 0);
    } else {
        out.extend_from_slice(&u32v(e.offset.unwrap() as u32, be));
    }
}

fn build_tiff(
    width: u32,
    height: u32,
    bits: u16,
    photo: u16,
    compression: u16,
    image: Vec<u8>,
    extra: Option<Vec<u8>>,
    be: bool,
) -> Vec<u8> {
    // Intentionally place Orientation before the image tags to exercise order-independent IFD parsing.
    let mut fields = vec![
        entry(274, 3, 1, shorts(&[1], be)),
        entry(257, 4, 1, u32v(height, be).to_vec()),
        entry(273, 4, 1, vec![0; 4]),
        entry(256, 4, 1, u32v(width, be).to_vec()),
        entry(258, 3, 1, shorts(&[bits], be)),
        entry(262, 3, 1, shorts(&[photo], be)),
        entry(259, 3, 1, shorts(&[compression], be)),
        entry(277, 3, 1, shorts(&[1], be)),
        entry(278, 4, 1, u32v(height, be).to_vec()),
        entry(279, 4, 1, u32v(image.len() as u32, be).to_vec()),
    ];
    if let Some(color_map) = extra {
        fields.push(entry(320, 3, (color_map.len() / 2) as u32, color_map));
    }
    let ifd_len = 2 + fields.len() * 12 + 4;
    let mut out = vec![0; 8 + ifd_len];
    let mut cursor = out.len();
    for field in &mut fields {
        if field.payload.len() > 4 {
            if cursor & 1 != 0 {
                cursor += 1;
            }
            field.offset = Some(cursor);
            cursor += field.payload.len();
        }
    }
    let image_offset = cursor;
    for field in &mut fields {
        if field.tag == 273 {
            field.payload = u32v(image_offset as u32, be).to_vec();
        }
    }
    out.resize(image_offset + image.len(), 0);
    for field in &fields {
        if let Some(at) = field.offset {
            out[at..at + field.payload.len()].copy_from_slice(&field.payload);
        }
    }
    out[image_offset..].copy_from_slice(&image);
    let mut ifd = Vec::with_capacity(ifd_len);
    ifd.extend_from_slice(&u16v(fields.len() as u16, be));
    for field in &fields {
        write_entry(&mut ifd, field, be);
    }
    ifd.extend_from_slice(&[0; 4]);
    out[8..8 + ifd.len()].copy_from_slice(&ifd);
    out[0..2].copy_from_slice(if be { b"MM" } else { b"II" });
    out[2..4].copy_from_slice(&u16v(42, be));
    out[4..8].copy_from_slice(&u32v(8, be));
    out
}

fn bits(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, bit) in text.bytes().enumerate() {
        if index % 8 == 0 {
            out.push(0);
        }
        if bit == b'1' {
            *out.last_mut().unwrap() |= 1 << (7 - index % 8);
        }
    }
    out
}

fn palette_map(be: bool) -> Vec<u8> {
    let mut map = vec![0u16; 256 * 3];
    map[0] = u16::MAX;
    map[256 + 1] = u16::MAX;
    shorts(&map, be)
}

#[test]
fn palette_colormap_is_read_as_16bit_entries_in_both_byte_orders() {
    for be in [false, true] {
        let bytes = build_tiff(2, 1, 8, 3, 1, vec![0, 1], Some(palette_map(be)), be);
        let image = image_load(&bytes).unwrap();
        assert_eq!(image.buffer.unwrap(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }
}

#[test]
fn ccitt_huffman_rle_decodes_a_white_scanline() {
    // White run length 8 is 10011 in the CCITT white terminating table.
    let bytes = build_tiff(8, 1, 1, 0, 2, bits("10011"), None, false);
    let image = image_load(&bytes).unwrap();
    assert_eq!(image.buffer.unwrap(), vec![255; 8 * 4]);
}

#[test]
fn ccitt_group3_1d_decodes_a_white_scanline() {
    // Group 3 1D starts with EOL (000000000001), followed by a white run of 8.
    let bytes = build_tiff(8, 1, 1, 0, 3, bits("00000000000110011"), None, false);
    let image = image_load(&bytes).unwrap();
    assert_eq!(image.buffer.unwrap(), vec![255; 8 * 4]);
}

#[test]
fn ccitt_group4_horizontal_mode_decodes_a_white_scanline() {
    // Group 4 horizontal mode 001, white run 8 (10011), black run 0 (0000110111).
    let bytes = build_tiff(8, 1, 1, 0, 4, bits("001100110000110111"), None, false);
    let image = image_load(&bytes).unwrap();
    assert_eq!(image.buffer.unwrap(), vec![255; 8 * 4]);
}
