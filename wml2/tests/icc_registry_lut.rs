#![cfg(feature = "color-management")]

use wml2::highres::icc::{FrameTransform, TransformOptions};
use wml2::highres::{
    ColorInformationSet, FrameMetadata, ImageDescriptor, ImageFrame, PixelBuffer, Plane,
};
use wml2::metadata::DataMap;

#[test]
fn published_cms_preserves_midgray_through_a_lab_pcs_lut_pair() {
    let source_icc = include_bytes!("fixtures/review/lab-pcs-rgb.icc");
    let source_color = ColorInformationSet::new()
        .with_icc_profile(source_icc.to_vec())
        .unwrap();
    let descriptor = ImageDescriptor::rgb(1, 1, 16)
        .unwrap()
        .with_color_information(source_color.clone());
    let planes = descriptor
        .planes()
        .iter()
        .map(|plane| Plane::new(plane.layout().clone(), vec![32768u16]).unwrap())
        .collect();
    let mut metadata = FrameMetadata::new(source_color.clone());
    for key in ["ICC Profile", "Source ICC Profile"] {
        metadata
            .tags_mut()
            .insert(key.into(), DataMap::ICCProfile(source_icc.to_vec()));
    }
    let frame = ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap())
        .unwrap()
        .with_metadata(metadata.clone());
    let source_pixels = frame.pixels().clone();
    let mut destination_icc = source_icc.to_vec();
    // Give the destination a distinct, nonsemantic creator signature so this
    // also verifies active destination bytes and original source provenance.
    destination_icc[80..84].copy_from_slice(b"WML2");
    let transform =
        FrameTransform::new(source_icc, &destination_icc, TransformOptions::default()).unwrap();
    let output = transform.transform(&frame).unwrap();

    // Independent LittleCMS 2.16 RGB16 reference, relative intent, BPC off.
    // Published 0.0.6 incorrectly returned [458, 182, 294] for this input.
    let reference = [32753u16, 32772, 32778];
    let actual: Vec<u16> = output
        .pixels()
        .u16_planes()
        .unwrap()
        .iter()
        .map(|plane| plane.samples()[0])
        .collect();
    for (&actual, expected) in actual.iter().zip(reference) {
        assert!(
            actual.abs_diff(expected) <= 32,
            "paired Lab-PCS LUT midgray: {actual}, expected {expected}"
        );
    }
    assert_eq!(frame.pixels(), &source_pixels);
    assert_eq!(frame.descriptor().color_information(), &source_color);
    assert_eq!(frame.metadata().source_color(), &source_color);
    assert_eq!(frame.metadata(), &metadata);
    assert_eq!(output.metadata().source_color(), &source_color);
    for (key, expected) in [
        ("ICC Profile", destination_icc.as_slice()),
        ("Source ICC Profile", source_icc.as_slice()),
    ] {
        assert!(
            matches!(output.metadata().tags().get(key),
            Some(DataMap::ICCProfile(profile)) if profile == expected),
            "incorrect {key} after explicit conversion"
        );
    }
    assert_eq!(
        output.descriptor().color_information().icc_profile(),
        Some(destination_icc.as_slice())
    );
    assert!(
        output
            .descriptor()
            .planes()
            .iter()
            .all(|plane| plane.meaningful_bits() == 16)
    );

    let without_tags = frame
        .clone()
        .with_metadata(FrameMetadata::new(source_color));
    let converted = transform.transform(&without_tags).unwrap();
    assert!(!converted.metadata().tags().contains_key("ICC Profile"));
    assert_eq!(
        converted.descriptor().color_information().icc_profile(),
        Some(destination_icc.as_slice())
    );
}
