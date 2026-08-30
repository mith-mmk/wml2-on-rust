use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use wml2::highres::avif::{
    AvifSequenceFrame, AvifSequenceInformation, AvifSequenceReader, NativeDecodeLimits,
};
use wml2::highres::{ChannelModel, ChannelRole, ResourceLimits};

fn generated_sequence_fixture() -> Option<Vec<u8>> {
    static FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        ".test-wml2-highres-avis-{}-{}",
        std::process::id(),
        FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).ok()?;
    let output = root.join("sequence.avifs");
    let status = Command::new("ffmpeg")
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

fn resource_limits(input: usize, max_total_live: usize) -> ResourceLimits {
    resource_limits_with_frame_count(input, max_total_live, 1024)
}

fn resource_limits_with_frame_count(
    input: usize,
    max_total_live: usize,
    max_frame_count: usize,
) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(input)
        .max_width(8192)
        .max_height(8192)
        .max_pixels(1 << 28)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 30)
        .max_frame_bytes(1 << 30)
        .max_total_live_decoded_bytes(max_total_live)
        .max_references(128)
        .max_frame_count(max_frame_count)
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

fn required_avis_fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should exist")
        .join("test/images/external/avif/unsupported")
        .join(name);
    assert!(
        path.is_file(),
        "required AVIS fixture is missing: {}",
        path.display()
    );
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("required AVIS fixture cannot be read: {path:?}: {error}"))
}

fn assert_native_sequence_planes(
    frame: &AvifSequenceFrame,
    expected: &[(ChannelRole, u32, u32)],
    bit_depth: u8,
) {
    let descriptor = frame.frame().descriptor();
    let planes = frame
        .frame()
        .pixels()
        .u16_planes()
        .expect("strict AVIS highres output must retain U16 planes");
    assert_eq!(planes.len(), descriptor.planes().len());
    assert_eq!(planes.len(), expected.len());
    assert_eq!(descriptor.model(), ChannelModel::YCbCr);
    for (index, (plane, plane_descriptor)) in planes.iter().zip(descriptor.planes()).enumerate() {
        let (role, width, height) = expected[index];
        assert_eq!(plane_descriptor.roles(), &[role], "plane {index} role");
        assert_eq!(
            (plane_descriptor.layout().width(), plane_descriptor.layout().height()),
            (width, height),
            "plane {index} dimensions"
        );
        assert_eq!(
            plane_descriptor.meaningful_bits(),
            bit_depth,
            "plane {index} depth"
        );
        assert_eq!(plane.layout().width(), plane_descriptor.layout().width());
        assert_eq!(plane.layout().height(), plane_descriptor.layout().height());
        let minimum_samples = plane
            .layout()
            .row_stride()
            .checked_mul(plane.layout().height() as usize)
            .expect("plane sample count should not overflow");
        assert_eq!(
            plane.samples().len(),
            minimum_samples,
            "plane {index} native sample count"
        );
        assert!(
            plane
                .samples()
                .iter()
                .all(|sample| *sample < (1u16 << bit_depth)),
            "plane {index} contains a sample outside its native bit depth"
        );
    }
}

#[test]
fn sequence_reader_public_surface_is_additive_and_native() {
    let _reader: fn(
        &[u8],
        NativeDecodeLimits,
        ResourceLimits,
    ) -> Result<AvifSequenceReader, wml2::highres::avif::DecodeError> = AvifSequenceReader::new;
    let _frame: fn(&AvifSequenceFrame) -> u64 = AvifSequenceFrame::index;
    let _information: fn(&AvifSequenceInformation) -> u64 = |information| information.frame_count();
}

#[test]
fn sequence_reader_preserves_index_timing_and_stable_eof() {
    let Some(data) = generated_sequence_fixture() else {
        eprintln!("ffmpeg/libaom unavailable; skipping AVIS reader integration");
        return;
    };
    let limits = resource_limits(data.len(), 1 << 30);
    let mut reader = AvifSequenceReader::new(&data, native_limits(data.len()), limits)
        .expect("strict AVIS fixture should construct");
    let information = reader.information();
    assert!(information.frame_count() > 0);
    assert!(information.timescale() > 0);
    let mut count = 0u64;
    while let Some(frame) = reader.next_frame().expect("native AVIS frame should map") {
        assert_eq!(frame.index(), count);
        assert_eq!(
            frame.frame().timing().map(|timing| timing.timescale()),
            Some(information.timescale())
        );
        count += 1;
    }
    assert_eq!(count, information.frame_count());
    assert!(
        reader
            .next_frame()
            .expect("EOF must remain stable")
            .is_none()
    );
}

#[test]
fn sequence_mapping_error_keeps_cursor_retryable() {
    let Some(data) = generated_sequence_fixture() else {
        eprintln!("ffmpeg/libaom unavailable; skipping AVIS rollback integration");
        return;
    };
    // The native decoder is now admitted under the same aggregate ceiling as
    // the bridge.  Find the first small ceiling that admits the retained
    // container state but rejects the mapped frame, rather than relying on a
    // limit (such as one byte) that must correctly fail at construction.
    let mut reader = None;
    for max_total_live in [
        29, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
    ] {
        let limits = resource_limits(data.len(), max_total_live);
        let Ok(candidate) = AvifSequenceReader::new(&data, native_limits(data.len()), limits)
        else {
            continue;
        };
        let mut candidate = candidate;
        if candidate.next_frame().is_err() {
            reader = Some(candidate);
            break;
        }
    }
    let mut reader = reader.expect("a bounded bridge ceiling should reject mapping");
    let first = reader
        .next_frame()
        .expect_err("the bounded highres budget must reject mapping");
    let second = reader
        .next_frame()
        .expect_err("the rejected frame must remain retryable");
    assert_eq!(first, second);
    assert_eq!(reader.information().frame_count(), 4);
}

#[test]
fn sequence_resource_limits_are_applied_before_native_construction() {
    let Some(data) = generated_sequence_fixture() else {
        eprintln!("ffmpeg/libaom unavailable; skipping AVIS construction limits");
        return;
    };

    let frame_limited = resource_limits_with_frame_count(data.len(), 1 << 30, 1);
    let frame_error = match AvifSequenceReader::new(&data, native_limits(data.len()), frame_limited)
    {
        Ok(_) => panic!("native sequence construction must enforce max_frame_count"),
        Err(error) => error,
    };
    assert!(matches!(
        frame_error,
        wml2::highres::avif::DecodeError::Codec(_)
    ));

    let zero_live = resource_limits(data.len(), 0);
    let zero_error = match AvifSequenceReader::new(&data, native_limits(data.len()), zero_live) {
        Ok(_) => panic!("zero aggregate budget must fail before codec construction"),
        Err(error) => error,
    };
    assert!(matches!(
        zero_error,
        wml2::highres::avif::DecodeError::Processing(_)
    ));
}

#[test]
fn official_animated_12bit_sequence_retains_native_yuv422_and_timing() {
    let data = required_avis_fixture("colors-animated-12bpc-keyframes-0-2-3.avif");
    let mut reader = AvifSequenceReader::new(
        &data,
        native_limits(data.len()),
        resource_limits(data.len(), 1 << 30),
    )
    .expect("official 12-bit AVIS fixture should construct");
    let information = reader.information();
    assert_eq!(information.frame_count(), 5);
    assert!(information.timescale() > 0);
    assert!(information.duration_in_timescales() > 0);

    let mut frames = Vec::new();
    while let Some(frame) = reader
        .next_frame()
        .expect("official 12-bit AVIS frame should decode")
    {
        frames.push(frame);
    }
    assert_eq!(frames.len(), 5);
    let expected_planes = [
        (ChannelRole::Y, 64, 64),
        (ChannelRole::Cb, 32, 64),
        (ChannelRole::Cr, 32, 64),
        (ChannelRole::Alpha, 64, 64),
    ];
    let mut previous_pts = None;
    let mut total_duration = 0u64;
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.index(), index as u64);
        assert_native_sequence_planes(frame, &expected_planes, 12);
        let timing = frame
            .frame()
            .timing()
            .expect("frame timing should be retained");
        assert_eq!(timing.timescale(), information.timescale());
        assert!(timing.duration() > 0);
        if let Some(previous_pts) = previous_pts {
            assert!(timing.pts() >= previous_pts);
        }
        previous_pts = Some(timing.pts());
        assert!(timing.pts() < information.duration_in_timescales());
        total_duration = total_duration
            .checked_add(timing.duration())
            .expect("animation duration sum should not overflow");
    }
    assert_eq!(
        total_duration,
        information.duration_in_timescales(),
        "frame durations should cover the advertised animation duration"
    );
    assert!(reader.next_frame().expect("EOF should be stable").is_none());
    assert!(
        reader
            .next_frame()
            .expect("EOF should remain stable")
            .is_none()
    );
}

#[test]
fn official_animated_alpha_sequence_retains_gray_alpha_and_metadata_timing() {
    let data = required_avis_fixture("colors-animated-8bpc-alpha-exif-xmp.avif");
    let mut reader = AvifSequenceReader::new(
        &data,
        native_limits(data.len()),
        resource_limits(data.len(), 1 << 30),
    )
    .expect("official alpha AVIS fixture should construct");
    let information = reader.information();
    assert_eq!(information.frame_count(), 5);
    assert!(information.timescale() > 0);
    assert!(information.duration_in_timescales() > 0);

    let expected_planes = [
        (ChannelRole::Y, 150, 150),
        (ChannelRole::Cb, 75, 75),
        (ChannelRole::Cr, 75, 75),
        (ChannelRole::Alpha, 150, 150),
    ];
    let mut previous_pts = None;
    let mut total_duration = 0u64;
    for expected_index in 0..information.frame_count() {
        let frame = reader
            .next_frame()
            .expect("official alpha AVIS frame should decode")
            .expect("official alpha AVIS sequence should have all frames");
        assert_eq!(frame.index(), expected_index);
        assert_native_sequence_planes(&frame, &expected_planes, 8);
        let descriptor = frame.frame().descriptor();
        let alpha_index = descriptor
            .planes()
            .iter()
            .position(|plane| plane.roles().contains(&ChannelRole::Alpha))
            .expect("official alpha fixture must retain a Gray alpha plane");
        assert_eq!(
            descriptor.planes()[alpha_index].roles(),
            &[ChannelRole::Alpha]
        );
        assert!(frame.frame().metadata().source_color().av1().is_some());
        let timing = frame
            .frame()
            .timing()
            .expect("frame timing should be retained");
        assert_eq!(timing.timescale(), information.timescale());
        assert!(timing.duration() > 0);
        assert!(timing.pts() < information.duration_in_timescales());
        if let Some(previous_pts) = previous_pts {
            assert!(timing.pts() >= previous_pts);
        }
        previous_pts = Some(timing.pts());
        total_duration = total_duration
            .checked_add(timing.duration())
            .expect("animation duration sum should not overflow");
    }
    assert_eq!(
        total_duration,
        information.duration_in_timescales(),
        "frame durations should cover the advertised animation duration"
    );
    assert!(reader.next_frame().expect("EOF should be stable").is_none());
    assert!(
        reader
            .next_frame()
            .expect("EOF should remain stable")
            .is_none()
    );
}
