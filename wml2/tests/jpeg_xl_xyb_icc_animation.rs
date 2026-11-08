#![cfg(feature = "jpeg-xl")]
//! Distinct embedded/source and reconstruction ICCs must never be interchanged.
use wml2::{draw::*, jpegxl::*, metadata::DataMap};

const INPUT: &[u8] = include_bytes!("fixtures/review/jxl-animation-xyb-icc16.jxl");
const GOLD: &[u8] = include_bytes!("fixtures/review/jxl-animation-xyb-icc16.rgba16le");

fn values(bytes: &[u8]) -> impl Iterator<Item = u16> + '_ {
    bytes
        .chunks_exact(2)
        .map(|v| u16::from_le_bytes(v.try_into().unwrap()))
}
fn rounded(v: u16) -> u8 {
    ((u32::from(v) * 255 + 32767) / 65535) as u8
}
fn compare(actual: impl Iterator<Item = u16>) {
    let actual: Vec<_> = actual.collect();
    let gold: Vec<_> = values(GOLD).collect();
    assert_eq!(actual.len(), 3 * 32 * 24 * 4);
    assert_eq!(actual.len(), gold.len());
    assert!(gold.iter().filter(|&&v| v % 257 != 0).count() > 8800);
    for (i, (&a, &b)) in actual.iter().zip(&gold).enumerate() {
        assert!(
            a.abs_diff(b) <= u16::from(i % 4 != 3),
            "sample {i}: {a}/{b}"
        );
    }
}
fn compare8(actual: impl Iterator<Item = u8>) {
    let actual: Vec<_> = actual.collect();
    let gold: Vec<_> = values(GOLD).map(rounded).collect();
    assert_eq!(actual.len(), gold.len());
    for (i, (&a, &b)) in actual.iter().zip(&gold).enumerate() {
        assert!(a.abs_diff(b) <= u8::from(i % 4 != 3), "sample {i}: {a}/{b}");
    }
}

#[test]
fn xyb_icc_native_uses_srgb_reconstruction_not_embedded_coordinates() {
    use jxl_preflight::headers::{FileHeader, JxlHeader};
    let header = FileHeader::read(&mut jxl_preflight::bit_reader::BitReader::new(INPUT)).unwrap();
    assert!(header.image_metadata.xyb_encoded);
    assert!(header.image_metadata.color_encoding.want_icc);
    let image =
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| false).unwrap();
    assert_eq!(
        (image.bits_per_sample, image.alpha_bits_per_sample),
        (16, Some(16))
    );
    assert_ne!(image.source_icc, image.output_icc);
    assert_eq!(image.source_icc.len(), 620);
    assert_eq!(&image.source_icc[16..20], b"RGB ");
    assert_eq!(&image.output_icc[16..20], b"RGB ");
    assert_eq!(
        (
            image.ticks_per_second_numerator,
            image.ticks_per_second_denominator,
            image.loop_count
        ),
        (1000, 1, 0)
    );
    assert_eq!(
        image
            .frames
            .iter()
            .map(|f| f.duration_ticks)
            .collect::<Vec<_>>(),
        [50, 90, 130]
    );
    compare(image.frames.iter().flat_map(|f| f.rgba16.iter().copied()));
}

#[test]
fn xyb_icc_common_and_ex_tag_delivered_samples_with_output_icc() {
    let native =
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| false).unwrap();
    let image = image_from(INPUT).unwrap();
    let metadata = image.metadata.as_ref().unwrap();
    assert!(matches!(&metadata["ICC Profile"], DataMap::ICCProfile(p) if *p == native.output_icc));
    assert!(
        matches!(&metadata["JPEG XL Source ICC Profile"], DataMap::ICCProfile(p) if *p == native.source_icc)
    );
    let frames = image.animation.as_ref().unwrap();
    assert_eq!(image.buffer.as_ref().unwrap(), &frames[0].buffer);
    assert_eq!(
        frames
            .iter()
            .map(|f| f.control.await_time)
            .collect::<Vec<_>>(),
        [50, 90, 130]
    );
    assert_eq!(image.loop_count, Some(0));
    compare8(frames.iter().flat_map(|f| f.buffer.iter().copied()));
    #[cfg(feature = "image-buffer-ex")]
    {
        let ex = image_from_ex(INPUT, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
        assert!(
            matches!(&ex.metadata.as_ref().unwrap()["ICC Profile"], DataMap::ICCProfile(p) if *p == native.output_icc)
        );
        let frames = ex.animation().unwrap();
        assert_eq!(ex.bytes().unwrap(), frames[0].pixels.bytes());
        compare(frames.iter().flat_map(|f| values(f.pixels.bytes())));
    }
}

#[cfg(feature = "color-management")]
#[test]
fn xyb_icc_managed_frames_transform_output_profile_once_and_keep_alpha() {
    let native =
        animation_rgba16_from_with_limits_and_cancel(INPUT, Default::default(), &|| false).unwrap();
    let image = image_from_srgb_with_limits(INPUT, Default::default()).unwrap();
    let metadata = image.metadata.as_ref().unwrap();
    assert!(
        matches!(&metadata["ICC Profile"], DataMap::ICCProfile(p) if p == include_bytes!("../src/srgb.icc"))
    );
    assert!(
        matches!(&metadata["JPEG XL Reconstruction ICC Profile"], DataMap::ICCProfile(p) if *p == native.output_icc)
    );
    assert!(
        matches!(&metadata["JPEG XL Source ICC Profile"], DataMap::ICCProfile(p) if *p == native.source_icc)
    );
    // Using the embedded per-channel gamma curves again would give visibly
    // wrong colors, far beyond this independent one-unit RGB tolerance.
    compare8(
        image
            .animation
            .as_ref()
            .unwrap()
            .iter()
            .flat_map(|f| f.buffer.iter().copied()),
    );
}
