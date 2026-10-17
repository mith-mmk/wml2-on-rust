#![cfg(all(feature = "jpeg2000", feature = "image-buffer-ex"))]
#[allow(dead_code)]
#[path = "../../wml2-test/scripts/jpeg2000_ex_checks.rs"]
mod checks;
#[path = "../../wml2-test/scripts/jpeg2000_ex_color_checks.rs"]
mod color;
use std::path::{Path, PathBuf};
use wml2::draw::*;
use wml2::limits::DecodeLimits;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/review")
}
fn read(name: &str) -> Vec<u8> {
    std::fs::read(root().join(name)).unwrap()
}

#[test]
fn all_color_sources_independent_pgx_channels_rct_alpha_and_strides_match() {
    let manifest = std::fs::read_to_string(root().join("jpeg2000-ex-color-manifest.txt")).unwrap();
    let mut count = 0;
    for line in manifest.lines() {
        let f: Vec<_> = line.split_whitespace().collect();
        let bits: u8 = f[2].parse().unwrap();
        let w: usize = f[3].parse().unwrap();
        let h: usize = f[4].parse().unwrap();
        let nc: usize = f[5].parse().unwrap();
        let expected = read(f[1]);
        for c in 0..nc {
            let pgx = read(&f[1].replace(".rgba16le", &format!("_{c}.pgx")));
            let start = pgx.iter().position(|&v| v == b'\n').unwrap() + 1;
            assert_eq!(
                std::str::from_utf8(&pgx[..start]).unwrap().trim(),
                format!("PG ML + {bits} {w} {h}")
            );
            for (i, v) in pgx[start..].chunks_exact(2).enumerate() {
                assert_eq!(
                    u16::from_be_bytes([v[0], v[1]]).to_le_bytes(),
                    expected[i * 8 + c * 2..i * 8 + c * 2 + 2]
                );
            }
        }
        if nc == 3 {
            for v in expected.chunks_exact(8) {
                assert_eq!(
                    u16::from_le_bytes([v[6], v[7]]),
                    ((1u32 << bits) - 1) as u16
                );
            }
        }
        color::pixels(&read(f[0]), &expected, w, h, bits);
        count += 1;
    }
    assert_eq!(count, 104);
}

#[test]
fn color_callbacks_resource_limits_and_legacy_rejection_remain_explicit() {
    for bits in [12, 16] {
        for kind in ["rgb", "rgba"] {
            let data = read(&format!("jp2_ex_{kind}{bits}_edges_mct1.jp2"));
            let expected = read(&format!("jp2_ex_{kind}{bits}_edges.rgba16le"));
            color::callbacks_and_limits(&data, &expected, 9, 7, bits);
            let mut legacy = ImageBuffer::new();
            assert!(
                image_decoder_ex(
                    &mut bin_rs::reader::BytesReader::new(&data),
                    &mut DecodeOptions {
                        debug_flag: 0,
                        drawer: &mut legacy
                    }
                )
                .is_err()
            );
            assert!(legacy.buffer.is_none());
        }
    }
}

#[test]
fn normalized_exact_round_include_straight_alpha_and_transparent_rgb() {
    for bits in [12, 16] {
        let data = read(&format!("jp2_ex_rgba{bits}_edges_mct1.jp2"));
        let expected = read(&format!("jp2_ex_rgba{bits}_edges.rgba16le"));
        assert!(
            expected
                .chunks_exact(8)
                .any(|v| v[6..] == [0, 0] && v[..6].iter().any(|&v| v != 0))
        );
        assert!(image_from_ex(&data, PixelFormatEx::Rgba8, PrecisionConversion::Exact).is_err());
        let image = image_from_ex(&data, PixelFormatEx::Rgba8, PrecisionConversion::Round).unwrap();
        let max = (1u32 << bits) - 1;
        let scaled: Vec<_> = expected
            .chunks_exact(2)
            .map(|v| ((u32::from(u16::from_le_bytes([v[0], v[1]])) * 255 + max / 2) / max) as u8)
            .collect();
        assert_eq!(image.bytes().unwrap(), scaled);
        assert!(image_from_ex(&data, PixelFormatEx::Gray16, PrecisionConversion::Round).is_err());
    }
}

#[test]
fn color_large_origins_and_32bit_canvas_arithmetic_preserve_samples() {
    for bits in [12, 16] {
        for kind in ["rgb", "rgba"] {
            for mct in [0, 1] {
                color::origins_and_geometry(
                    &read(&format!("jp2_ex_{kind}{bits}_noise_mct{mct}.jp2")),
                    &read(&format!("jp2_ex_{kind}{bits}_noise.rgba16le")),
                    17,
                    13,
                    bits,
                );
            }
        }
    }
}

fn rejected(data: &[u8]) {
    let mut p = checks::Probe::with_format(PixelFormatEx::Rgba16);
    assert!(checks::decode(data, &mut p, DecodeLimits::default()).is_err());
    assert!(p.events.is_empty());
}

fn channel_definitions(data: &[u8], groups: &[&[(u16, u16, u16)]]) -> Vec<u8> {
    fn boxed(kind: &[u8], data: &[u8]) -> Vec<u8> {
        [
            ((data.len() + 8) as u32).to_be_bytes().as_slice(),
            kind,
            data,
        ]
        .concat()
    }
    let mut out = Vec::new();
    let mut p = 0;
    while p < data.len() {
        let len = u32::from_be_bytes(data[p..p + 4].try_into().unwrap()) as usize;
        let kind = &data[p + 4..p + 8];
        let mut payload = data[p + 8..p + len].to_vec();
        if kind == b"jp2h" {
            let mut header = Vec::new();
            let mut q = 0;
            while q < payload.len() {
                let n = u32::from_be_bytes(payload[q..q + 4].try_into().unwrap()) as usize;
                if &payload[q + 4..q + 8] != b"cdef" {
                    header.extend_from_slice(&payload[q..q + n]);
                }
                q += n;
            }
            for definitions in groups {
                let mut v = (definitions.len() as u16).to_be_bytes().to_vec();
                for &(index, kind, association) in *definitions {
                    for field in [index, kind, association] {
                        v.extend_from_slice(&field.to_be_bytes());
                    }
                }
                header.extend(boxed(b"cdef", &v));
            }
            payload = header;
        }
        out.extend(boxed(kind, &payload));
        p += len;
    }
    out
}

#[test]
fn incomplete_ambiguous_or_nonstraight_cdef_is_rejected_before_initialization() {
    let data = read("jp2_ex_rgba16_noise_mct1.jp2");
    let canonical = [(0, 0, 1), (1, 0, 2), (2, 0, 3), (3, 1, 0)];
    color::pixels(
        &channel_definitions(&data, &[&canonical]),
        &read("jp2_ex_rgba16_noise.rgba16le"),
        17,
        13,
        16,
    );
    for groups in [
        vec![],
        vec![&[][..]],
        vec![&canonical[..3]],
        vec![&[][..], &canonical[..]],
        vec![&canonical[..], &canonical[..]],
    ] {
        rejected(&channel_definitions(&data, &groups));
    }
    for alpha in [(3, 2, 0), (3, 1, 1), (0, 1, 0), (3, 65535, 65535)] {
        let mut defs = canonical;
        defs[3] = alpha;
        rejected(&channel_definitions(&data, &[&defs]));
    }
    let alpha_first = [(1, 0, 1), (2, 0, 2), (3, 0, 3), (0, 1, 0)];
    rejected(&channel_definitions(&data, &[&alpha_first]));
}

#[test]
fn unsupported_color_precision_transform_and_alpha_declarations_emit_no_callbacks() {
    for nc in [3, 4] {
        for reason in ["signed16", "depth10"] {
            rejected(&read(&format!(
                "jp2_ex_color_unsupported_{nc}_{reason}.jp2"
            )));
        }
    }
    let raw = read("jp2_ex_rgb16_noise_mct1.j2k");
    let siz = raw.windows(2).position(|v| v == [255, 0x51]).unwrap();
    let cod = raw.windows(2).position(|v| v == [255, 0x52]).unwrap();
    for (offset, value) in [
        (siz + 43, 11),
        (siz + 43, 0x8f),
        (siz + 44, 2),
        (cod + 13, 0),
    ] {
        let mut d = raw.clone();
        d[offset] = value;
        rejected(&d);
    }
    let jp2 = read("jp2_ex_rgba16_noise_mct1.jp2");
    let cdef = jp2.windows(4).position(|v| v == b"cdef").unwrap();
    // Complete colour definitions plus trailing whole-image straight alpha.
    for (offset, value) in [
        (cdef + 27, 2),
        (cdef + 29, 1),
        (cdef + 25, 0),
        (cdef + 11, 0),
        (cdef + 17, 1),
    ] {
        let mut d = jp2.clone();
        d[offset] = value;
        rejected(&d);
    }
    let colr = jp2.windows(4).position(|v| v == b"colr").unwrap();
    for (offset, value) in [(colr + 4, 2), (colr + 10, 18)] {
        let mut d = jp2.clone();
        d[offset] = value;
        rejected(&d);
    }
    for ext in ["j2k", "jp2"] {
        rejected(&read(&format!("jp2_ex_unsupported_rgba16.{ext}")));
    }
}
