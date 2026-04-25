use std::fs;
use std::path::{Path, PathBuf};

use wml2::draw::image_load;
use wml2::metadata::DataMap;
use wml2::metadata::c2pa::{
    C2PA_JSON_KEY, C2PA_RAW_KEY, PNG_CHUNK_TYPE, jpeg_app11_payload_to_manifest_store,
};
use wml2::metadata::exif::gps_coordinate;

fn sample_dir() -> PathBuf {
    if let Ok(path) = std::env::var("WML2_C2PA_EXIF_SAMPLES_DIR") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".test")
        .join("c2pa+exif")
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<usize> {
    let data: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(data) as usize)
}

fn png_cabx_payload(data: &[u8]) -> Option<Vec<u8>> {
    if data.get(..8) != Some(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a][..]) {
        return None;
    }
    let mut cursor = 8usize;
    while cursor + 12 <= data.len() {
        let length = read_u32_be(data, cursor)?;
        let chunk_type = data.get(cursor + 4..cursor + 8)?;
        let data_start = cursor + 8;
        let data_end = data_start.checked_add(length)?;
        if data_end + 4 > data.len() {
            return None;
        }
        if chunk_type == PNG_CHUNK_TYPE.as_slice() {
            return Some(data[data_start..data_end].to_vec());
        }
        if chunk_type == b"IEND" {
            return None;
        }
        cursor = data_end + 4;
    }
    None
}

fn jpeg_app11_c2pa_payload(data: &[u8]) -> Option<Vec<u8>> {
    if data.get(..2) != Some(&[0xff, 0xd8][..]) {
        return None;
    }
    let mut cursor = 2usize;
    let mut payload = Vec::new();
    while cursor + 4 <= data.len() {
        if data[cursor] != 0xff {
            return None;
        }
        let marker = data[cursor + 1];
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        let length = u16::from_be_bytes(data[cursor + 2..cursor + 4].try_into().ok()?) as usize;
        if length < 2 || cursor + 2 + length > data.len() {
            return None;
        }
        if marker == 0xeb {
            let segment_payload = &data[cursor + 4..cursor + 2 + length];
            if let Some(mut part) = jpeg_app11_payload_to_manifest_store(segment_payload) {
                payload.append(&mut part);
            }
        }
        cursor += 2 + length;
    }
    (!payload.is_empty()).then_some(payload)
}

fn has_jpeg_exif(data: &[u8]) -> bool {
    if data.get(..2) != Some(&[0xff, 0xd8][..]) {
        return false;
    }
    let mut cursor = 2usize;
    while cursor + 10 <= data.len() {
        if data[cursor] != 0xff {
            return false;
        }
        let marker = data[cursor + 1];
        if marker == 0xda || marker == 0xd9 {
            return false;
        }
        let Ok(length) = <[u8; 2]>::try_from(&data[cursor + 2..cursor + 4]) else {
            return false;
        };
        let length = u16::from_be_bytes(length) as usize;
        if length < 2 || cursor + 2 + length > data.len() {
            return false;
        }
        if marker == 0xe1 && data.get(cursor + 4..cursor + 10) == Some(&b"Exif\0\0"[..]) {
            return true;
        }
        cursor += 2 + length;
    }
    false
}

#[test]
fn local_c2pa_exif_samples_decode_expected_metadata_when_present() {
    let dir = sample_dir();
    if !dir.exists() {
        eprintln!("skipping local sample test: {} is missing", dir.display());
        return;
    }

    let mut saw_file = false;
    let mut saw_c2pa = false;
    let mut saw_exif = false;
    let mut saw_gps = false;

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        saw_file = true;
        let data = fs::read(&path).unwrap();
        let expected_c2pa = png_cabx_payload(&data).or_else(|| jpeg_app11_c2pa_payload(&data));
        let expected_exif = has_jpeg_exif(&data);

        let decoded = image_load(&data).unwrap_or_else(|err| {
            panic!("failed to decode {}: {err}", display_path(&path));
        });
        let metadata = decoded.metadata.as_ref().unwrap_or_else(|| {
            panic!("missing metadata for {}", display_path(&path));
        });

        if let Some(expected_c2pa) = expected_c2pa {
            saw_c2pa = true;
            match metadata.get(C2PA_JSON_KEY) {
                Some(DataMap::JSON(json)) => {
                    assert!(
                        json.contains("\"format\":\"c2pa\""),
                        "C2PA JSON missing format for {}",
                        display_path(&path)
                    );
                    assert!(
                        json.contains("\"manifest_store_base64\""),
                        "C2PA JSON missing payload for {}",
                        display_path(&path)
                    );
                }
                other => panic!(
                    "unexpected C2PA JSON metadata for {}: {other:?}",
                    display_path(&path)
                ),
            }
            assert!(matches!(
                metadata.get(C2PA_RAW_KEY),
                Some(DataMap::Raw(raw)) if raw == &expected_c2pa
            ));
        }

        if expected_exif {
            saw_exif = true;
            let headers = match metadata.get("EXIF") {
                Some(DataMap::Exif(headers)) => headers,
                other => panic!(
                    "unexpected EXIF metadata for {}: {other:?}",
                    display_path(&path)
                ),
            };
            if headers.gps.is_some() {
                assert!(
                    gps_coordinate(headers).is_some(),
                    "GPS tags did not parse into decimal coordinates for {}",
                    display_path(&path)
                );
                saw_gps = true;
            }
        }
    }

    assert!(saw_file, "no files found in {}", dir.display());
    assert!(saw_c2pa, "no C2PA sample found in {}", dir.display());
    assert!(saw_exif, "no EXIF sample found in {}", dir.display());
    assert!(saw_gps, "no GPS sample found in {}", dir.display());
}

fn display_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid-name>")
        .to_string()
}
