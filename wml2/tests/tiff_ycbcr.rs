#![cfg(feature = "tiff")]

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

fn rationals(values: &[(u32, u32)], be: bool) -> Vec<u8> {
    values
        .iter()
        .flat_map(|(n, d)| u32v(*n, be).into_iter().chain(u32v(*d, be)))
        .collect()
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

fn write_entry(out: &mut Vec<u8>, entry: &Entry, be: bool) {
    out.extend_from_slice(&u16v(entry.tag, be));
    out.extend_from_slice(&u16v(entry.ty, be));
    out.extend_from_slice(&u32v(entry.count, be));
    if entry.payload.len() <= 4 {
        out.extend_from_slice(&entry.payload);
        out.resize(out.len() + 4 - entry.payload.len(), 0);
    } else {
        out.extend_from_slice(&u32v(entry.offset.unwrap() as u32, be));
    }
}

fn build_ycbcr_tiff(
    width: u32,
    height: u32,
    subsampling: (u16, u16),
    positioning: u16,
    predictor: Option<u16>,
    samples: Vec<u8>,
) -> Vec<u8> {
    build_ycbcr_tiff_with(
        width,
        height,
        subsampling,
        positioning,
        predictor,
        samples,
        &[(299, 1000), (587, 1000), (114, 1000)],
        &[(0, 1), (255, 1), (128, 1), (255, 1), (128, 1), (255, 1)],
    )
}

fn build_ycbcr_tiff_with(
    width: u32,
    height: u32,
    subsampling: (u16, u16),
    positioning: u16,
    predictor: Option<u16>,
    samples: Vec<u8>,
    coefficient_values: &[(u32, u32)],
    reference_values: &[(u32, u32)],
) -> Vec<u8> {
    let be = false;
    let coefficients = rationals(coefficient_values, be);
    let reference = rationals(reference_values, be);
    let mut fields = vec![
        entry(256, 4, 1, u32v(width, be).to_vec()),
        entry(257, 4, 1, u32v(height, be).to_vec()),
        entry(258, 3, 3, shorts(&[8, 8, 8], be)),
        entry(259, 3, 1, shorts(&[1], be)),
        entry(262, 3, 1, shorts(&[6], be)),
        entry(273, 4, 1, vec![0; 4]),
        entry(277, 3, 1, shorts(&[3], be)),
        entry(278, 4, 1, u32v(height, be).to_vec()),
        entry(279, 4, 1, u32v(samples.len() as u32, be).to_vec()),
        entry(529, 5, 3, coefficients),
        entry(530, 3, 2, shorts(&[subsampling.0, subsampling.1], be)),
        entry(531, 3, 1, shorts(&[positioning], be)),
        entry(532, 5, 6, reference),
    ];
    if let Some(predictor) = predictor {
        fields.push(entry(317, 3, 1, shorts(&[predictor], be)));
    }

    let ifd_len = 2 + fields.len() * 12 + 4;
    let mut output = vec![0; 8 + ifd_len];
    let mut cursor = output.len();
    for field in &mut fields {
        if field.payload.len() > 4 {
            cursor += cursor & 1;
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
    output.resize(image_offset + samples.len(), 0);
    for field in &fields {
        if let Some(offset) = field.offset {
            output[offset..offset + field.payload.len()].copy_from_slice(&field.payload);
        }
    }
    output[image_offset..].copy_from_slice(&samples);

    let mut ifd = Vec::with_capacity(ifd_len);
    ifd.extend_from_slice(&u16v(fields.len() as u16, be));
    for field in &fields {
        write_entry(&mut ifd, field, be);
    }
    ifd.extend_from_slice(&[0; 4]);
    output[8..8 + ifd.len()].copy_from_slice(&ifd);
    output[0..2].copy_from_slice(b"II");
    output[2..4].copy_from_slice(&u16v(42, be));
    output[4..8].copy_from_slice(&u32v(8, be));
    output
}

fn rgb(y: u8, cb: u8, cr: u8) -> [u8; 4] {
    let y = f32::from(y);
    let cb = f32::from(cb) - 128.0;
    let cr = f32::from(cr) - 128.0;
    let r = (y + 1.402 * cr).round().clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * cb - 0.714136 * cr)
        .round()
        .clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * cb).round().clamp(0.0, 255.0) as u8;
    [r, g, b, 255]
}

#[test]
fn ycbcr_1x1_converts_each_pixel_and_restores_predictor() {
    let encoded = [100, 128, 128, 10, 2, 248];
    let bytes = build_ycbcr_tiff(2, 1, (1, 1), 1, Some(2), encoded.to_vec());
    let image = image_load(&bytes).unwrap();
    assert_eq!(
        image.buffer.unwrap(),
        [rgb(100, 128, 128), rgb(110, 130, 120)].concat()
    );
}

#[test]
fn ycbcr_2x2_expands_one_chroma_pair_to_the_unit() {
    let bytes = build_ycbcr_tiff(2, 2, (2, 2), 1, None, vec![80, 90, 100, 110, 128, 200]);
    let image = image_load(&bytes).unwrap();
    let pixel = rgb(80, 128, 200);
    assert_eq!(
        image.buffer.unwrap(),
        [
            pixel,
            rgb(90, 128, 200),
            rgb(100, 128, 200),
            rgb(110, 128, 200)
        ]
        .concat()
    );
}

#[test]
fn ycbcr_2x1_accepts_cosited_positioning() {
    let bytes = build_ycbcr_tiff(2, 1, (2, 1), 2, None, vec![80, 90, 128, 200]);
    let image = image_load(&bytes).unwrap();
    assert_eq!(
        image.buffer.unwrap(),
        [rgb(80, 128, 200), rgb(90, 128, 200)].concat()
    );
}

#[test]
fn ycbcr_uses_coefficients_and_reference_ranges() {
    let bytes = build_ycbcr_tiff_with(
        2,
        1,
        (1, 1),
        1,
        None,
        vec![16, 128, 128, 235, 240, 240],
        &[(1, 4), (1, 2), (1, 4)],
        &[(16, 1), (235, 1), (128, 1), (240, 1), (128, 1), (240, 1)],
    );
    let image = image_load(&bytes).unwrap();
    assert_eq!(image.buffer.unwrap(), [0, 0, 0, 255, 255, 65, 255, 255]);
}
