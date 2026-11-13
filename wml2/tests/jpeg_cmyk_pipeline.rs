#![cfg(feature = "jpeg")]
use wml2::draw::{image_from, image_load_with_warnings};
use wml2::metadata::DataMap;
const BASE: &[u8] = include_bytes!("fixtures/review/jpeg-cmyk-blocks.jpg");
const PROGRESSIVE: &[u8] = include_bytes!("fixtures/review/jpeg-cmyk-blocks-progressive.jpg");
fn append_app(jpeg: &[u8], marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = jpeg[..2].to_vec();
    out.extend([255, marker]);
    out.extend(((payload.len() + 2) as u16).to_be_bytes());
    out.extend(payload);
    out.extend(&jpeg[2..]);
    out
}
#[test]
fn nonuniform_cmyk_baseline_and_progressive_declared_ids_match() {
    for jpeg in [BASE, PROGRESSIVE] {
        let image = image_from(jpeg).unwrap();
        assert_eq!((image.width, image.height), (16, 16));
        let data = image.buffer.unwrap();
        for y in 0..16 {
            for x in 0..16 {
                let block = (y / 8 * 2 + x / 8) as u8;
                assert_eq!(
                    &data[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4],
                    &[165 + block * 12, 185 - block * 8, 195 - block * 10, 255]
                );
            }
        }
        assert!(
            matches!(image.metadata.unwrap().get("Color Space"), Some(DataMap::Ascii(s)) if s == "RGB")
        );
    }
}
#[test]
fn short_adobe_payloads_are_ancillary_not_panics() {
    let adobe = b"Adobe\0\x64\0\0\0\0\0";
    for n in 0..12 {
        let image = image_from(&append_app(BASE, 0xee, &adobe[..n])).unwrap();
        assert_eq!(image.buffer, image_from(BASE).unwrap().buffer);
    }
}
#[test]
fn malformed_icc_association_warns_and_preserves_legacy_pixels() {
    let malformed = append_app(BASE, 0xe2, b"ICC_PROFILE\0\x02\x01bad");
    let (image, warnings) = image_load_with_warnings(&malformed).unwrap();
    assert_eq!(image.buffer, image_from(BASE).unwrap().buffer);
    assert!(format!("{:?}", warnings).contains("Invalid ICC APP2"));
}
#[test]
fn existing_real_cmyk_progressive_fixture_decodes() {
    let image = image_from(include_bytes!(
        "fixtures/review/jpeg-icc-cmyk-progressive.jpg"
    ))
    .unwrap();
    assert!(
        image
            .buffer
            .as_ref()
            .unwrap()
            .chunks_exact(4)
            .all(|p| p == [159, 127, 95, 255])
    );
}

#[cfg(feature = "color-management")]
#[path = "common/cmyk_profile.rs"]
mod profiles;

#[cfg(feature = "color-management")]
fn tagged(jpeg: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
    payload.extend(profiles::cmyk_profile());
    append_app(jpeg, 0xe2, &payload)
}

#[cfg(feature = "color-management")]
#[test]
fn embedded_icc_transforms_canonical_adobe_and_unmarked_samples() {
    use icc_profile::{Profile, Transform, TransformOptions};
    let profile = profiles::cmyk_profile();
    let source = Profile::parse(&profile).unwrap();
    let destination = Profile::parse(include_bytes!("../src/srgb.icc")).unwrap();
    if !profiles::supports_cmyk() {
        for jpeg in [BASE, PROGRESSIVE] {
            for adobe in [true, false] {
                let unmarked = [&jpeg[..2], &jpeg[18..]].concat();
                let plain = if adobe { jpeg } else { &unmarked };
                let (image, warnings) = image_load_with_warnings(&tagged(plain)).unwrap();
                assert_eq!(
                    warnings
                        .unwrap()
                        .iter()
                        .filter(|w| w.to_string().starts_with("CMYK color management:"))
                        .count(),
                    1
                );
                assert_eq!(image.buffer, image_from(plain).unwrap().buffer);
                assert!(!image.metadata.unwrap().contains_key("ICC Profile"));
            }
        }
        return;
    }
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    for jpeg in [BASE, PROGRESSIVE] {
        for adobe in [true, false] {
            // Generated APP14 is the first segment, 16 bytes including marker.
            let unmarked = [&jpeg[..2], &jpeg[18..]].concat();
            let data = tagged(if adobe { jpeg } else { &unmarked });
            let image = image_from(&data).unwrap();
            let pixels = image.buffer.as_ref().unwrap();
            for y in 0..16 {
                for x in 0..16 {
                    let b = (y / 8 * 2 + x / 8) as u8;
                    let mut ink = [180 + b * 12, 200 - b * 8, 210 - b * 10, 240];
                    if adobe {
                        ink.iter_mut().for_each(|v| *v = 255 - *v);
                    }
                    let mut rgb = [0; 3];
                    transform.transform_u8(&ink, &mut rgb).unwrap();
                    assert_eq!(&pixels[(y * 16 + x) * 4..(y * 16 + x) * 4 + 3], &rgb);
                    assert_eq!(pixels[(y * 16 + x) * 4 + 3], 255);
                }
            }
            let metadata = image.metadata.unwrap();
            assert!(
                matches!(metadata.get("Source ICC Profile"),Some(DataMap::ICCProfile(p)) if p == &profile)
            );
            assert!(!metadata.contains_key("ICC Profile"));
        }
    }
}

#[cfg(feature = "color-management")]
#[test]
fn ycck_icc_normalizes_before_transform() {
    use icc_profile::{Profile, Transform, TransformOptions};
    // Existing independently encoded constant YCCK fixture has DCT samples
    // Y=90, Cb=149, Cr=109, K=223 (quantized from its original CMYK patch).
    let jpeg = include_bytes!("fixtures/review/jpeg-icc-ycck.jpg");
    if !profiles::supports_cmyk() {
        let (image, warnings) = image_load_with_warnings(&tagged(jpeg)).unwrap();
        assert_eq!(
            warnings
                .unwrap()
                .iter()
                .filter(|w| w.to_string().starts_with("CMYK color management:"))
                .count(),
            1
        );
        assert_eq!(image.buffer, image_from(jpeg).unwrap().buffer);
        return;
    }
    let image = image_from(&tagged(jpeg)).unwrap();
    let bytes = profiles::cmyk_profile();
    let source = Profile::parse(&bytes).unwrap();
    let dest = Profile::parse(include_bytes!("../src/srgb.icc")).unwrap();
    let transform = Transform::new(&source, &dest, TransformOptions::default()).unwrap();
    let mut rgb = [0; 3];
    transform
        .transform_u8(&[64, 96, 127, 32], &mut rgb)
        .unwrap();
    // Integer IDCT/chroma rounding permits one ink code from the encoded source.
    for p in image.buffer.unwrap().chunks_exact(4) {
        assert!(
            p[..3].iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= 2),
            "{p:?} vs {rgb:?}"
        );
        assert_eq!(p[3], 255);
    }
}

#[cfg(all(feature = "color-management", feature = "png"))]
#[test]
#[ignore = "manual visual artifact and timing generation"]
fn jpeg_cmyk_visual_benchmark() {
    use std::time::Instant;
    std::fs::create_dir_all("../.test-jpeg-validation").unwrap();
    for (name, jpeg) in [
        ("baseline", BASE.to_vec()),
        ("progressive", PROGRESSIVE.to_vec()),
        ("icc-baseline", tagged(BASE)),
        ("icc-progressive", tagged(PROGRESSIVE)),
        (
            "ycck",
            include_bytes!("fixtures/review/jpeg-icc-ycck.jpg").to_vec(),
        ),
        (
            "icc-ycck",
            tagged(include_bytes!("fixtures/review/jpeg-icc-ycck.jpg")),
        ),
    ] {
        let start = Instant::now();
        for _ in 0..100 {
            std::hint::black_box(image_from(&jpeg).unwrap());
        }
        eprintln!("{name}: {} us/decode", start.elapsed().as_micros() / 100);
        let mut image = image_from(&jpeg).unwrap();
        let png = wml2::draw::image_to(&mut image, wml2::util::ImageFormat::Png, None).unwrap();
        std::fs::write(format!("../.test-jpeg-validation/{name}.png"), png).unwrap();
    }
}
