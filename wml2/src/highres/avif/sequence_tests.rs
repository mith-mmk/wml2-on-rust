use super::{AvifSequenceReader, DecodeError, NativeDecodeLimits};
use crate::highres::{ProcessingError, ResourceLimits};

fn generated_sequence_fixture() -> Option<Vec<u8>> {
    let root = std::env::temp_dir().join(format!(
        ".test-wml2-highres-avis-unit-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).ok()?;
    let output = root.join("sequence.avifs");
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error"])
        .args(["-f", "lavfi", "-i", "color=c=red:size=64x64:rate=1"])
        .args(["-frames:v", "4", "-c:v", "libaom-av1"])
        .args([
            "-still-picture",
            "0",
            "-g",
            "1",
            "-lag-in-frames",
            "0",
            "-auto-alt-ref",
            "0",
            "-f",
            "avif",
        ])
        .arg(&output)
        .status()
        .ok()?;
    let result = status.success().then(|| std::fs::read(&output).ok())??;
    let _ = std::fs::remove_dir_all(root);
    Some(result)
}

fn native_limits(input: usize) -> NativeDecodeLimits {
    NativeDecodeLimits::new(
        input,
        8192,
        8192,
        1 << 28,
        1 << 30,
        1 << 30,
        1 << 30,
        128,
        1024,
        1 << 24,
        128,
        1024,
    )
}

fn resource_limits(input: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(input)
        .max_width(8192)
        .max_height(8192)
        .max_pixels(1 << 28)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 30)
        .max_frame_bytes(1 << 30)
        .max_total_live_decoded_bytes(1 << 30)
        .max_references(128)
        .max_frame_count(1024)
        .max_metadata_bytes(1 << 24)
        .max_icc_bytes(1 << 24)
        .max_clut_bytes(1 << 28)
        .max_parser_entries(1 << 20)
        .max_parser_depth(128)
        .max_grid_cells(1 << 20)
        .max_derived_work(1 << 28)
        .max_derived_depth(64)
        .build()
        .expect("sequence limits are valid")
}

#[test]
fn effective_native_projection_intersects_shared_fields_only() {
    let native =
        NativeDecodeLimits::new(201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212)
            .with_max_live_allocation_bytes(9)
            .expect("native live limit should be valid");
    let mut resource = ResourceLimits::builder()
        .max_input_bytes(101)
        .max_width(102)
        .max_height(103)
        .max_pixels(104)
        .max_channels(105)
        .max_planes(106)
        .max_plane_bytes(107)
        .max_frame_bytes(108)
        .max_total_live_decoded_bytes(109)
        .max_references(110)
        .max_frame_count(111)
        .max_metadata_bytes(112)
        .max_icc_bytes(113)
        .max_clut_bytes(114)
        .max_parser_entries(115)
        .max_parser_depth(116)
        .max_grid_cells(117)
        .max_derived_work(118)
        .max_derived_depth(119)
        .build()
        .expect("projection limits are valid");
    resource.max_total_live_decoded_bytes = 109;
    let effective = AvifSequenceReader::effective_native_limits(native, resource)
        .expect("non-zero resource limit should project");
    assert_eq!(effective.max_input_bytes(), 101);
    assert_eq!(effective.max_width(), 102);
    assert_eq!(effective.max_height(), 103);
    assert_eq!(effective.max_pixels(), 104);
    assert_eq!(effective.max_plane_bytes(), 107);
    assert_eq!(effective.max_metadata_bytes(), 112);
    assert_eq!(effective.max_icc_bytes(), 113);
    assert_eq!(effective.max_items(), 208);
    assert_eq!(effective.max_properties(), 209);
    assert_eq!(effective.max_grid_cells(), 117);
    assert_eq!(effective.max_derived_depth(), 119);
    assert_eq!(effective.max_frames(), 111);
    let expected =
        NativeDecodeLimits::new(101, 102, 103, 104, 107, 112, 113, 208, 209, 117, 119, 111)
            .with_max_live_allocation_bytes(9)
            .expect("expected native live limit should be valid");
    assert_eq!(effective, expected);

    // A second row in the opposite direction guards against accidentally
    // treating the resource projection as an overwrite rather than an
    // intersection.  Item/property counts are deliberately never projected
    // because ResourceLimits has no equivalent native parser fields.
    let native = NativeDecodeLimits::new(11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)
        .with_max_live_allocation_bytes(209)
        .expect("native live limit should be valid");
    let mut resource = ResourceLimits::builder()
        .max_input_bytes(101)
        .max_width(102)
        .max_height(103)
        .max_pixels(104)
        .max_channels(105)
        .max_planes(106)
        .max_plane_bytes(107)
        .max_frame_bytes(108)
        .max_total_live_decoded_bytes(109)
        .max_references(110)
        .max_frame_count(111)
        .max_metadata_bytes(112)
        .max_icc_bytes(113)
        .max_clut_bytes(114)
        .max_parser_entries(115)
        .max_parser_depth(116)
        .max_grid_cells(117)
        .max_derived_work(118)
        .max_derived_depth(119)
        .build()
        .expect("projection limits are valid");
    resource.max_total_live_decoded_bytes = 109;
    let effective = AvifSequenceReader::effective_native_limits(native, resource)
        .expect("non-zero resource limit should project");
    assert_eq!(effective.max_input_bytes(), 11);
    assert_eq!(effective.max_width(), 12);
    assert_eq!(effective.max_height(), 13);
    assert_eq!(effective.max_pixels(), 14);
    assert_eq!(effective.max_plane_bytes(), 15);
    assert_eq!(effective.max_metadata_bytes(), 16);
    assert_eq!(effective.max_icc_bytes(), 17);
    assert_eq!(effective.max_items(), 18);
    assert_eq!(effective.max_properties(), 19);
    assert_eq!(effective.max_grid_cells(), 20);
    assert_eq!(effective.max_derived_depth(), 21);
    assert_eq!(effective.max_frames(), 22);
    let expected = NativeDecodeLimits::new(11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)
        .with_max_live_allocation_bytes(109)
        .expect("expected native live limit should be valid");
    assert_eq!(effective, expected);
}

#[test]
fn one_shot_mapping_failure_keeps_reader_retryable() {
    let Some(data) = generated_sequence_fixture() else {
        eprintln!("ffmpeg/libaom unavailable; skipping sequence failpoint test");
        return;
    };
    let mut reader = AvifSequenceReader::new(
        &data,
        native_limits(data.len()),
        resource_limits(data.len()),
    )
    .expect("strict AVIS fixture should construct");

    let failpoint = super::mapping::construction_failpoint(0);
    let error = reader
        .next_frame()
        .expect_err("the one-shot construction failpoint must fail mapping");
    assert!(matches!(
        error,
        DecodeError::Processing(ProcessingError::Allocation(_))
    ));
    drop(failpoint);

    let first = reader
        .next_frame()
        .expect("retry after one-shot mapping failure should succeed")
        .expect("first frame should remain pending");
    assert_eq!(first.index(), 0);
    let first_descriptor = first.frame().descriptor().clone();
    let first_planes: Vec<Vec<u16>> = first
        .frame()
        .pixels()
        .u16_planes()
        .expect("native AVIS bridge retains U16 planes")
        .iter()
        .map(|plane| plane.samples().to_vec())
        .collect();

    // Compare the retry result with a fresh reader.  This guards both the
    // transactional cursor and the native U16 ownership path independently
    // of the second frame's contents.
    let mut baseline_reader = AvifSequenceReader::new(
        &data,
        native_limits(data.len()),
        resource_limits(data.len()),
    )
    .expect("fresh strict AVIS reader should construct");
    let baseline = baseline_reader
        .next_frame()
        .expect("fresh reader frame should decode")
        .expect("fresh reader should expose frame zero");
    assert_eq!(baseline.index(), first.index());
    assert_eq!(baseline.frame().descriptor(), first.frame().descriptor());
    assert_eq!(baseline.frame().timing(), first.frame().timing());
    let baseline_planes: Vec<Vec<u16>> = baseline
        .frame()
        .pixels()
        .u16_planes()
        .expect("fresh native AVIS frame retains U16 planes")
        .iter()
        .map(|plane| plane.samples().to_vec())
        .collect();
    assert_eq!(baseline_planes, first_planes);

    let second = reader
        .next_frame()
        .expect("second frame should decode")
        .expect("second frame should be present");
    assert_eq!(second.index(), 1);
    assert_eq!(second.frame().descriptor(), &first_descriptor);
    assert_eq!(
        second.frame().timing().map(|timing| timing.duration()),
        first.frame().timing().map(|timing| timing.duration())
    );
    let second_planes: Vec<Vec<u16>> = second
        .frame()
        .pixels()
        .u16_planes()
        .expect("native AVIS bridge retains U16 planes")
        .iter()
        .map(|plane| plane.samples().to_vec())
        .collect();
    assert_eq!(second_planes, first_planes);
}
