use wml2::draw::{
    CallbackResponse, DecodeOptions, DrawCallback, DrawOptions, InitOptions, NextOptions,
    PickCallback, TerminateOptions, VerboseOptions, image_loader,
};
use wml2::metadata::DataMap;

fn be16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn be32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn bei16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn bei32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn packbits_row(row: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in row.chunks(128) {
        out.push((chunk.len() - 1) as u8);
        out.extend_from_slice(chunk);
    }
    out
}

fn predicted(raw: &[u8], width: usize, height: usize, channels: usize, depth: u16) -> Vec<u8> {
    let row_bytes = width * usize::from(depth / 8);
    let mut out = Vec::with_capacity(raw.len());
    for row in raw.chunks_exact(row_bytes).take(height * channels) {
        let shuffled = if depth == 16 {
            let mut value = Vec::with_capacity(row.len());
            value.extend((0..width).map(|x| row[x * 2]));
            value.extend((0..width).map(|x| row[x * 2 + 1]));
            value
        } else {
            row.to_vec()
        };
        out.push(shuffled[0]);
        for index in 1..shuffled.len() {
            out.push(shuffled[index].wrapping_sub(shuffled[index - 1]));
        }
    }
    out
}

fn compressed_payload(
    compression: u16,
    raw: &[u8],
    width: usize,
    height: usize,
    channels: usize,
    depth: u16,
) -> Vec<u8> {
    match compression {
        0 => raw.to_vec(),
        1 => {
            let row_bytes = width * usize::from(depth / 8);
            let rows: Vec<Vec<u8>> = raw.chunks_exact(row_bytes).map(packbits_row).collect();
            let mut out = Vec::new();
            for row in &rows {
                be16(&mut out, row.len() as u16);
            }
            for row in rows {
                out.extend_from_slice(&row);
            }
            assert_eq!(raw.len() / row_bytes, height * channels);
            out
        }
        2 => miniz_oxide::deflate::compress_to_vec_zlib(raw, 6),
        3 => miniz_oxide::deflate::compress_to_vec_zlib(
            &predicted(raw, width, height, channels, depth),
            6,
        ),
        _ => unreachable!(),
    }
}

fn resource(id: u16, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"8BIM");
    be16(&mut out, id);
    out.extend_from_slice(&[0, 0]);
    be32(&mut out, data.len() as u32);
    out.extend_from_slice(data);
    if data.len() % 2 != 0 {
        out.push(0);
    }
    out
}

struct LayerSpec<'a> {
    top: i32,
    left: i32,
    width: usize,
    height: usize,
    name: &'a str,
    visible: bool,
    opacity: u8,
    blend: [u8; 4],
    channels: Vec<(i16, Vec<u8>)>,
    compression: u16,
}

fn layer_section(specs: &[LayerSpec<'_>], depth: u16) -> Vec<u8> {
    let mut records = Vec::new();
    let mut channel_data = Vec::new();
    for spec in specs {
        bei32(&mut records, spec.top);
        bei32(&mut records, spec.left);
        bei32(&mut records, spec.top + spec.height as i32);
        bei32(&mut records, spec.left + spec.width as i32);
        be16(&mut records, spec.channels.len() as u16);
        let mut encoded_channels = Vec::new();
        for (id, raw) in &spec.channels {
            let payload =
                compressed_payload(spec.compression, raw, spec.width, spec.height, 1, depth);
            let mut encoded = Vec::new();
            be16(&mut encoded, spec.compression);
            encoded.extend_from_slice(&payload);
            bei16(&mut records, *id);
            be32(&mut records, encoded.len() as u32);
            encoded_channels.push(encoded);
        }
        records.extend_from_slice(b"8BIM");
        records.extend_from_slice(&spec.blend);
        records.push(spec.opacity);
        records.push(0);
        records.push(if spec.visible { 0 } else { 2 });
        records.push(0);

        let mut extra = Vec::new();
        be32(&mut extra, 0);
        be32(&mut extra, 0);
        let fallback = b"layer";
        extra.push(fallback.len() as u8);
        extra.extend_from_slice(fallback);
        while extra.len() % 4 != 0 {
            extra.push(0);
        }
        let mut unicode = Vec::new();
        let utf16: Vec<u16> = spec.name.encode_utf16().collect();
        be32(&mut unicode, utf16.len() as u32);
        for value in utf16 {
            be16(&mut unicode, value);
        }
        extra.extend_from_slice(b"8BIMluni");
        be32(&mut extra, unicode.len() as u32);
        extra.extend_from_slice(&unicode);
        if unicode.len() % 2 != 0 {
            extra.push(0);
        }
        be32(&mut records, extra.len() as u32);
        records.extend_from_slice(&extra);
        for encoded in encoded_channels {
            channel_data.extend_from_slice(&encoded);
        }
    }
    let mut layer_info = Vec::new();
    bei16(&mut layer_info, specs.len() as i16);
    layer_info.extend_from_slice(&records);
    layer_info.extend_from_slice(&channel_data);
    if layer_info.len() % 2 != 0 {
        layer_info.push(0);
    }
    let mut section = Vec::new();
    be32(&mut section, layer_info.len() as u32);
    section.extend_from_slice(&layer_info);
    section
}

fn psd(
    width: usize,
    height: usize,
    depth: u16,
    mode: u16,
    planes: &[Vec<u8>],
    compression: u16,
    color_data: &[u8],
    resources: &[u8],
    layers: &[u8],
) -> Vec<u8> {
    let raw: Vec<u8> = planes.iter().flatten().copied().collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"8BPS");
    be16(&mut out, 1);
    out.extend_from_slice(&[0; 6]);
    be16(&mut out, planes.len() as u16);
    be32(&mut out, height as u32);
    be32(&mut out, width as u32);
    be16(&mut out, depth);
    be16(&mut out, mode);
    be32(&mut out, color_data.len() as u32);
    out.extend_from_slice(color_data);
    be32(&mut out, resources.len() as u32);
    out.extend_from_slice(resources);
    be32(&mut out, layers.len() as u32);
    out.extend_from_slice(layers);
    be16(&mut out, compression);
    out.extend_from_slice(&compressed_payload(
        compression,
        &raw,
        width,
        height,
        planes.len(),
        depth,
    ));
    out
}

#[test]
fn decodes_rgb_alpha_for_all_compressions() {
    let planes = vec![
        vec![255, 0, 10, 20],
        vec![0, 255, 30, 40],
        vec![0, 0, 255, 60],
        vec![255, 128, 64, 0],
    ];
    let expected = vec![
        255, 0, 0, 255, 0, 255, 0, 128, 10, 30, 255, 64, 20, 40, 60, 0,
    ];
    for compression in 0..=3 {
        let image =
            wml2::draw::image_load(&psd(2, 2, 8, 3, &planes, compression, &[], &[], &[])).unwrap();
        assert_eq!(image.buffer.unwrap(), expected, "compression {compression}");
    }
}

#[test]
fn decodes_16_bit_grayscale_indexed_and_cmyk() {
    let gray = psd(
        2,
        1,
        16,
        1,
        &[vec![0x00, 0x00, 0xff, 0xff]],
        3,
        &[],
        &[],
        &[],
    );
    assert_eq!(
        wml2::draw::image_load(&gray).unwrap().buffer.unwrap(),
        [0, 0, 0, 255, 255, 255, 255, 255]
    );

    let mut palette = vec![0; 768];
    palette[1] = 10;
    palette[257] = 20;
    palette[513] = 30;
    let indexed = psd(1, 1, 8, 2, &[vec![1]], 1, &palette, &[], &[]);
    assert_eq!(
        wml2::draw::image_load(&indexed).unwrap().buffer.unwrap(),
        [10, 20, 30, 255]
    );

    let cmyk = psd(
        1,
        1,
        8,
        4,
        &[vec![255], vec![0], vec![0], vec![255]],
        2,
        &[],
        &[],
        &[],
    );
    assert_eq!(
        wml2::draw::image_load(&cmyk).unwrap().buffer.unwrap(),
        [255, 0, 0, 255]
    );
}

#[test]
fn exposes_raster_layers_without_encoding_them_as_animation() {
    let layers = layer_section(
        &[LayerSpec {
            top: -1,
            left: 1,
            width: 1,
            height: 1,
            name: "前景",
            visible: false,
            opacity: 128,
            blend: *b"mul ",
            channels: vec![
                (0, vec![100]),
                (1, vec![110]),
                (2, vec![120]),
                (-1, vec![200]),
            ],
            compression: 1,
        }],
        8,
    );
    let mut image = wml2::draw::image_load(&psd(
        1,
        1,
        8,
        3,
        &[vec![1], vec![2], vec![3]],
        0,
        &[],
        &[],
        &layers,
    ))
    .unwrap();
    assert_eq!(image.buffer.as_deref(), Some(&[1, 2, 3, 255][..]));
    let frames = image.animation.as_ref().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!((frames[0].start_x, frames[0].start_y), (1, -1));
    assert_eq!(frames[0].buffer, [100, 110, 120, 100]);
    let metadata = image.metadata.as_ref().unwrap();
    assert_eq!(
        metadata.get("wml2.psd.layer.0.name"),
        Some(&DataMap::I18NString("前景".to_string()))
    );
    assert_eq!(
        metadata.get("wml2.psd.layer.0.visible"),
        Some(&DataMap::UInt(0))
    );
    assert_eq!(
        metadata.get("wml2.psd.layer.0.blend_mode"),
        Some(&DataMap::Ascii("mul ".to_string()))
    );

    let profile = PickCallback::encode_start(&mut image, None)
        .unwrap()
        .unwrap();
    assert!(
        !profile
            .metadata
            .unwrap()
            .contains_key("wml2.animation.frames")
    );
}

#[test]
fn extracts_psd_resources() {
    let mut resources = resource(1039, &[1, 2, 3]);
    resources.extend_from_slice(&resource(1058, &[4, 5]));
    resources.extend_from_slice(&resource(1060, b"<x:xmpmeta/>"));
    let image = wml2::draw::image_load(&psd(
        1,
        1,
        8,
        3,
        &[vec![1], vec![2], vec![3]],
        0,
        &[],
        &resources,
        &[],
    ))
    .unwrap();
    let metadata = image.metadata.unwrap();
    assert_eq!(
        metadata.get("ICC Profile"),
        Some(&DataMap::ICCProfile(vec![1, 2, 3]))
    );
    assert_eq!(metadata.get("EXIF Raw"), Some(&DataMap::Raw(vec![4, 5])));
    assert_eq!(
        metadata.get("XMP"),
        Some(&DataMap::Ascii("<x:xmpmeta/>".to_string()))
    );
}

#[test]
fn rejects_psb_unsupported_modes_and_truncation() {
    let valid = psd(1, 1, 8, 3, &[vec![1], vec![2], vec![3]], 0, &[], &[], &[]);
    let mut psb = valid.clone();
    psb[4..6].copy_from_slice(&2u16.to_be_bytes());
    assert!(
        wml2::draw::image_load(&psb)
            .err()
            .unwrap()
            .to_string()
            .contains("PSB")
    );

    let mut depth = valid.clone();
    depth[22..24].copy_from_slice(&32u16.to_be_bytes());
    assert!(wml2::draw::image_load(&depth).is_err());

    let mut mode = valid.clone();
    mode[24..26].copy_from_slice(&9u16.to_be_bytes());
    assert!(wml2::draw::image_load(&mode).is_err());

    for cut in 0..valid.len() {
        assert!(wml2::draw::image_load(&valid[..cut]).is_err());
    }
}

#[test]
fn rejects_corrupt_compression_lengths_and_dimensions() {
    let planes = [vec![1], vec![2], vec![3]];

    let mut oversized = psd(1, 1, 8, 3, &planes, 0, &[], &[], &[]);
    oversized[18..22].copy_from_slice(&30_001u32.to_be_bytes());
    assert!(wml2::draw::image_load(&oversized).is_err());

    let mut rle = psd(1, 1, 8, 3, &planes, 1, &[], &[], &[]);
    // The first composite RLE byte count follows the 38-byte empty-section header
    // and the two-byte compression field.
    rle[40..42].copy_from_slice(&u16::MAX.to_be_bytes());
    assert!(wml2::draw::image_load(&rle).is_err());

    let mut zip = psd(1, 1, 8, 3, &planes, 2, &[], &[], &[]);
    let last = zip.len() - 1;
    zip[last] ^= 0xff;
    assert!(wml2::draw::image_load(&zip).is_err());

    let missing_composite = psd(1, 1, 8, 3, &planes, 0, &[], &[], &[]);
    assert!(wml2::draw::image_load(&missing_composite[..38]).is_err());

    let layers = layer_section(
        &[LayerSpec {
            top: 0,
            left: 0,
            width: 1,
            height: 1,
            name: "layer",
            visible: true,
            opacity: 255,
            blend: *b"norm",
            channels: vec![(0, vec![1]), (1, vec![2]), (2, vec![3])],
            compression: 0,
        }],
        8,
    );
    let mut invalid_channel_length = psd(1, 1, 8, 3, &planes, 0, &[], &[], &layers);
    // First channel length: file header/empty sections (38), layer-info length
    // and count (6), rectangle/count/id (20).
    invalid_channel_length[64..68].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(wml2::draw::image_load(&invalid_channel_length).is_err());
}

#[derive(Default)]
struct AbortOnComposite {
    draws: usize,
    nexts: usize,
    terminates: usize,
}

impl DrawCallback for AbortOnComposite {
    fn init(
        &mut self,
        _: usize,
        _: usize,
        _: Option<InitOptions>,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    fn draw(
        &mut self,
        _: usize,
        _: usize,
        _: usize,
        _: usize,
        _: &[u8],
        _: Option<DrawOptions>,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        self.draws += 1;
        Ok(Some(CallbackResponse::abort()))
    }

    fn terminate(
        &mut self,
        _: Option<TerminateOptions>,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        self.terminates += 1;
        Ok(None)
    }

    fn next(
        &mut self,
        _: Option<NextOptions>,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        self.nexts += 1;
        Ok(None)
    }

    fn verbose(
        &mut self,
        _: &str,
        _: Option<VerboseOptions>,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    fn set_metadata(
        &mut self,
        _: &str,
        _: DataMap,
    ) -> Result<Option<CallbackResponse>, Box<dyn std::error::Error>> {
        Ok(None)
    }
}

#[test]
fn callback_abort_skips_layers_and_terminate() {
    let data = psd(1, 1, 8, 3, &[vec![1], vec![2], vec![3]], 0, &[], &[], &[]);
    let mut drawer = AbortOnComposite::default();
    let mut options = DecodeOptions {
        debug_flag: 0,
        drawer: &mut drawer,
    };
    image_loader(&data, &mut options).unwrap();
    assert_eq!(drawer.draws, 1);
    assert_eq!(drawer.nexts, 0);
    assert_eq!(drawer.terminates, 0);
}

#[test]
fn public_capability_reports_psd() {
    let data = psd(1, 1, 8, 3, &[vec![1], vec![2], vec![3]], 0, &[], &[], &[]);
    assert!(wml2::get_can_decode(&data).unwrap());
    assert!(
        wml2::get_decoder_extentions()
            .iter()
            .any(|ext| ext == "psd")
    );
}
