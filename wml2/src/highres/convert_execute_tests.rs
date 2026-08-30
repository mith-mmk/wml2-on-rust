use super::super::super::output_plan::OwnerElement;
use super::super::super::{
    AlphaAssociation, Av1ColorInformation, ChannelModel, ChannelRole, ColorConvertOptions,
    ColorInformationSet, ColorProvenance, Destination, FrameMetadata, GeometryOperation,
    IccColorType, ImageDescriptor, ImageFrame, MatrixCoefficients, NativeSampleEncoding,
    NclxColorInformation, PixelBuffer, PixelChannelInformation, Plane, PlaneDescriptor,
    PlaneLayout, ProcessingError, ResourceLimits, RgbPrimaries, SampleDomain, SampleRange,
    SourceInterpretation, Subsampling, UnknownColorInformation,
};
use super::{CandidateMaker, OrdinaryMaker, OwnerKey, PreparedConversion, convert_frame};
use std::cell::Cell;
use std::mem::size_of;
use std::sync::{Arc, Barrier, mpsc};

thread_local! {
    static RESTORE_BOUNDARY_COUNT: Cell<usize> = const { Cell::new(0) };
    static RESTORE_TARGET_SNAPSHOT: Cell<Option<bool>> = const { Cell::new(None) };
    static RESTORE_TARGET_DEALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
    static RESTORE_OWNER_STATUS: Cell<Option<(usize, usize, bool)>> = const { Cell::new(None) };
}

fn count_restore_boundary() {
    RESTORE_BOUNDARY_COUNT.with(|count| count.set(count.get() + 1));
    RESTORE_TARGET_SNAPSHOT.with(|snapshot| {
        snapshot.set(Some(
            super::super::native::DeallocationObservationGuard::target_deallocated(),
        ));
        RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| {
            snapshot.set(Some(
                super::super::native::DeallocationObservationGuard::target_deallocations(),
            ));
        });
        RESTORE_OWNER_STATUS.with(|snapshot| {
            snapshot.set(Some(
                super::super::native::DeallocationObservationGuard::registered_status(),
            ));
        });
    });
}

struct RestoreObserverGuard {
    previous: Option<fn()>,
    deallocations: super::super::native::DeallocationObservationGuard,
}

impl RestoreObserverGuard {
    fn begin() -> Self {
        let deallocations = super::super::native::DeallocationObservationGuard::begin();
        let previous = super::super::super::allocation::swap_restore_boundary_observer(Some(
            count_restore_boundary,
        ));
        Self {
            previous,
            deallocations,
        }
    }

    fn boundary_deallocations(&self) -> usize {
        self.deallocations.deallocations()
    }
}

impl Drop for RestoreObserverGuard {
    fn drop(&mut self) {
        super::super::super::allocation::swap_restore_boundary_observer(self.previous);
    }
}

fn limits() -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(64)
        .max_height(64)
        .max_pixels(4096)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 20)
        .max_frame_bytes(1 << 22)
        .max_total_live_decoded_bytes(1 << 24)
        .max_references(16)
        .max_frame_count(16)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(4096)
        .max_parser_depth(32)
        .max_grid_cells(4096)
        .max_derived_work(4096)
        .max_derived_depth(16)
        .build()
        .unwrap()
}

fn rgb_frame(samples: Vec<u8>, width: u32, height: u32, alpha: Option<Vec<u8>>) -> ImageFrame {
    rgb_frame_with_domain(samples, width, height, alpha, SampleDomain::Encoded)
}

fn rgb_frame_with_domain(
    samples: Vec<u8>,
    width: u32,
    height: u32,
    alpha: Option<Vec<u8>>,
    domain: SampleDomain,
) -> ImageFrame {
    let layout = PlaneLayout::planar(width, height, Subsampling::FULL).unwrap();
    let mut descriptors = Vec::new();
    let mut planes = Vec::new();
    for (role, offset) in [
        (ChannelRole::Red, 0usize),
        (ChannelRole::Green, 1usize),
        (ChannelRole::Blue, 2usize),
    ] {
        descriptors.push(PlaneDescriptor::planar(layout.clone(), role, 8).unwrap());
        planes.push(Plane::new(layout.clone(), vec![samples[offset]]).unwrap());
    }
    if let Some(alpha) = alpha {
        descriptors.push(PlaneDescriptor::planar(layout.clone(), ChannelRole::Alpha, 8).unwrap());
        planes.push(Plane::new(layout, alpha).unwrap());
    }
    let descriptor = ImageDescriptor::new(width, height, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(if planes.len() == 4 {
            AlphaAssociation::Straight
        } else {
            AlphaAssociation::None
        })
        .unwrap()
        .with_color_information(
            ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, true)),
        )
        .with_domain(domain);
    ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap()
}

fn bt2020_primaries() -> RgbPrimaries {
    RgbPrimaries::new(
        (0.708, 0.292),
        (0.170, 0.797),
        (0.131, 0.046),
        (0.3127, 0.3290),
    )
    .unwrap()
}

fn u16_rgb_transfer_frame(code: u16, bits: u8, transfer: u16) -> ImageFrame {
    u16_rgb_transfer_frame_with_color(
        code,
        bits,
        ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, transfer, 0, true)),
    )
}

fn u16_rgb_transfer_frame_with_color(
    code: u16,
    bits: u8,
    color: ColorInformationSet,
) -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let descriptors = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
        .into_iter()
        .map(|role| PlaneDescriptor::planar(layout.clone(), role, bits).unwrap())
        .collect();
    let planes = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
        .into_iter()
        .map(|_| Plane::new(layout.clone(), vec![code]).unwrap())
        .collect();
    let descriptor = ImageDescriptor::new(1, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_color_information(color)
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap()).unwrap()
}

fn u16_gray_transfer_frame(code: u16, bits: u8, transfer: u16) -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::gray(1, 1, bits)
        .unwrap()
        .with_color_information(
            ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, transfer, 0, true)),
        )
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(
        descriptor,
        PixelBuffer::u16(vec![Plane::new(layout, vec![code]).unwrap()]).unwrap(),
    )
    .unwrap()
}

fn u16_ycbcr_transfer_frame(y_code: u16, bits: u8, subsampling: Subsampling) -> ImageFrame {
    let width = 5;
    let height = 3;
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, 16, 1, false));
    let y_subsampling = Subsampling::FULL;
    let (chroma_width, chroma_height) = subsampling.dimensions(width, height).unwrap();
    let y_layout = PlaneLayout::planar(width, height, y_subsampling).unwrap();
    let chroma_layout = PlaneLayout::planar(chroma_width, chroma_height, subsampling).unwrap();
    let descriptor = ImageDescriptor::ycbcr(
        width,
        height,
        bits,
        [y_subsampling, subsampling, subsampling],
    )
    .unwrap()
    .with_color_information(color)
    .with_domain(SampleDomain::Encoded);
    let y_count = usize::try_from(width * height).unwrap();
    let chroma_count = usize::try_from(chroma_width * chroma_height).unwrap();
    ImageFrame::new(
        descriptor,
        PixelBuffer::u16(
            [
                Plane::new(y_layout, vec![y_code; y_count]).unwrap(),
                Plane::new(chroma_layout.clone(), vec![128 << (bits - 8); chroma_count]).unwrap(),
                Plane::new(chroma_layout, vec![128 << (bits - 8); chroma_count]).unwrap(),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn gray_frame() -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let mut provenance = Vec::with_capacity(8);
    provenance.push(super::super::super::ColorProvenance::ContainerNclx);
    let descriptor = ImageDescriptor::new(
        1,
        1,
        ChannelModel::Gray,
        vec![PlaneDescriptor::planar(layout.clone(), ChannelRole::Gray, 8).unwrap()],
    )
    .unwrap()
    .with_color_information(ColorInformationSet::from_owned_parts(
        provenance,
        None,
        None,
        Some(NclxColorInformation::new(1, 13, 0, true)),
        None,
        Vec::new(),
    ))
    .with_alpha(AlphaAssociation::None)
    .unwrap()
    .with_domain(SampleDomain::Encoded);
    ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, vec![128]).unwrap()]).unwrap(),
    )
    .unwrap()
}

fn rich_gray_frame() -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let mut provenance = Vec::with_capacity(8);
    provenance.push(super::super::super::ColorProvenance::ContainerNclx);
    let colors = ColorInformationSet::from_owned_parts(
        provenance,
        None,
        None,
        Some(NclxColorInformation::new(1, 13, 0, true)),
        None,
        Vec::new(),
    );
    let descriptor = ImageDescriptor::new(
        1,
        1,
        ChannelModel::Gray,
        vec![PlaneDescriptor::planar(layout.clone(), ChannelRole::Gray, 8).unwrap()],
    )
    .unwrap()
    .with_color_information(colors)
    .with_alpha(AlphaAssociation::None)
    .unwrap()
    .with_domain(SampleDomain::Encoded);
    ImageFrame::new(
        descriptor,
        PixelBuffer::u8(vec![Plane::new(layout, vec![128]).unwrap()]).unwrap(),
    )
    .unwrap()
}

fn bounded_limits(frame: usize, live: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(8)
        .max_height(8)
        .max_pixels(64)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 20)
        .max_frame_bytes(frame)
        .max_total_live_decoded_bytes(live)
        .max_references(8)
        .max_frame_count(8)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(128)
        .max_parser_depth(16)
        .max_grid_cells(64)
        .max_derived_work(64)
        .max_derived_depth(8)
        .build()
        .unwrap()
}

struct C1GrayInventory {
    base: usize,
    active_color: usize,
    source_metadata: usize,
    source_live: usize,
    metadata: usize,
    frame: usize,
    live: usize,
}

fn c1_gray_inventory(frame: &ImageFrame) -> C1GrayInventory {
    let base = size_of::<PlaneDescriptor>()
        .checked_add(size_of::<super::super::super::Plane<u8>>())
        .and_then(|value| value.checked_add(2 * size_of::<usize>()))
        .and_then(|value| value.checked_add(size_of::<ChannelRole>()))
        .and_then(|value| value.checked_add(size_of::<u8>()))
        .unwrap();
    let color = frame.descriptor().color_information();
    let active_color = color
        .provenance_capacity_for_test()
        .checked_mul(size_of::<super::super::super::ColorProvenance>())
        .and_then(|value| value.checked_add(8))
        .unwrap();
    let source_metadata = color
        .provenance()
        .len()
        .checked_mul(size_of::<super::super::super::ColorProvenance>())
        .and_then(|value| value.checked_add(8))
        .unwrap();
    let source_live = base
        .checked_add(active_color)
        .and_then(|value| value.checked_add(source_metadata))
        .unwrap();
    let metadata = size_of::<super::super::super::ColorProvenance>()
        .checked_add(8)
        .unwrap();
    let output_per_plane = size_of::<f32>()
        .checked_add(size_of::<PlaneDescriptor>())
        .and_then(|value| value.checked_add(size_of::<super::super::super::Plane<f32>>()))
        .and_then(|value| value.checked_add(2 * size_of::<usize>()))
        .and_then(|value| value.checked_add(size_of::<ChannelRole>()))
        .unwrap();
    let frame = 3usize
        .checked_mul(output_per_plane)
        .and_then(|value| value.checked_add(metadata))
        .unwrap();
    C1GrayInventory {
        base,
        active_color,
        source_metadata,
        source_live,
        metadata,
        frame,
        live: source_live + frame,
    }
}

fn c1_rich_inventory(frame: &ImageFrame) -> (usize, usize, usize, usize) {
    let color = frame.descriptor().color_information();
    let native_plane = size_of::<PlaneDescriptor>()
        .checked_add(size_of::<super::super::super::Plane<u8>>())
        .and_then(|value| value.checked_add(2 * size_of::<usize>()))
        .and_then(|value| value.checked_add(size_of::<ChannelRole>()))
        .and_then(|value| value.checked_add(size_of::<u8>()))
        .unwrap();
    let active = color
        .provenance_capacity_for_test()
        .checked_mul(size_of::<super::super::super::ColorProvenance>())
        .and_then(|value| value.checked_add(8))
        .unwrap();
    let source = color
        .provenance()
        .len()
        .checked_mul(size_of::<super::super::super::ColorProvenance>())
        .and_then(|value| value.checked_add(8))
        .unwrap();
    let source_live = native_plane
        .checked_add(active)
        .and_then(|value| value.checked_add(source))
        .unwrap();
    let metadata = source;
    let output_plane = size_of::<f32>()
        .checked_add(size_of::<PlaneDescriptor>())
        .and_then(|value| value.checked_add(size_of::<super::super::super::Plane<f32>>()))
        .and_then(|value| value.checked_add(2 * size_of::<usize>()))
        .and_then(|value| value.checked_add(size_of::<ChannelRole>()))
        .unwrap();
    let frame_bytes = 3usize
        .checked_mul(output_plane)
        .and_then(|value| value.checked_add(metadata))
        .unwrap();
    (active, source_live, metadata, frame_bytes)
}

#[test]
fn convert_frame_materializes_linear_planar_rgb_without_reinspecting_plan() {
    let frame = rgb_frame(vec![255, 0, 0], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    assert_eq!(converted.descriptor().model(), ChannelModel::RGB);
    assert_eq!(
        converted.descriptor().domain(),
        SampleDomain::LinearRelative
    );
    assert_eq!(
        converted.descriptor().primaries(),
        Some(RgbPrimaries::srgb())
    );
    let planes = converted.pixels().f32_planes().unwrap();
    assert_eq!(planes.len(), 3);
    assert!((planes[0].samples()[0] - 1.0).abs() < 1e-6);
    assert!(planes[1].samples()[0].abs() < 1e-6);
    assert!(planes[2].samples()[0].abs() < 1e-6);
}

#[test]
fn convert_frame_pq_u16_rgb_supports_8_10_12_bit_and_dark_values() {
    let primaries = bt2020_primaries();
    for bits in [8u8, 10, 12] {
        let max = (1u16 << bits) - 1;
        for code in [0u16, 1, max] {
            let frame = u16_rgb_transfer_frame(code, bits, 16);
            let options = ColorConvertOptions::new(Destination::linear_rgb(
                SampleDomain::LinearAbsoluteNits,
                primaries,
            ));
            let converted = convert_frame(&frame, &options, &limits()).unwrap();
            assert_eq!(
                converted.descriptor().domain(),
                SampleDomain::LinearAbsoluteNits
            );
            let samples = converted.pixels().f32_planes().unwrap();
            let expected_signal = f64::from(code) / f64::from(max);
            let expected = super::pq_eotf_f64(expected_signal, true).unwrap() as f32;
            for plane in samples {
                assert!((plane.samples()[0] - expected).abs() <= 1e-4);
            }
            let record = converted.metadata().last_conversion().unwrap();
            assert_eq!(
                record.destination(),
                super::super::super::metadata::ConversionDestination::LinearAbsoluteNitsRgb
            );
            assert_eq!(
                record.destination_domain(),
                SampleDomain::LinearAbsoluteNits
            );
        }
    }
}

#[test]
fn convert_frame_hlg_u16_gray_preserves_scene_endpoints() {
    let primaries = bt2020_primaries();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::HlgSceneLinear,
        primaries,
    ));
    for code in [0u16, 2048, 4095] {
        let expected =
            super::hlg_scene_from_signal_f64(f64::from(code) / 4095.0, true).unwrap() as f32;
        let frame = u16_gray_transfer_frame(code, 12, 18);
        let converted = convert_frame(&frame, &options, &limits()).unwrap();
        assert_eq!(
            converted.descriptor().domain(),
            SampleDomain::HlgSceneLinear
        );
        assert_eq!(
            converted
                .metadata()
                .last_conversion()
                .unwrap()
                .destination(),
            super::super::super::metadata::ConversionDestination::HlgSceneLinearRgb
        );
        let planes = converted.pixels().f32_planes().unwrap();
        assert_eq!(planes.len(), 3);
        for plane in planes {
            assert!((plane.samples()[0] - expected).abs() <= 1e-5);
        }
    }
}

#[test]
fn convert_frame_pq_u16_ycbcr_limited_420_422_444() {
    let primaries = bt2020_primaries();
    let location = super::super::super::ChromaLocation::from_h273_code(0).unwrap();
    for bits in [8u8, 10, 12] {
        let scale = 1u16 << (bits - 8);
        let max = (1u16 << bits) - 1;
        for subsampling in [
            Subsampling::new(2, 2).unwrap(),
            Subsampling::new(2, 1).unwrap(),
            Subsampling::FULL,
        ] {
            let native_location = (subsampling != Subsampling::FULL).then_some(location);
            let options = ColorConvertOptions::new(Destination::linear_rgb(
                SampleDomain::LinearAbsoluteNits,
                primaries,
            ))
            .with_native_sample_encoding(
                super::super::super::NativeSampleEncoding::new(
                    super::super::super::SampleRange::Limited,
                    super::super::super::MatrixCoefficients::Bt709,
                    native_location,
                ),
            );
            for y_code in [16 * scale, 235 * scale] {
                let frame = u16_ycbcr_transfer_frame(y_code, bits, subsampling);
                let converted = convert_frame(&frame, &options, &limits()).unwrap();
                assert_eq!(
                    converted.descriptor().domain(),
                    SampleDomain::LinearAbsoluteNits
                );
                let expected_signal = if y_code == 16 * scale { 0.0 } else { 1.0 };
                let expected = super::pq_eotf_f64(expected_signal, true).unwrap() as f32;
                for plane in converted.pixels().f32_planes().unwrap() {
                    for sample in plane.samples() {
                        assert!((sample - expected).abs() <= 1e-4);
                    }
                }
                assert!(y_code <= max);
            }
        }
    }
}

fn valid_rgb_profile() -> Vec<u8> {
    let mut profile = vec![0u8; 204];
    let profile_len = profile.len() as u32;
    profile[..4].copy_from_slice(&profile_len.to_be_bytes());
    profile[8] = 4;
    profile[12..16].copy_from_slice(b"mntr");
    profile[16..20].copy_from_slice(b"RGB ");
    profile[20..24].copy_from_slice(b"XYZ ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&1u32.to_be_bytes());
    profile[128..132].copy_from_slice(&6u32.to_be_bytes());
    for index in 0..6 {
        let record = 132 + index * 12;
        profile[record + 4..record + 8].copy_from_slice(&204u32.to_be_bytes());
    }
    profile
}

fn u16_rgb_pq_frame_with_alpha(
    code: u16,
    bits: u8,
    alpha_code: u16,
    association: AlphaAssociation,
) -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, 16, 0, true));
    let descriptors = [
        (ChannelRole::Red, bits),
        (ChannelRole::Green, bits),
        (ChannelRole::Blue, bits),
        (ChannelRole::Alpha, bits),
    ]
    .into_iter()
    .map(|(role, precision)| PlaneDescriptor::planar(layout.clone(), role, precision).unwrap())
    .collect();
    let planes = [code, code, code, alpha_code]
        .into_iter()
        .map(|sample| Plane::new(layout.clone(), vec![sample]).unwrap())
        .collect();
    let descriptor = ImageDescriptor::new(1, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(association)
        .unwrap()
        .with_color_information(color)
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(descriptor, PixelBuffer::u16(planes).unwrap()).unwrap()
}

fn u16_ycbcr_pq_spatial_frame(
    bits: u8,
    subsampling: Subsampling,
    matrix: u16,
    y_code: u16,
) -> ImageFrame {
    let width = 8;
    let height = 8;
    let (chroma_width, chroma_height) = subsampling.dimensions(width, height).unwrap();
    let y_layout = PlaneLayout::planar(width, height, Subsampling::FULL).unwrap();
    let chroma_layout = PlaneLayout::planar(chroma_width, chroma_height, subsampling).unwrap();
    let color = ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, 16, matrix, false));
    let descriptor = ImageDescriptor::ycbcr(
        width,
        height,
        bits,
        [Subsampling::FULL, subsampling, subsampling],
    )
    .unwrap()
    .with_color_information(color)
    .with_domain(SampleDomain::Encoded);
    let scale = 1u16 << (bits - 8);
    let y_count = usize::try_from(width * height).unwrap();
    let cb = (0..chroma_height)
        .flat_map(|y| (0..chroma_width).map(move |x| (136 + 2 * x + 2 * y) as u16 * scale))
        .collect();
    let cr = (0..chroma_height)
        .flat_map(|y| (0..chroma_width).map(move |x| (140 + 3 * x + y) as u16 * scale))
        .collect();
    ImageFrame::new(
        descriptor,
        PixelBuffer::u16(
            [
                Plane::new(y_layout, vec![y_code; y_count]).unwrap(),
                Plane::new(chroma_layout.clone(), cb).unwrap(),
                Plane::new(chroma_layout, cr).unwrap(),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn f32_rgb_transfer_frame(value: f32, transfer: u16) -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let color =
        ColorInformationSet::new().with_nclx(NclxColorInformation::new(9, transfer, 0, true));
    let descriptors = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
        .into_iter()
        .map(|role| PlaneDescriptor::planar(layout.clone(), role, 32).unwrap())
        .collect();
    let planes = [ChannelRole::Red, ChannelRole::Green, ChannelRole::Blue]
        .into_iter()
        .map(|_| Plane::new(layout.clone(), vec![value]).unwrap())
        .collect();
    let descriptor = ImageDescriptor::new(1, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_color_information(color)
        .with_domain(SampleDomain::Encoded);
    ImageFrame::new(descriptor, PixelBuffer::f32(planes).unwrap()).unwrap()
}

#[test]
fn hdr_midpoints_match_independent_f64_references() {
    let pq = convert_frame(
        &u16_rgb_transfer_frame(2048, 12, 16),
        &ColorConvertOptions::new(Destination::linear_rgb(
            SampleDomain::LinearAbsoluteNits,
            bt2020_primaries(),
        )),
        &limits(),
    )
    .unwrap();
    let signal = 2048.0f64 / 4095.0;
    let m1 = 2610.0 / 16384.0;
    let m2 = 2523.0 / 32.0;
    let c1 = 3424.0 / 4096.0;
    let c2 = 2413.0 / 128.0;
    let c3 = 2392.0 / 128.0;
    let p = signal.powf(1.0 / m2);
    let expected_pq = 10000.0 * ((p - c1).max(0.0) / (c2 - c3 * p)).powf(1.0 / m1);
    for plane in pq.pixels().f32_planes().unwrap() {
        assert!((f64::from(plane.samples()[0]) - expected_pq).abs() <= 1e-3);
    }

    let hlg = convert_frame(
        &u16_gray_transfer_frame(2048, 12, 18),
        &ColorConvertOptions::new(Destination::linear_rgb(
            SampleDomain::HlgSceneLinear,
            bt2020_primaries(),
        )),
        &limits(),
    )
    .unwrap();
    let signal = 2048.0f64 / 4095.0;
    let a = 0.17883277f64;
    let b = 1.0 - 4.0 * a;
    let c = 0.5 - a * (4.0 * a).ln();
    let expected_hlg = if signal <= 0.5 {
        signal * signal / 3.0
    } else {
        (((signal - c) / a).exp() + b) / 12.0
    };
    assert!(
        (f64::from(hlg.pixels().f32_planes().unwrap()[0].samples()[0]) - expected_hlg).abs()
            <= 1e-6
    );
}

#[test]
fn explicit_pq_overrides_active_icc_and_preserves_source_profile() {
    let profile = valid_rgb_profile();
    let color = ColorInformationSet::new()
        .with_icc_profile(profile.clone())
        .unwrap()
        .with_icc_color_type(IccColorType::Prof)
        .with_nclx(NclxColorInformation::new(9, 16, 0, true));
    let frame = u16_rgb_transfer_frame_with_color(2048, 12, color.clone())
        .with_metadata(FrameMetadata::new(color));
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearAbsoluteNits,
        bt2020_primaries(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        9, 16, 0, true,
    )))
    .with_native_sample_encoding(NativeSampleEncoding::new(
        SampleRange::Full,
        MatrixCoefficients::Identity,
        None,
    ));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    assert_eq!(
        frame.descriptor().color_information().icc_profile(),
        Some(profile.as_slice())
    );
    assert_eq!(
        converted.metadata().source_color().icc_profile(),
        Some(profile.as_slice())
    );
    let record = converted.metadata().last_conversion().unwrap();
    assert_eq!(
        record.source(),
        super::super::super::metadata::ConversionSource::ExplicitCicp
    );
    assert_eq!(
        record.destination(),
        super::super::super::metadata::ConversionDestination::LinearAbsoluteNitsRgb
    );
    assert_eq!(
        record.destination_domain(),
        SampleDomain::LinearAbsoluteNits
    );
}

#[test]
fn pq_ycbcr_non_neutral_vectors_cover_matrices_and_chroma_phases() {
    let options_for = |matrix, location| {
        ColorConvertOptions::new(Destination::linear_rgb(
            SampleDomain::LinearAbsoluteNits,
            bt2020_primaries(),
        ))
        .with_native_sample_encoding(NativeSampleEncoding::new(
            SampleRange::Limited,
            matrix,
            location,
        ))
    };
    let reference_pq = |signal: f64| {
        let m1 = 2610.0 / 16384.0;
        let m2 = 2523.0 / 32.0;
        let c1 = 3424.0 / 4096.0;
        let c2 = 2413.0 / 128.0;
        let c3 = 2392.0 / 128.0;
        let p = signal.powf(1.0 / m2);
        10000.0 * ((p - c1).max(0.0) / (c2 - c3 * p)).powf(1.0 / m1)
    };
    let cases = [
        (
            MatrixCoefficients::Bt709,
            1u16,
            Subsampling::new(2, 2).unwrap(),
            [
                Some(super::super::super::ChromaLocation::from_h273_code(0).unwrap()),
                Some(super::super::super::ChromaLocation::from_h273_code(1).unwrap()),
            ],
        ),
        (
            MatrixCoefficients::Bt601,
            6u16,
            Subsampling::new(2, 1).unwrap(),
            [
                Some(super::super::super::ChromaLocation::from_h273_code(0).unwrap()),
                Some(super::super::super::ChromaLocation::from_h273_code(1).unwrap()),
            ],
        ),
        (MatrixCoefficients::Bt2020, 9u16, Subsampling::FULL, [None, None]),
    ];
    let bits = 10u8;
    let scale = 1u16 << (bits - 8);
    let y_code = 128 * scale;
    let y = (f64::from(y_code) - 16.0 * f64::from(scale)) / (219.0 * f64::from(scale));
    let interpolate = |
        base: u16,
        x_step: u16,
        y_step: u16,
        x: u32,
        y: u32,
        subsampling: Subsampling,
        location: Option<super::super::super::ChromaLocation>,
    | {
        let plane_width = u32::from(8 / subsampling.x());
        let plane_height = u32::from(8 / subsampling.y());
        let value = |column: usize, row: usize| {
            f64::from((base + x_step * column as u16 + y_step * row as u16) * scale)
        };
        if subsampling == Subsampling::FULL {
            return value(x as usize, y as usize);
        }
        let location = location.expect("subsampled chroma location");
        let phase = |phase| match phase {
            super::super::super::ChromaPhase::Zero => 0.0,
            super::super::super::ChromaPhase::Half => 0.5,
            super::super::super::ChromaPhase::One => 1.0,
        };
        let axis = |index: u32, factor: u8, phase: f64, extent: u32| {
            if factor == 1 {
                return (index as usize, index as usize, 0.0);
            }
            let coordinate = (f64::from(index) - phase) / f64::from(factor);
            let lower = coordinate.floor().clamp(0.0, f64::from(extent - 1)) as usize;
            let upper = (coordinate.floor() + 1.0).clamp(0.0, f64::from(extent - 1)) as usize;
            (lower, upper, coordinate - coordinate.floor())
        };
        let (x0, x1, wx) = axis(
            x,
            subsampling.x(),
            phase(location.x_phase()),
            plane_width,
        );
        let (y0, y1, wy) = axis(
            y,
            subsampling.y(),
            phase(location.y_phase()),
            plane_height,
        );
        value(x0, y0) * (1.0 - wx) * (1.0 - wy)
            + value(x1, y0) * wx * (1.0 - wy)
            + value(x0, y1) * (1.0 - wx) * wy
            + value(x1, y1) * wx * wy
    };
    let mut first_phase: Option<f32> = None;
    for (matrix, matrix_code, subsampling, locations) in cases {
        let mut phase_values = Vec::new();
        for location in locations {
        let (kr, kb) = match matrix {
            MatrixCoefficients::Bt709 => (0.2126, 0.0722),
            MatrixCoefficients::Bt601 => (0.299, 0.114),
            MatrixCoefficients::Bt2020 => (0.2627, 0.0593),
                _ => unreachable!(),
        };
        let cb_code = interpolate(136, 2, 2, 3, 3, subsampling, location);
        let cr_code = interpolate(140, 3, 1, 3, 3, subsampling, location);
        let cb = (cb_code - 128.0 * f64::from(scale)) / (224.0 * f64::from(scale));
        let cr = (cr_code - 128.0 * f64::from(scale)) / (224.0 * f64::from(scale));
        let rgb = [
            y + 2.0 * (1.0 - kr) * cr,
            (y - kr * (y + 2.0 * (1.0 - kr) * cr) - kb * (y + 2.0 * (1.0 - kb) * cb))
                / (1.0 - kr - kb),
            y + 2.0 * (1.0 - kb) * cb,
        ];
        let frame = u16_ycbcr_pq_spatial_frame(bits, subsampling, matrix_code, y_code);
        let converted = convert_frame(&frame, &options_for(matrix, location), &limits()).unwrap();
        let samples = converted.pixels().f32_planes().unwrap();
        let index = 3 * 8 + 3;
        for (plane, expected) in samples.iter().zip(rgb.map(reference_pq)) {
            assert!(
                (f64::from(plane.samples()[index]) - expected).abs() <= 1e-3,
                "matrix={matrix:?} location={location:?} got={} expected={expected}",
                plane.samples()[index]
            );
        }
        let red = samples[0].samples()[index];
        assert!((red - samples[1].samples()[index]).abs() > 1e-4);
        phase_values.push(red);
        }
        if locations[0].is_some() {
            assert!((phase_values[0] - phase_values[1]).abs() > 1e-4);
        }
        if let Some(previous) = first_phase {
            assert!((phase_values[0] - previous).abs() > 1e-4);
        }
        first_phase = Some(phase_values[0]);
    }
}

#[test]
fn hdr_alpha_is_straight_only_and_preserved_without_conversion() {
    let frame = u16_rgb_pq_frame_with_alpha(2048, 12, 1234, AlphaAssociation::Straight);
    let before = frame.clone();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearAbsoluteNits,
        bt2020_primaries(),
    ));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    let alpha = converted.pixels().f32_planes().unwrap()[3].samples()[0];
    assert!((alpha - 1234.0 / 4095.0).abs() <= 1e-6);
    assert_eq!(converted.descriptor().alpha(), AlphaAssociation::Straight);
    assert_eq!(frame, before);

    let premultiplied =
        u16_rgb_pq_frame_with_alpha(2048, 12, 1234, AlphaAssociation::Premultiplied);
    let before = premultiplied.clone();
    assert!(matches!(
        convert_frame(&premultiplied, &options, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
    assert_eq!(premultiplied, before);
}

#[test]
fn invalid_hdr_sample_or_domain_fails_without_mutating_source() {
    let pq = f32_rgb_transfer_frame(1.1, 16);
    let before = pq.clone();
    let absolute = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearAbsoluteNits,
        bt2020_primaries(),
    ));
    assert!(matches!(
        convert_frame(&pq, &absolute, &limits()),
        Err(ProcessingError::Invalid(_))
    ));
    assert_eq!(pq, before);

    let valid_pq = f32_rgb_transfer_frame(0.5, 16);
    let before = valid_pq.clone();
    let wrong_domain = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::HlgSceneLinear,
        bt2020_primaries(),
    ));
    assert!(matches!(
        convert_frame(&valid_pq, &wrong_domain, &limits()),
        Err(ProcessingError::Unsupported(_))
    ));
    assert_eq!(valid_pq, before);

    let hlg = f32_rgb_transfer_frame(1.1, 18);
    let before = hlg.clone();
    let scene = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::HlgSceneLinear,
        bt2020_primaries(),
    ));
    assert!(matches!(
        convert_frame(&hlg, &scene, &limits()),
        Err(ProcessingError::Invalid(_))
    ));
    assert_eq!(hlg, before);
}

#[test]
fn c1_rich_inventory_uses_visible_provenance_capacity_and_admission_snapshot() {
    let frame = rich_gray_frame();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let (active, source_live, metadata, frame_bytes) = c1_rich_inventory(&frame);
    assert!(
        frame
            .descriptor()
            .color_information()
            .provenance_capacity_for_test()
            >= frame.descriptor().color_information().provenance().len()
    );
    assert_eq!(
        active,
        frame
            .descriptor()
            .color_information()
            .provenance_capacity_for_test()
            * size_of::<super::super::super::ColorProvenance>()
            + 8
    );
    let live = source_live.checked_add(frame_bytes).unwrap();
    let exact = bounded_limits(frame_bytes, live);
    let mut prepared = PreparedConversion::new(&frame, &options, &exact).unwrap();
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (0, 0, source_live, (frame_bytes, metadata), 0)
    );
    let mut maker = CountingMaker { calls: 0 };
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert!(maker.calls > 0);
    assert_eq!(output.pixels().f32_planes().unwrap().len(), 3);
}

#[test]
fn c1_rich_prepared_conversion_uses_independent_qms_and_preserves_input_owners() {
    use super::super::super::output_plan::tests::{c1_rich_inventory, c1_rich_source};

    let source = c1_rich_source(true);
    let before = source.clone();
    let inventory = c1_rich_inventory(&source);
    let frame_limit = inventory.source_live.max(inventory.requested_output);
    let expected_live = inventory
        .source_live
        .checked_add(inventory.requested_output)
        .unwrap();
    let rich_limits = bounded_limits(frame_limit, expected_live);
    assert!(source.validate_with_limits(&rich_limits).is_ok());

    let sample_pointers: Vec<*const f32> = source
        .pixels()
        .f32_planes()
        .unwrap()
        .iter()
        .map(|plane| plane.samples().as_ptr())
        .collect();
    let source_color = source.metadata().source_color();
    let provenance_pointer = source_color.provenance().as_ptr();
    let icc_pointer = source_color.icc_profile().unwrap().as_ptr();
    let unknown_pointers = [
        source_color.unknown_colr()[0].payload.as_ptr(),
        source_color.unknown_colr()[1].payload.as_ptr(),
    ];
    let coded_pointer = source.metadata().coded_geometry().as_ptr();
    let render_pointer = source.metadata().render_geometry().as_ptr();

    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )));
    let mut prepared = PreparedConversion::new(&source, &options, &rich_limits).unwrap();
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.requested_output, inventory.requested_metadata),
            0,
        )
    );

    let mut maker = CountingMaker { calls: 0 };
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(maker.calls, 27);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            inventory.requested_output,
            inventory.requested_metadata,
            expected_live,
            (0, 0),
            31,
        )
    );
    assert_eq!(output.descriptor().alpha(), AlphaAssociation::Straight);
    assert_eq!(output.pixels().f32_planes().unwrap().len(), 4);
    assert_eq!(
        output.metadata().source_color(),
        source.metadata().source_color()
    );
    assert_eq!(
        output.metadata().coded_geometry(),
        source.metadata().coded_geometry()
    );
    assert_eq!(
        output.metadata().render_geometry(),
        source.metadata().render_geometry()
    );
    assert_eq!(output.metadata().coded_dimensions(), Some((5, 1)));
    assert_eq!(output.metadata().render_dimensions(), Some((5, 1)));
    assert_eq!(output.timing(), source.timing());
    assert_eq!(
        output.pixels().f32_planes().unwrap()[3].samples(),
        source.pixels().f32_planes().unwrap()[3].samples()
    );

    assert_eq!(source, before);
    assert_eq!(
        source
            .pixels()
            .f32_planes()
            .unwrap()
            .iter()
            .map(|plane| plane.samples().as_ptr())
            .collect::<Vec<_>>(),
        sample_pointers
    );
    assert_eq!(source_color.provenance().as_ptr(), provenance_pointer);
    assert_eq!(source_color.icc_profile().unwrap().as_ptr(), icc_pointer);
    assert_eq!(
        [
            source_color.unknown_colr()[0].payload.as_ptr(),
            source_color.unknown_colr()[1].payload.as_ptr(),
        ],
        unknown_pointers
    );
    assert_eq!(source.metadata().coded_geometry().as_ptr(), coded_pointer);
    assert_eq!(source.metadata().render_geometry().as_ptr(), render_pointer);
}

#[test]
fn convert_frame_keeps_alpha_and_source_metadata_separate() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, Some(vec![128]));
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    assert_eq!(converted.descriptor().alpha(), AlphaAssociation::Straight);
    assert_eq!(converted.pixels().f32_planes().unwrap().len(), 4);
    assert!(
        (converted.pixels().f32_planes().unwrap()[3].samples()[0] - 128.0 / 255.0).abs() < 1e-6
    );
    assert_eq!(
        converted.metadata().source_color().nclx(),
        frame.descriptor().color_information().nclx()
    );
    assert!(converted.descriptor().color_information().nclx().is_none());
    let record = converted.metadata().last_conversion().unwrap();
    assert_eq!(
        record.source(),
        super::super::super::metadata::ConversionSource::ActiveCicp
    );
    assert_eq!(
        record.source_cicp(),
        Some(NclxColorInformation::new(1, 13, 0, true))
    );
    assert_eq!(record.source_primaries(), RgbPrimaries::srgb());
}

#[test]
fn convert_frame_second_pass_uses_active_linear_descriptor_without_reapplying_transfer() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let first = convert_frame(&frame, &options, &limits()).unwrap();
    let second = convert_frame(&first, &options, &limits()).unwrap();
    let first_values: Vec<f32> = first
        .pixels()
        .f32_planes()
        .unwrap()
        .iter()
        .map(|plane| plane.samples()[0])
        .collect();
    let second_values: Vec<f32> = second
        .pixels()
        .f32_planes()
        .unwrap()
        .iter()
        .map(|plane| plane.samples()[0])
        .collect();
    assert_eq!(first_values, second_values);
    assert_eq!(
        second.metadata().source_color().nclx(),
        frame.descriptor().color_information().nclx()
    );
    let second_record = second.metadata().last_conversion().unwrap();
    assert_eq!(
        second_record.source(),
        super::super::super::metadata::ConversionSource::LinearRelative
    );
    assert_eq!(second_record.source_cicp(), None);
    assert_eq!(second_record.source_primaries(), RgbPrimaries::srgb());
}

#[test]
fn convert_frame_records_explicit_cicp_as_the_source_authority() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let selected = NclxColorInformation::new(9, 1, 0, true);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(selected));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    assert_eq!(
        converted.metadata().last_conversion().unwrap().source(),
        super::super::super::metadata::ConversionSource::ExplicitCicp
    );
    assert_eq!(
        converted
            .metadata()
            .last_conversion()
            .unwrap()
            .source_cicp(),
        Some(selected)
    );
    assert_eq!(
        converted
            .metadata()
            .last_conversion()
            .unwrap()
            .source_primaries(),
        RgbPrimaries::new(
            (0.708, 0.292),
            (0.170, 0.797),
            (0.131, 0.046),
            (0.3127, 0.3290),
        )
        .unwrap()
    );
}

#[test]
fn convert_frame_records_explicit_cicp_on_declared_linear_source() {
    let frame = rgb_frame_with_domain(vec![64, 128, 192], 1, 1, None, SampleDomain::LinearRelative);
    let selected = NclxColorInformation::new(1, 8, 0, true);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(selected));
    let converted = convert_frame(&frame, &options, &limits()).unwrap();
    let record = converted.metadata().last_conversion().unwrap();
    assert_eq!(
        record.source(),
        super::super::super::metadata::ConversionSource::ExplicitCicp
    );
    assert_eq!(record.source_cicp(), Some(selected));
    assert_eq!(record.source_primaries(), RgbPrimaries::srgb());
}

#[test]
fn convert_frame_rejects_non_linear_destination_before_output() {
    let frame = rgb_frame(vec![1, 2, 3], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::encoded_cicp_rgb(
        NclxColorInformation::new(1, 13, 0, true),
    ));
    let error = convert_frame(&frame, &options, &limits()).unwrap_err();
    assert!(matches!(
        error,
        super::super::super::ProcessingError::Unsupported(_)
    ));
}

#[test]
fn convert_frame_rejects_unknown_destination_primaries_before_output() {
    let frame = rgb_frame(vec![1, 2, 3], 1, 1, None);
    let unknown =
        RgbPrimaries::new((0.64, 0.33), (0.29, 0.60), (0.15, 0.06), (0.3127, 0.3290)).unwrap();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        unknown,
    ));
    let error = convert_frame(&frame, &options, &limits()).unwrap_err();
    assert!(matches!(
        error,
        super::super::super::ProcessingError::Unsupported(_)
    ));
}

struct FailOnceMaker {
    fail_sample_one: bool,
}

struct ObservedFailOnceMaker {
    fail_sample_one: bool,
}

struct CountingMaker {
    calls: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DeniedRequest {
    owner_key: OwnerKey,
    key: u8,
    count: usize,
    ordinal: usize,
}

struct RealAllocationDenyMaker {
    deny_key: OwnerKey,
    next_ordinal: usize,
    denied_calls: usize,
    denied_request: Option<DeniedRequest>,
}

fn owner_key_code(key: OwnerKey) -> u8 {
    match key {
        OwnerKey::Sample(index) => 0x10 + index as u8,
        OwnerKey::DescriptorOuter => 2,
        OwnerKey::PixelOuter => 3,
        OwnerKey::DescriptorLayout { .. } => 4,
        OwnerKey::Role { .. } => 5,
        OwnerKey::PixelLayout { .. } => 6,
        OwnerKey::CodedGeometry => 7,
        OwnerKey::RenderGeometry => 8,
        OwnerKey::PixiBits => 9,
        OwnerKey::PixiExtended => 10,
        OwnerKey::Provenance => 11,
        OwnerKey::IccProfile => 12,
        OwnerKey::IccSourceProfile => 19,
        OwnerKey::IccDestinationProfile => 20,
        OwnerKey::IccDestinationProvenance => 21,
        OwnerKey::UnknownOuter => 13,
        OwnerKey::UnknownPayload(index) => 0x40 + index as u8,
        OwnerKey::SourceNclx => 15,
        OwnerKey::SourceAv1 => 16,
        OwnerKey::CodedDimensions => 17,
        OwnerKey::RenderDimensions => 18,
    }
}

fn owner_key_from_registration(key: u8, ordinal: usize) -> OwnerKey {
    match ordinal {
        0..=3 => OwnerKey::Sample(ordinal),
        4 => OwnerKey::DescriptorOuter,
        5 => OwnerKey::PixelOuter,
        6..=17 => match key {
            4 => OwnerKey::DescriptorLayout {
                plane: (ordinal - 6) / 3,
            },
            5 => OwnerKey::Role {
                plane: (ordinal - 7) / 3,
            },
            6 => OwnerKey::PixelLayout {
                plane: (ordinal - 8) / 3,
            },
            _ => panic!("unexpected structural owner registry key"),
        },
        18 => OwnerKey::CodedGeometry,
        19 => OwnerKey::RenderGeometry,
        20 => OwnerKey::PixiBits,
        21 => OwnerKey::PixiExtended,
        22 => OwnerKey::Provenance,
        23 => OwnerKey::IccProfile,
        24 => OwnerKey::UnknownOuter,
        25 => OwnerKey::UnknownPayload(0),
        _ => panic!("unexpected owner registry ordinal"),
    }
}

impl RealAllocationDenyMaker {
    fn new(deny_key: OwnerKey) -> Self {
        Self {
            deny_key,
            next_ordinal: 0,
            denied_calls: 0,
            denied_request: None,
        }
    }
}

impl CandidateMaker for RealAllocationDenyMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        let result = if key == self.deny_key {
            self.denied_calls += 1;
            self.denied_request = Some(DeniedRequest {
                owner_key: key,
                key: owner_key_code(key),
                count,
                ordinal,
            });
            let _deny = super::super::native::ActualAllocationDenyGuard::arm_once();
            OrdinaryMaker.make(key, count)
        } else {
            OrdinaryMaker.make(key, count)
        };
        if let Ok(ref values) = result {
            let bytes = values
                .capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| ProcessingError::ResourceLimit("owner bytes overflow".into()))?;
            super::super::native::DeallocationObservationGuard::register_owner(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
                key,
                owner_key_code(key),
                count,
                ordinal,
            );
        }
        result
    }
}

impl CandidateMaker for CountingMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        self.calls += 1;
        let mut ordinary = OrdinaryMaker;
        ordinary.make(key, count)
    }
}

impl CandidateMaker for ObservedFailOnceMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        if self.fail_sample_one && key == OwnerKey::Sample(1) {
            self.fail_sample_one = false;
            return Err(ProcessingError::Allocation(
                "test candidate allocation failure".into(),
            ));
        }
        let mut ordinary = OrdinaryMaker;
        let values: Vec<T> = ordinary.make(key, count)?;
        if key == OwnerKey::Sample(0) {
            let bytes = values
                .capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| ProcessingError::ResourceLimit("test target overflow".into()))?;
            super::super::native::DeallocationObservationGuard::register_target(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
            );
        }
        Ok(values)
    }
}

impl CandidateMaker for FailOnceMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        if self.fail_sample_one && key == OwnerKey::Sample(1) {
            self.fail_sample_one = false;
            return Err(ProcessingError::Allocation(
                "test candidate allocation failure".into(),
            ));
        }
        let mut ordinary = OrdinaryMaker;
        ordinary.make(key, count)
    }
}

#[test]
fn prepared_conversion_retries_the_same_inspected_context_with_a_new_maker() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    let mut prepared = PreparedConversion::new(&frame, &options, &limits()).unwrap();
    let mut failing = FailOnceMaker {
        fail_sample_one: true,
    };
    let failed = prepared.materialize_with(&mut failing);
    assert!(matches!(failed, Err(ProcessingError::Allocation(_))));

    let mut ordinary = OrdinaryMaker;
    let converted = prepared.materialize_with(&mut ordinary).unwrap();
    assert_eq!(converted.descriptor().width(), 1);
    assert_eq!(converted.pixels().f32_planes().unwrap().len(), 3);
}

#[test]
fn c1_prepared_exact_keeps_gray_source_and_rejects_before_maker_on_admission() {
    let frame = gray_frame();
    let source_sample = frame.pixels().u8_planes().unwrap()[0].samples()[0];
    let source_ptr = frame.pixels().u8_planes().unwrap()[0].samples().as_ptr();
    let source_metadata = frame.metadata().source_color().clone();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 13, 0, true,
    )));
    let inventory = c1_gray_inventory(&frame);
    assert_eq!(
        inventory.source_live,
        inventory.base + inventory.active_color + inventory.source_metadata
    );
    assert_eq!(inventory.live, inventory.source_live + inventory.frame);
    let exact_limits = bounded_limits(inventory.frame, inventory.live);
    assert!(frame.validate_with_limits(&exact_limits).is_ok());
    let mut prepared = PreparedConversion::new(&frame, &options, &exact_limits).unwrap();
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.frame, inventory.metadata),
            0,
        )
    );
    let mut exact_maker = CountingMaker { calls: 0 };
    assert!(prepared.materialize_with(&mut exact_maker).is_ok());
    assert!(exact_maker.calls > 0);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            inventory.frame,
            inventory.metadata,
            inventory.live,
            (0, 0),
            19
        )
    );
    assert_eq!(
        frame.pixels().u8_planes().unwrap()[0].samples()[0],
        source_sample
    );
    assert_eq!(
        frame.pixels().u8_planes().unwrap()[0].samples().as_ptr(),
        source_ptr
    );
    assert_eq!(frame.metadata().source_color(), &source_metadata);

    for (frame_limit, live_limit) in [
        (inventory.frame - 1, inventory.live),
        (inventory.frame, inventory.live - 1),
    ] {
        let constrained = bounded_limits(frame_limit, live_limit);
        assert!(frame.validate_with_limits(&constrained).is_ok());
        let mut maker = CountingMaker { calls: 0 };
        let result = PreparedConversion::new(&frame, &options, &constrained)
            .and_then(|mut p| p.materialize_with(&mut maker));
        assert!(result.is_err());
        assert_eq!(maker.calls, 0);
    }
}

struct C2PreparedMaker;

impl CandidateMaker for C2PreparedMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let mut values = OrdinaryMaker.make(key, count)?;
        let target = match key {
            OwnerKey::Sample(0) if count == 5 => 8,
            OwnerKey::Provenance if count == 3 => 5,
            OwnerKey::IccProfile if count == 4093 => 4160,
            OwnerKey::UnknownPayload(1) if count == 13 => 4160,
            _ => count,
        };
        if target > values.capacity() {
            values.try_reserve_exact(target).map_err(|error| {
                ProcessingError::Allocation(format!("C2 prepared growth failed: {error}"))
            })?;
        }
        if values.capacity() != target {
            return Err(ProcessingError::ResourceLimit(
                "C2 prepared candidate capacity is not deterministic".into(),
            ));
        }
        Ok(values)
    }
}

/// C2e-1's only surplus owner: the first f32 sample plane requests five
/// elements (20 bytes), while the controlled fallible allocation returns an
/// empty candidate with capacity eight (32 bytes).  All other owners use the
/// ordinary exact-count path so this test cannot accidentally duplicate the
/// later C2e slices.
struct C2e1SampleMaker {
    surplus: bool,
    requests: [Option<(OwnerKey, usize)>; 32],
    request_count: usize,
    sample_length: usize,
    sample_capacity: usize,
}

impl C2e1SampleMaker {
    fn new(surplus: bool) -> Self {
        Self {
            surplus,
            requests: [None; 32],
            request_count: 0,
            sample_length: 0,
            sample_capacity: 0,
        }
    }

    fn record_request(&mut self, key: OwnerKey, count: usize) {
        assert!(self.request_count < self.requests.len());
        self.requests[self.request_count] = Some((key, count));
        self.request_count += 1;
    }
}

impl CandidateMaker for C2e1SampleMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        self.record_request(key, count);
        let mut values = OrdinaryMaker.make(key, count)?;
        if self.surplus && key == OwnerKey::Sample(0) && count == 5 {
            values.try_reserve_exact(8).map_err(|error| {
                ProcessingError::Allocation(format!("C2e-1 surplus allocation failed: {error}"))
            })?;
            self.sample_length = values.len();
            self.sample_capacity = values.capacity();
            assert_eq!(self.sample_length, 0);
            assert_eq!(self.sample_capacity, 8);
            let bytes = self
                .sample_capacity
                .checked_mul(size_of::<T>())
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("C2e-1 sample bytes overflow".into())
                })?;
            super::super::native::DeallocationObservationGuard::register_target(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
            );
        }
        Ok(values)
    }
}

fn c2e1_limits(frame: usize, live: usize, max_plane: usize) -> ResourceLimits {
    let mut result = limits();
    result.max_frame_bytes = frame;
    result.max_total_live_decoded_bytes = live;
    result.max_plane_bytes = max_plane;
    result
}

#[test]
fn c2e1_sample0_actual_capacity_transaction_is_exact_and_retryable() {
    use super::super::super::output_plan::tests::{
        c2_assert_source_unchanged, c2_packed_rich_source, c2_rich_inventory, c2_source_snapshot,
    };

    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let surplus = 12usize;
    let exact_frame = inventory.requested_output.checked_add(surplus).unwrap();
    let exact_live = inventory.source_live.checked_add(exact_frame).unwrap();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )));

    // The exact control exercises the complete owner walk and checks the
    // independent actual-capacity accounting before the published frame is
    // dropped.
    let source_snapshot = c2_source_snapshot(&source);
    let exact_limits = c2e1_limits(exact_frame, exact_live, 32);
    assert!(source.validate_with_limits(&exact_limits).is_ok());
    let reference = {
        let mut prepared = PreparedConversion::new(&source, &options, &limits()).unwrap();
        let mut maker = OrdinaryMaker;
        prepared.materialize_with(&mut maker).unwrap()
    };
    let mut prepared = PreparedConversion::new(&source, &options, &exact_limits).unwrap();
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.requested_output, inventory.requested_metadata),
            0,
        )
    );
    let boundary = RestoreObserverGuard::begin();
    let mut maker = C2e1SampleMaker::new(true);
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(maker.request_count, 27);
    assert_eq!(maker.requests[0], Some((OwnerKey::Sample(0), 5)));
    assert_eq!(maker.sample_length, 0);
    assert_eq!(maker.sample_capacity, 8);
    assert_eq!(
        c2_owned_output_bytes(&output),
        inventory.requested_output.checked_add(surplus).unwrap()
    );
    assert_c2b_output_matches(&reference, &output);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            exact_frame,
            inventory.requested_metadata,
            exact_live,
            (0, 0),
            31,
        )
    );
    assert!(!super::super::native::DeallocationObservationGuard::target_deallocated());
    #[cfg(miri)]
    super::super::native::drop_registered_value(output);
    #[cfg(not(miri))]
    drop(output);
    assert!(super::super::native::DeallocationObservationGuard::target_deallocated());
    assert_eq!(
        super::super::native::DeallocationObservationGuard::target_deallocations(),
        1
    );
    drop(boundary);
    c2_assert_source_unchanged(&source, &source_snapshot);

    // Each refusal gets an independent Prepared object, but the same source
    // and maker prove that admission itself succeeds and the first actual
    // candidate is the only request before the typed limit error.
    for (frame_limit, live_limit, max_plane) in [
        (exact_frame - 1, exact_live, 32),
        (exact_frame, exact_live - 1, 32),
        (exact_frame, exact_live, 31),
    ] {
        let source_snapshot = c2_source_snapshot(&source);
        let row_limits = c2e1_limits(frame_limit, live_limit, max_plane);
        assert!(source.validate_with_limits(&row_limits).is_ok());
        let mut prepared = PreparedConversion::new(&source, &options, &row_limits).unwrap();
        let before = prepared.admitted.c1_accounting_snapshot();
        assert_eq!(
            before,
            (
                0,
                0,
                inventory.source_live,
                (inventory.requested_output, inventory.requested_metadata),
                0,
            )
        );
        RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
        RESTORE_TARGET_SNAPSHOT.with(|snapshot| snapshot.set(None));
        RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| snapshot.set(None));
        RESTORE_OWNER_STATUS.with(|status| status.set(None));
        let boundary = RestoreObserverGuard::begin();
        let mut failing = C2e1SampleMaker::new(true);
        let result = prepared.materialize_with(&mut failing);
        assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
        assert_eq!(failing.request_count, 1);
        assert_eq!(failing.requests[0], Some((OwnerKey::Sample(0), 5)));
        assert_eq!(failing.sample_length, 0);
        assert_eq!(failing.sample_capacity, 8);
        assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
        assert_eq!(RESTORE_TARGET_SNAPSHOT.with(Cell::get), Some(true));
        assert_eq!(RESTORE_TARGET_DEALLOCATIONS.with(Cell::get), Some(1));
        assert_eq!(RESTORE_OWNER_STATUS.with(Cell::get), Some((0, 0, true)));
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        c2_assert_source_unchanged(&source, &source_snapshot);
        drop(boundary);

        // The restored Prepared object must follow the ordinary exact-count
        // path and publish only once.
        let mut ordinary = C2e1SampleMaker::new(false);
        let actual = prepared.materialize_with(&mut ordinary).unwrap();
        assert_eq!(ordinary.requests[0], Some((OwnerKey::Sample(0), 5)));
        assert_c2b_output_matches(&reference, &actual);
        assert_eq!(
            prepared.admitted.c1_accounting_snapshot(),
            (
                inventory.requested_output,
                inventory.requested_metadata,
                inventory.source_live + inventory.requested_output,
                (0, 0),
                31,
            )
        );
        assert!(matches!(
            prepared.materialize_with(&mut ordinary),
            Err(ProcessingError::Unsupported(_))
        ));
        drop(actual);
        c2_assert_source_unchanged(&source, &source_snapshot);
    }
}

/// C2e-2 isolates the ICC metadata owner.  The source profile requests 4093
/// bytes, while the controlled candidate has an actual capacity of 4160
/// bytes.  Every other owner follows the ordinary exact-count route so the
/// four independent resource-limit refusals below can only be caused by the
/// ICC capacity surplus.
struct C2e2IccMaker {
    surplus: bool,
    requests: [Option<(OwnerKey, usize)>; 32],
    request_count: usize,
    next_ordinal: usize,
    icc_length: usize,
    icc_capacity: usize,
}

impl C2e2IccMaker {
    fn new(surplus: bool) -> Self {
        Self {
            surplus,
            requests: [None; 32],
            request_count: 0,
            next_ordinal: 0,
            icc_length: 0,
            icc_capacity: 0,
        }
    }

    fn record_request(&mut self, key: OwnerKey, count: usize) {
        assert!(self.request_count < self.requests.len());
        self.requests[self.request_count] = Some((key, count));
        self.request_count += 1;
    }
}

impl CandidateMaker for C2e2IccMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        self.record_request(key, count);
        let mut values = OrdinaryMaker.make(key, count)?;
        if self.surplus && key == OwnerKey::IccProfile && count == 4093 {
            values.try_reserve_exact(4160).map_err(|error| {
                ProcessingError::Allocation(format!("C2e-2 surplus allocation failed: {error}"))
            })?;
            self.icc_length = values.len();
            self.icc_capacity = values.capacity();
            assert_eq!(self.icc_length, 0);
            assert_eq!(self.icc_capacity, 4160);
            let bytes = self
                .icc_capacity
                .checked_mul(size_of::<T>())
                .ok_or_else(|| ProcessingError::ResourceLimit("C2e-2 ICC bytes overflow".into()))?;
            super::super::native::DeallocationObservationGuard::register_target(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
            );
        } else {
            let bytes = values
                .capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("C2e-2 owner bytes overflow".into())
                })?;
            super::super::native::DeallocationObservationGuard::register_owner(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
                key,
                owner_key_code(key),
                count,
                ordinal,
            );
        }
        Ok(values)
    }
}

fn c2e2_limits(frame: usize, live: usize, metadata: usize, icc: usize) -> ResourceLimits {
    let mut result = c2e1_limits(frame, live, 32);
    result.max_metadata_bytes = metadata;
    result.max_icc_bytes = icc;
    result
}

#[test]
fn c2e2_icc_actual_capacity_transaction_is_exact_and_retryable() {
    use super::super::super::output_plan::tests::{
        c2_assert_source_unchanged, c2_packed_rich_source, c2_rich_inventory, c2_source_snapshot,
    };

    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let surplus = 4160usize.checked_sub(4093).unwrap();
    let exact_frame = inventory.requested_output.checked_add(surplus).unwrap();
    let exact_live = inventory.source_live.checked_add(exact_frame).unwrap();
    let exact_metadata = inventory.requested_metadata.checked_add(surplus).unwrap();
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )));

    let source_snapshot = c2_source_snapshot(&source);
    let exact_limits = c2e2_limits(exact_frame, exact_live, exact_metadata, 4160);
    assert!(source.validate_with_limits(&exact_limits).is_ok());
    let reference = {
        let mut prepared = PreparedConversion::new(&source, &options, &limits()).unwrap();
        let mut maker = OrdinaryMaker;
        prepared.materialize_with(&mut maker).unwrap()
    };
    let mut prepared = PreparedConversion::new(&source, &options, &exact_limits).unwrap();
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.requested_output, inventory.requested_metadata),
            0,
        )
    );
    let boundary = RestoreObserverGuard::begin();
    let mut maker = C2e2IccMaker::new(true);
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(maker.request_count, 27);
    assert_eq!(maker.requests[23], Some((OwnerKey::IccProfile, 4093)));
    assert_eq!(maker.icc_length, 0);
    assert_eq!(maker.icc_capacity, 4160);
    assert_eq!(
        c2_owned_output_bytes(&output),
        inventory.requested_output.checked_add(surplus).unwrap()
    );
    assert_eq!(
        output.metadata().source_color().icc_profile_capacity(),
        4160
    );
    assert_c2b_output_matches(&reference, &output);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (exact_frame, exact_metadata, exact_live, (0, 0), 31)
    );
    assert!(!super::super::native::DeallocationObservationGuard::target_deallocated());
    #[cfg(miri)]
    super::super::native::drop_registered_value(output);
    #[cfg(not(miri))]
    drop(output);
    assert!(super::super::native::DeallocationObservationGuard::target_deallocated());
    assert_eq!(
        super::super::native::DeallocationObservationGuard::target_deallocations(),
        1
    );
    let (owners, owner_count) =
        super::super::native::DeallocationObservationGuard::registered_snapshot();
    assert_eq!(owner_count, 26);
    for owner in owners[..owner_count].iter() {
        let Some((key, count)) = maker.requests[owner.ordinal] else {
            panic!("C2e-2 registered an owner outside the recorded walk");
        };
        assert_eq!(
            (owner.owner_key, owner.key, owner.count),
            (key, owner_key_code(key), count)
        );
        assert_ne!(owner.ordinal, 23);
    }
    #[cfg(miri)]
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (26, 0, false)
    );
    #[cfg(not(miri))]
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (26, 26, true)
    );
    drop(boundary);
    c2_assert_source_unchanged(&source, &source_snapshot);

    // The requested 4093-byte ICC profile fits every one-under row.  Each
    // failure therefore proves that the post-allocation 4160-byte capacity is
    // checked transactionally against exactly one independent limit.
    for (frame_limit, live_limit, metadata_limit, icc_limit) in [
        (exact_frame - 1, exact_live, exact_metadata, 4160),
        (exact_frame, exact_live - 1, exact_metadata, 4160),
        (exact_frame, exact_live, exact_metadata - 1, 4160),
        (exact_frame, exact_live, exact_metadata, 4159),
    ] {
        let source_snapshot = c2_source_snapshot(&source);
        let row_limits = c2e2_limits(frame_limit, live_limit, metadata_limit, icc_limit);
        assert!(source.validate_with_limits(&row_limits).is_ok());
        let mut prepared = PreparedConversion::new(&source, &options, &row_limits).unwrap();
        let before = prepared.admitted.c1_accounting_snapshot();
        assert_eq!(
            before,
            (
                0,
                0,
                inventory.source_live,
                (inventory.requested_output, inventory.requested_metadata),
                0,
            )
        );
        RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
        RESTORE_TARGET_SNAPSHOT.with(|snapshot| snapshot.set(None));
        RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| snapshot.set(None));
        RESTORE_OWNER_STATUS.with(|status| status.set(None));
        let boundary = RestoreObserverGuard::begin();
        let mut failing = C2e2IccMaker::new(true);
        let result = prepared.materialize_with(&mut failing);
        assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
        assert_eq!(failing.request_count, 24);
        assert_eq!(failing.requests[23], Some((OwnerKey::IccProfile, 4093)));
        assert_eq!(failing.icc_length, 0);
        assert_eq!(failing.icc_capacity, 4160);
        assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
        assert_eq!(RESTORE_TARGET_SNAPSHOT.with(Cell::get), Some(true));
        assert_eq!(RESTORE_TARGET_DEALLOCATIONS.with(Cell::get), Some(1));
        let (owners, owner_count) =
            super::super::native::DeallocationObservationGuard::registered_snapshot();
        assert_eq!(owner_count, 23);
        #[cfg(miri)]
        assert_eq!(
            super::super::native::DeallocationObservationGuard::registered_status(),
            (23, 0, false)
        );
        #[cfg(not(miri))]
        assert_eq!(RESTORE_OWNER_STATUS.with(Cell::get), Some((23, 23, true)));
        for (owner, &(expected_key, expected_count, expected_ordinal)) in owners[..owner_count]
            .iter()
            .zip(C2C4_OWNER_PREFIX[..23].iter())
        {
            #[cfg(miri)]
            assert_eq!(
                (owner.owner_key, owner.key, owner.count, owner.ordinal),
                (
                    owner_key_from_registration(expected_key, expected_ordinal),
                    expected_key,
                    expected_count,
                    expected_ordinal,
                )
            );
            #[cfg(not(miri))]
            assert_eq!(
                (
                    owner.owner_key,
                    owner.key,
                    owner.count,
                    owner.ordinal,
                    owner.deallocations
                ),
                (
                    owner_key_from_registration(expected_key, expected_ordinal),
                    expected_key,
                    expected_count,
                    expected_ordinal,
                    1,
                )
            );
        }
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        c2_assert_source_unchanged(&source, &source_snapshot);
        drop(boundary);

        let mut ordinary = C2e2IccMaker::new(false);
        let actual = prepared.materialize_with(&mut ordinary).unwrap();
        assert_eq!(ordinary.requests[23], Some((OwnerKey::IccProfile, 4093)));
        assert_c2b_output_matches(&reference, &actual);
        assert_eq!(
            prepared.admitted.c1_accounting_snapshot(),
            (
                inventory.requested_output,
                inventory.requested_metadata,
                inventory.source_live + inventory.requested_output,
                (0, 0),
                31,
            )
        );
        assert!(matches!(
            prepared.materialize_with(&mut ordinary),
            Err(ProcessingError::Unsupported(_))
        ));
        drop(actual);
        c2_assert_source_unchanged(&source, &source_snapshot);
    }
}

/// C2e-3 exercises the remaining metadata-backed fresh candidates whose
/// actual vector capacity can exceed their logical source length.  Keeping
/// this maker separate from C2e-1/e-2 makes each target's transaction and
/// deallocation boundary observable without widening a production API.
#[derive(Clone, Copy)]
struct C2e3Case {
    target: OwnerKey,
    count: usize,
    ordinal: usize,
    capacity: usize,
    prefix: &'static [(u8, usize, usize)],
}

struct C2e3MetadataMaker {
    case: C2e3Case,
    surplus: bool,
    requests: [Option<(OwnerKey, usize)>; 32],
    request_count: usize,
    next_ordinal: usize,
    target_length: usize,
    target_capacity: usize,
}

impl C2e3MetadataMaker {
    fn new(case: C2e3Case, surplus: bool) -> Self {
        Self {
            case,
            surplus,
            requests: [None; 32],
            request_count: 0,
            next_ordinal: 0,
            target_length: 0,
            target_capacity: 0,
        }
    }

    fn record_request(&mut self, key: OwnerKey, count: usize) {
        assert!(self.request_count < self.requests.len());
        self.requests[self.request_count] = Some((key, count));
        self.request_count += 1;
    }
}

impl CandidateMaker for C2e3MetadataMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        self.record_request(key, count);
        let mut values = OrdinaryMaker.make(key, count)?;
        if self.surplus && key == self.case.target && count == self.case.count {
            values
                .try_reserve_exact(self.case.capacity)
                .map_err(|error| {
                    ProcessingError::Allocation(format!("C2e-3 surplus allocation failed: {error}"))
                })?;
            self.target_length = values.len();
            self.target_capacity = values.capacity();
            assert_eq!(self.target_length, 0);
            assert_eq!(self.target_capacity, self.case.capacity);
            let bytes = self
                .target_capacity
                .checked_mul(size_of::<T>())
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("C2e-3 target bytes overflow".into())
                })?;
            super::super::native::DeallocationObservationGuard::register_target(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
            );
        } else {
            let bytes = values
                .capacity()
                .checked_mul(size_of::<T>())
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("C2e-3 owner bytes overflow".into())
                })?;
            super::super::native::DeallocationObservationGuard::register_owner(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
                key,
                owner_key_code(key),
                count,
                ordinal,
            );
        }
        Ok(values)
    }
}

fn c2e3_target_element_size(case: C2e3Case) -> usize {
    match case.target {
        OwnerKey::Provenance => size_of::<ColorProvenance>(),
        OwnerKey::UnknownPayload(_) => size_of::<u8>(),
        OwnerKey::CodedGeometry => size_of::<GeometryOperation>(),
        _ => panic!("C2e-3 case is not a metadata vector owner"),
    }
}

fn c2e3_options() -> ColorConvertOptions<'static> {
    ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )))
}

fn c2e3_limits(frame: usize, live: usize, metadata: usize) -> ResourceLimits {
    c2e2_limits(frame, live, metadata, 4093)
}

fn c2e3_assert_target_capacity(output: &ImageFrame, case: C2e3Case) {
    match case.target {
        OwnerKey::Provenance => assert_eq!(
            output
                .metadata()
                .source_color()
                .provenance_capacity_for_test(),
            case.capacity
        ),
        OwnerKey::UnknownPayload(index) => assert_eq!(
            output.metadata().source_color().unknown_colr()[index]
                .payload
                .capacity(),
            case.capacity
        ),
        OwnerKey::CodedGeometry => assert_eq!(
            output.metadata().coded_geometry_capacity_for_test(),
            case.capacity
        ),
        _ => unreachable!("C2e-3 target kind was checked before running"),
    }
}

/// Runs the exact capacity admission and each independent one-under limit
/// against a single metadata owner.  Every row observes the real candidate's
/// drop before ledger restoration, verifies source immutability, then retries
/// the same prepared conversion and rejects a second publication.
fn c2e3_actual_capacity_transaction_is_exact_and_retryable(source: ImageFrame, case: C2e3Case) {
    use super::super::super::output_plan::tests::{
        c2_assert_source_unchanged, c2_rich_inventory, c2_source_snapshot,
    };

    let inventory = c2_rich_inventory(&source, 5);
    let element_size = c2e3_target_element_size(case);
    let requested = case.count.checked_mul(element_size).unwrap();
    let actual = case.capacity.checked_mul(element_size).unwrap();
    let surplus = actual.checked_sub(requested).unwrap();
    let exact_frame = inventory.requested_output.checked_add(surplus).unwrap();
    let exact_live = inventory.source_live.checked_add(exact_frame).unwrap();
    let exact_metadata = inventory.requested_metadata.checked_add(surplus).unwrap();
    let options = c2e3_options();

    let source_snapshot = c2_source_snapshot(&source);
    let exact_limits = c2e3_limits(exact_frame, exact_live, exact_metadata);
    assert!(source.validate_with_limits(&exact_limits).is_ok());
    let reference = {
        let mut prepared = PreparedConversion::new(&source, &options, &limits()).unwrap();
        let mut maker = OrdinaryMaker;
        prepared.materialize_with(&mut maker).unwrap()
    };
    let mut prepared = PreparedConversion::new(&source, &options, &exact_limits).unwrap();
    let before = (
        0,
        0,
        inventory.source_live,
        (inventory.requested_output, inventory.requested_metadata),
        0,
    );
    assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
    let boundary = RestoreObserverGuard::begin();
    let mut maker = C2e3MetadataMaker::new(case, true);
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(maker.request_count, 27);
    assert_eq!(
        maker.requests[case.ordinal],
        Some((case.target, case.count))
    );
    assert_eq!(maker.target_length, 0);
    assert_eq!(maker.target_capacity, case.capacity);
    assert_eq!(c2_owned_output_bytes(&output), exact_frame);
    c2e3_assert_target_capacity(&output, case);
    assert_c2b_output_matches(&reference, &output);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (exact_frame, exact_metadata, exact_live, (0, 0), 31)
    );
    assert!(!super::super::native::DeallocationObservationGuard::target_deallocated());
    #[cfg(miri)]
    super::super::native::drop_registered_value(output);
    #[cfg(not(miri))]
    drop(output);
    assert!(super::super::native::DeallocationObservationGuard::target_deallocated());
    assert_eq!(
        super::super::native::DeallocationObservationGuard::target_deallocations(),
        1
    );
    // Host execution proves physical pointer/size/alignment deallocation for
    // every prefix owner. Miri retains the independently registered prefix
    // shape for the failing transaction below and proves only the target's
    // logical post-drop event.
    #[cfg(not(miri))]
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (26, 26, true)
    );
    drop(boundary);
    c2_assert_source_unchanged(&source, &source_snapshot);

    for (frame_limit, live_limit, metadata_limit) in [
        (exact_frame - 1, exact_live, exact_metadata),
        (exact_frame, exact_live - 1, exact_metadata),
        (exact_frame, exact_live, exact_metadata - 1),
    ] {
        let source_snapshot = c2_source_snapshot(&source);
        let row_limits = c2e3_limits(frame_limit, live_limit, metadata_limit);
        assert!(source.validate_with_limits(&row_limits).is_ok());
        let mut prepared = PreparedConversion::new(&source, &options, &row_limits).unwrap();
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
        RESTORE_TARGET_SNAPSHOT.with(|snapshot| snapshot.set(None));
        RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| snapshot.set(None));
        RESTORE_OWNER_STATUS.with(|status| status.set(None));
        let boundary = RestoreObserverGuard::begin();
        let mut failing = C2e3MetadataMaker::new(case, true);
        let result = prepared.materialize_with(&mut failing);
        assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
        assert_eq!(failing.request_count, case.ordinal + 1);
        assert_eq!(
            failing.requests[case.ordinal],
            Some((case.target, case.count))
        );
        assert_eq!(failing.target_length, 0);
        assert_eq!(failing.target_capacity, case.capacity);
        assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
        assert_eq!(RESTORE_TARGET_SNAPSHOT.with(Cell::get), Some(true));
        assert_eq!(RESTORE_TARGET_DEALLOCATIONS.with(Cell::get), Some(1));
        let (owners, owner_count) =
            super::super::native::DeallocationObservationGuard::registered_snapshot();
        assert_eq!(owner_count, case.prefix.len());
        // Keep the complete construction prefix observable on every target:
        // logical Miri evidence includes key/count/ordinal registration, while
        // the host adds physical exact-once deallocation evidence below.
        #[cfg(miri)]
        for (owner, &(expected_key, expected_count, expected_ordinal)) in
            owners[..owner_count].iter().zip(case.prefix)
        {
            assert_eq!(
                (owner.owner_key, owner.key, owner.count, owner.ordinal),
                (
                    owner_key_from_registration(expected_key, expected_ordinal),
                    expected_key,
                    expected_count,
                    expected_ordinal,
                )
            );
        }
        #[cfg(not(miri))]
        assert_eq!(
            RESTORE_OWNER_STATUS.with(Cell::get),
            Some((case.prefix.len(), case.prefix.len(), true))
        );
        #[cfg(not(miri))]
        for (owner, &(expected_key, expected_count, expected_ordinal)) in
            owners[..owner_count].iter().zip(case.prefix)
        {
            assert_eq!(
                (
                    owner.owner_key,
                    owner.key,
                    owner.count,
                    owner.ordinal,
                    owner.deallocations,
                ),
                (
                    owner_key_from_registration(expected_key, expected_ordinal),
                    expected_key,
                    expected_count,
                    expected_ordinal,
                    1,
                )
            );
        }
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        c2_assert_source_unchanged(&source, &source_snapshot);
        drop(boundary);

        let mut ordinary = C2e3MetadataMaker::new(case, false);
        let output = prepared.materialize_with(&mut ordinary).unwrap();
        assert_eq!(
            ordinary.requests[case.ordinal],
            Some((case.target, case.count))
        );
        assert_c2b_output_matches(&reference, &output);
        assert_eq!(
            prepared.admitted.c1_accounting_snapshot(),
            (
                inventory.requested_output,
                inventory.requested_metadata,
                inventory.source_live + inventory.requested_output,
                (0, 0),
                31,
            )
        );
        assert!(matches!(
            prepared.materialize_with(&mut ordinary),
            Err(ProcessingError::Unsupported(_))
        ));
        drop(output);
        c2_assert_source_unchanged(&source, &source_snapshot);
    }
}

const C2E3_UNKNOWN_ONE_PREFIX: [(u8, usize, usize); 26] = [
    (0x10, 5, 0),
    (0x11, 5, 1),
    (0x12, 5, 2),
    (0x13, 5, 3),
    (2, 4, 4),
    (3, 4, 5),
    (4, 1, 6),
    (5, 1, 7),
    (6, 1, 8),
    (4, 1, 9),
    (5, 1, 10),
    (6, 1, 11),
    (4, 1, 12),
    (5, 1, 13),
    (6, 1, 14),
    (4, 1, 15),
    (5, 1, 16),
    (6, 1, 17),
    (7, 2, 18),
    (8, 1, 19),
    (9, 4, 20),
    (10, 4, 21),
    (11, 3, 22),
    (12, 4093, 23),
    (13, 2, 24),
    (0x40, 13, 25),
];

#[test]
fn c2e3_provenance_actual_capacity_transaction_is_exact_and_retryable() {
    c2e3_actual_capacity_transaction_is_exact_and_retryable(
        super::super::super::output_plan::tests::c2_packed_rich_source(),
        C2e3Case {
            target: OwnerKey::Provenance,
            count: 3,
            ordinal: 22,
            capacity: 5,
            prefix: &C2C4_OWNER_PREFIX[..22],
        },
    );
}

#[test]
fn c2e3_unknown_payload_one_actual_capacity_transaction_is_exact_and_retryable() {
    c2e3_actual_capacity_transaction_is_exact_and_retryable(
        super::super::super::output_plan::tests::c2_packed_rich_source(),
        C2e3Case {
            target: OwnerKey::UnknownPayload(1),
            count: 13,
            ordinal: 26,
            capacity: 4160,
            prefix: &C2E3_UNKNOWN_ONE_PREFIX,
        },
    );
}

#[test]
fn c2e3_zero_count_coded_geometry_actual_capacity_transaction_is_exact_and_retryable() {
    let source = super::super::super::output_plan::tests::c2_packed_rich_source();
    let mut metadata = source.metadata().clone();
    metadata.set_coded_geometry(Vec::new());
    c2e3_actual_capacity_transaction_is_exact_and_retryable(
        source.with_metadata(metadata),
        C2e3Case {
            target: OwnerKey::CodedGeometry,
            count: 0,
            ordinal: 18,
            capacity: 1,
            prefix: &C2C3_OWNER_PREFIX[..18],
        },
    );
}

/// C2e-4 aggregates the four non-exact capacities that occur in the packed
/// rich conversion.  The final unknown payload remains the target so every
/// admission and every preceding candidate has already succeeded before the
/// transactional capacity refusal is observed.
struct C2e4AggregateMaker {
    surplus: bool,
    requests: [Option<(OwnerKey, usize)>; 32],
    request_count: usize,
    next_ordinal: usize,
    sample_capacity: usize,
    provenance_capacity: usize,
    icc_capacity: usize,
    unknown_payload_capacity: usize,
}

impl C2e4AggregateMaker {
    fn new(surplus: bool) -> Self {
        Self {
            surplus,
            requests: [None; 32],
            request_count: 0,
            next_ordinal: 0,
            sample_capacity: 0,
            provenance_capacity: 0,
            icc_capacity: 0,
            unknown_payload_capacity: 0,
        }
    }

    fn target_capacity(key: OwnerKey, count: usize) -> Option<usize> {
        match (key, count) {
            (OwnerKey::Sample(0), 5) => Some(8),
            (OwnerKey::Provenance, 3) => Some(5),
            (OwnerKey::IccProfile, 4093) => Some(4160),
            (OwnerKey::UnknownPayload(1), 13) => Some(4160),
            _ => None,
        }
    }

    fn record_request(&mut self, key: OwnerKey, count: usize) {
        assert!(self.request_count < self.requests.len());
        self.requests[self.request_count] = Some((key, count));
        self.request_count += 1;
    }

    fn record_capacity(&mut self, key: OwnerKey, capacity: usize) {
        match key {
            OwnerKey::Sample(0) => self.sample_capacity = capacity,
            OwnerKey::Provenance => self.provenance_capacity = capacity,
            OwnerKey::IccProfile => self.icc_capacity = capacity,
            OwnerKey::UnknownPayload(1) => self.unknown_payload_capacity = capacity,
            _ => unreachable!("C2e-4 only records its four surplus owners"),
        }
    }
}

impl CandidateMaker for C2e4AggregateMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        self.record_request(key, count);

        let mut values = OrdinaryMaker.make(key, count)?;
        let surplus_capacity = self
            .surplus
            .then(|| Self::target_capacity(key, count))
            .flatten();
        if let Some(capacity) = surplus_capacity {
            values.try_reserve_exact(capacity).map_err(|error| {
                ProcessingError::Allocation(format!("C2e-4 surplus allocation failed: {error}"))
            })?;
            assert_eq!(values.len(), 0);
            assert_eq!(values.capacity(), capacity);
            self.record_capacity(key, capacity);
        }

        let bytes = values
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or_else(|| ProcessingError::ResourceLimit("C2e-4 owner bytes overflow".into()))?;
        if key == OwnerKey::UnknownPayload(1) && count == 13 && surplus_capacity.is_some() {
            super::super::native::DeallocationObservationGuard::register_target(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
            );
        } else {
            super::super::native::DeallocationObservationGuard::register_owner(
                values.as_ptr() as *const u8,
                bytes,
                std::mem::align_of::<T>(),
                key,
                owner_key_code(key),
                count,
                ordinal,
            );
        }
        Ok(values)
    }
}

fn c2e4_limits(frame: usize, live: usize, metadata: usize) -> ResourceLimits {
    c2e2_limits(frame, live, metadata, 4160)
}

fn c2e4_surplus_bytes() -> usize {
    [
        8usize.checked_mul(size_of::<f32>()).unwrap()
            - 5usize.checked_mul(size_of::<f32>()).unwrap(),
        5usize.checked_mul(size_of::<ColorProvenance>()).unwrap()
            - 3usize.checked_mul(size_of::<ColorProvenance>()).unwrap(),
        4160usize - 4093,
        4160usize - 13,
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap()
}

fn c2e4_metadata_surplus_bytes() -> usize {
    [
        5usize.checked_mul(size_of::<ColorProvenance>()).unwrap()
            - 3usize.checked_mul(size_of::<ColorProvenance>()).unwrap(),
        4160usize - 4093,
        4160usize - 13,
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap()
}

fn c2e4_assert_capacities(output: &ImageFrame) {
    assert_eq!(
        output.pixels().f32_planes().unwrap()[0].sample_capacity(),
        8
    );
    assert_eq!(
        output
            .metadata()
            .source_color()
            .provenance_capacity_for_test(),
        5
    );
    assert_eq!(
        output.metadata().source_color().icc_profile_capacity(),
        4160
    );
    assert_eq!(
        output.metadata().source_color().unknown_colr()[1]
            .payload
            .capacity(),
        4160
    );
}

fn c2e4_assert_prefix_shape() {
    let (owners, owner_count) =
        super::super::native::DeallocationObservationGuard::registered_snapshot();
    assert_eq!(owner_count, 26);
    for (owner, &(expected_key, expected_count, expected_ordinal)) in owners[..owner_count]
        .iter()
        .zip(C2E3_UNKNOWN_ONE_PREFIX.iter())
    {
        assert_eq!(
            (owner.owner_key, owner.key, owner.count, owner.ordinal),
            (
                owner_key_from_registration(expected_key, expected_ordinal),
                expected_key,
                expected_count,
                expected_ordinal,
            )
        );
    }
}

#[test]
fn c2e4_aggregate_actual_capacity_transaction_is_exact_and_retryable() {
    use super::super::super::output_plan::tests::{
        c2_assert_source_unchanged, c2_packed_rich_source, c2_rich_inventory, c2_source_snapshot,
    };

    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let surplus = c2e4_surplus_bytes();
    let exact_frame = inventory.requested_output.checked_add(surplus).unwrap();
    let exact_live = inventory.source_live.checked_add(exact_frame).unwrap();
    let exact_metadata = inventory
        .requested_metadata
        .checked_add(c2e4_metadata_surplus_bytes())
        .unwrap();
    let options = c2e3_options();
    let before = (
        0,
        0,
        inventory.source_live,
        (inventory.requested_output, inventory.requested_metadata),
        0,
    );
    let reference = {
        let mut prepared = PreparedConversion::new(&source, &options, &limits()).unwrap();
        let mut maker = OrdinaryMaker;
        prepared.materialize_with(&mut maker).unwrap()
    };

    let source_snapshot = c2_source_snapshot(&source);
    let exact_limits = c2e4_limits(exact_frame, exact_live, exact_metadata);
    assert!(source.validate_with_limits(&exact_limits).is_ok());
    let mut prepared = PreparedConversion::new(&source, &options, &exact_limits).unwrap();
    assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
    let boundary = RestoreObserverGuard::begin();
    let mut maker = C2e4AggregateMaker::new(true);
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(maker.request_count, 27);
    assert_eq!(maker.requests[0], Some((OwnerKey::Sample(0), 5)));
    assert_eq!(maker.requests[22], Some((OwnerKey::Provenance, 3)));
    assert_eq!(maker.requests[23], Some((OwnerKey::IccProfile, 4093)));
    assert_eq!(maker.requests[26], Some((OwnerKey::UnknownPayload(1), 13)));
    assert_eq!(maker.sample_capacity, 8);
    assert_eq!(maker.provenance_capacity, 5);
    assert_eq!(maker.icc_capacity, 4160);
    assert_eq!(maker.unknown_payload_capacity, 4160);
    assert_eq!(c2_owned_output_bytes(&output), exact_frame);
    c2e4_assert_capacities(&output);
    assert_c2b_output_matches(&reference, &output);
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        (exact_frame, exact_metadata, exact_live, (0, 0), 31)
    );
    assert!(!super::super::native::DeallocationObservationGuard::target_deallocated());
    #[cfg(miri)]
    super::super::native::drop_registered_value(output);
    #[cfg(not(miri))]
    drop(output);
    assert!(super::super::native::DeallocationObservationGuard::target_deallocated());
    assert_eq!(
        super::super::native::DeallocationObservationGuard::target_deallocations(),
        1
    );
    c2e4_assert_prefix_shape();
    #[cfg(not(miri))]
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (26, 26, true)
    );
    drop(boundary);
    c2_assert_source_unchanged(&source, &source_snapshot);

    // The source's requested values fit these rows.  The final ordinal-26
    // candidate is therefore the first point at which each one-under limit
    // can reject the accumulated actual capacities.
    for (frame_limit, live_limit, metadata_limit) in [
        (exact_frame - 1, exact_live, exact_metadata),
        (exact_frame, exact_live - 1, exact_metadata),
        (exact_frame, exact_live, exact_metadata - 1),
    ] {
        let source_snapshot = c2_source_snapshot(&source);
        let row_limits = c2e4_limits(frame_limit, live_limit, metadata_limit);
        assert!(source.validate_with_limits(&row_limits).is_ok());
        let mut prepared = PreparedConversion::new(&source, &options, &row_limits).unwrap();
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
        RESTORE_TARGET_SNAPSHOT.with(|snapshot| snapshot.set(None));
        RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| snapshot.set(None));
        RESTORE_OWNER_STATUS.with(|status| status.set(None));
        let boundary = RestoreObserverGuard::begin();
        let mut failing = C2e4AggregateMaker::new(true);
        let result = prepared.materialize_with(&mut failing);
        assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
        assert_eq!(failing.request_count, 27);
        assert_eq!(
            failing.requests[26],
            Some((OwnerKey::UnknownPayload(1), 13))
        );
        assert_eq!(failing.sample_capacity, 8);
        assert_eq!(failing.provenance_capacity, 5);
        assert_eq!(failing.icc_capacity, 4160);
        assert_eq!(failing.unknown_payload_capacity, 4160);
        assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
        assert_eq!(RESTORE_TARGET_SNAPSHOT.with(Cell::get), Some(true));
        assert_eq!(RESTORE_TARGET_DEALLOCATIONS.with(Cell::get), Some(1));
        c2e4_assert_prefix_shape();
        #[cfg(miri)]
        assert_eq!(
            super::super::native::DeallocationObservationGuard::registered_status(),
            (26, 0, false)
        );
        #[cfg(not(miri))]
        {
            assert_eq!(RESTORE_OWNER_STATUS.with(Cell::get), Some((26, 26, true)));
            assert_eq!(
                super::super::native::DeallocationObservationGuard::registered_status(),
                (26, 26, true)
            );
        }
        assert_eq!(prepared.admitted.c1_accounting_snapshot(), before);
        c2_assert_source_unchanged(&source, &source_snapshot);
        drop(boundary);

        let mut ordinary = C2e4AggregateMaker::new(false);
        let output = prepared.materialize_with(&mut ordinary).unwrap();
        assert_eq!(ordinary.request_count, 27);
        assert_c2b_output_matches(&reference, &output);
        assert_eq!(
            prepared.admitted.c1_accounting_snapshot(),
            (
                inventory.requested_output,
                inventory.requested_metadata,
                inventory.source_live + inventory.requested_output,
                (0, 0),
                31,
            )
        );
        assert!(matches!(
            prepared.materialize_with(&mut ordinary),
            Err(ProcessingError::Unsupported(_))
        ));
        drop(output);
        c2_assert_source_unchanged(&source, &source_snapshot);
    }
}

fn c2_checked_add(left: usize, right: usize) -> usize {
    left.checked_add(right).unwrap()
}

fn c2_checked_mul(left: usize, right: usize) -> usize {
    left.checked_mul(right).unwrap()
}

fn c2_owned_color_bytes(color: &ColorInformationSet) -> usize {
    let mut total = c2_checked_mul(
        color.provenance_capacity_for_test(),
        size_of::<ColorProvenance>(),
    );
    total = c2_checked_add(total, color.icc_profile_capacity());
    if color.nclx().is_some() {
        total = c2_checked_add(total, 8);
    }
    if color.av1().is_some() {
        total = c2_checked_add(total, size_of::<Av1ColorInformation>());
    }
    total = c2_checked_add(
        total,
        c2_checked_mul(
            color.unknown_colr_capacity_for_test(),
            size_of::<UnknownColorInformation>(),
        ),
    );
    for item in color.unknown_colr() {
        total = c2_checked_add(total, item.payload.capacity());
    }
    total
}

fn c2_owned_output_bytes(frame: &ImageFrame) -> usize {
    let descriptor = frame.descriptor();
    let mut total = c2_checked_mul(
        descriptor.planes_capacity_for_test(),
        size_of::<PlaneDescriptor>(),
    );
    for plane in descriptor.planes() {
        total = c2_checked_add(
            total,
            c2_checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        total = c2_checked_add(
            total,
            c2_checked_mul(plane.roles_capacity_for_test(), size_of::<ChannelRole>()),
        );
    }
    let pixels = frame.pixels().f32_planes().unwrap();
    total = c2_checked_add(
        total,
        c2_checked_mul(
            frame.pixels().f32_planes_capacity_for_test().unwrap(),
            size_of::<Plane<f32>>(),
        ),
    );
    for plane in pixels {
        total = c2_checked_add(
            total,
            c2_checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        total = c2_checked_add(
            total,
            c2_checked_mul(plane.sample_capacity(), size_of::<f32>()),
        );
    }
    let metadata = frame.metadata();
    total = c2_checked_add(total, c2_owned_color_bytes(metadata.source_color()));
    total = c2_checked_add(
        total,
        c2_checked_mul(
            metadata.coded_geometry_capacity_for_test(),
            size_of::<GeometryOperation>(),
        ),
    );
    total = c2_checked_add(
        total,
        c2_checked_mul(
            metadata.render_geometry_capacity_for_test(),
            size_of::<GeometryOperation>(),
        ),
    );
    if let Some(info) = metadata.pixel_information() {
        total = c2_checked_add(total, info.bits_capacity_for_test());
        total = c2_checked_add(
            total,
            c2_checked_mul(
                info.extended_capacity_for_test().unwrap_or(0),
                size_of::<PixelChannelInformation>(),
            ),
        );
    }
    total = c2_checked_add(
        total,
        c2_checked_mul(
            [metadata.coded_dimensions(), metadata.render_dimensions()]
                .into_iter()
                .filter(|value| value.is_some())
                .count(),
            8,
        ),
    );
    total
}

#[test]
fn c2_real_prepared_conversion_walks_actual_capacity_and_completes() {
    use super::super::super::output_plan::tests::{c2_packed_rich_source, c2_rich_inventory};

    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let delta = [
        12,
        67,
        c2_checked_mul(2, size_of::<ColorProvenance>()),
        4160usize.checked_sub(13).unwrap(),
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap();
    let limits = ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(8)
        .max_height(8)
        .max_pixels(64)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(32)
        .max_frame_bytes(std::cmp::max(
            inventory.source_live,
            c2_checked_add(inventory.requested_output, delta),
        ))
        .max_total_live_decoded_bytes(c2_checked_add(
            c2_checked_add(inventory.source_live, inventory.requested_output),
            delta,
        ))
        .max_references(8)
        .max_frame_count(8)
        .max_metadata_bytes(std::cmp::max(
            c2_checked_add(inventory.source_metadata, inventory.active_color),
            [
                inventory.requested_metadata,
                67,
                c2_checked_mul(2, size_of::<ColorProvenance>()),
                4160usize.checked_sub(13).unwrap(),
            ]
            .into_iter()
            .try_fold(0usize, |total, value| total.checked_add(value))
            .unwrap(),
        ))
        .max_icc_bytes(4160)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(128)
        .max_parser_depth(16)
        .max_grid_cells(64)
        .max_derived_work(64)
        .max_derived_depth(8)
        .build()
        .unwrap();
    assert!(source.validate_with_limits(&limits).is_ok());
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )));
    let mut prepared = PreparedConversion::new(&source, &options, &limits).unwrap();
    let mut maker = C2PreparedMaker;
    let output = prepared.materialize_with(&mut maker).unwrap();
    assert_eq!(
        c2_owned_output_bytes(&output),
        c2_checked_add(inventory.requested_output, delta)
    );
    assert_eq!(
        output.pixels().f32_planes().unwrap()[0].sample_capacity(),
        8
    );
    assert_eq!(
        output.metadata().source_color().icc_profile_capacity(),
        4160
    );
    assert_eq!(
        output
            .metadata()
            .source_color()
            .provenance_capacity_for_test(),
        5
    );
    assert_eq!(
        output.metadata().source_color().unknown_colr()[1]
            .payload
            .capacity(),
        4160
    );
}

fn assert_c2b_output_matches(reference: &ImageFrame, actual: &ImageFrame) {
    assert_eq!(actual, reference);
    assert_eq!(actual.descriptor().alpha(), reference.descriptor().alpha());
    let expected = reference.pixels().f32_planes().unwrap();
    let observed = actual.pixels().f32_planes().unwrap();
    assert_eq!(observed.len(), expected.len());
    for (observed, expected) in observed.iter().zip(expected) {
        assert_eq!(observed.samples(), expected.samples());
    }
    assert_eq!(actual.metadata(), reference.metadata());
    assert_eq!(actual.timing(), reference.timing());
    assert_eq!(
        actual.metadata().last_conversion(),
        reference.metadata().last_conversion()
    );
}

fn c2b_denial_preserves_source_and_retries(
    deny_key: OwnerKey,
    expected_prefix: &[(u8, usize, usize)],
    denied_count: usize,
    denied_ordinal: usize,
) {
    c2b_denial_preserves_source_and_retries_with_max_icc(
        deny_key,
        expected_prefix,
        denied_count,
        denied_ordinal,
        1 << 20,
    );
}

fn c2b_denial_preserves_source_and_retries_with_max_icc(
    deny_key: OwnerKey,
    expected_prefix: &[(u8, usize, usize)],
    denied_count: usize,
    denied_ordinal: usize,
    max_icc_bytes: usize,
) {
    use super::super::super::output_plan::tests::{
        c2_assert_source_unchanged, c2_packed_rich_source, c2_source_snapshot,
    };

    let source = c2_packed_rich_source();
    let source_snapshot = c2_source_snapshot(&source);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_source(SourceInterpretation::Cicp(NclxColorInformation::new(
        1, 8, 0, true,
    )));
    let reference = {
        let mut prepared = PreparedConversion::new(&source, &options, &limits()).unwrap();
        let mut maker = OrdinaryMaker;
        prepared.materialize_with(&mut maker).unwrap()
    };

    RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
    RESTORE_OWNER_STATUS.with(|status| status.set(None));
    let observer = RestoreObserverGuard::begin();
    let resource_limits = {
        let mut limits = limits();
        limits.max_icc_bytes = max_icc_bytes;
        limits
    };
    let mut prepared = PreparedConversion::new(&source, &options, &resource_limits).unwrap();
    let before_failure = prepared.admitted.c1_accounting_snapshot();
    let mut denied = RealAllocationDenyMaker::new(deny_key);
    let failed = prepared.materialize_with(&mut denied);
    assert!(matches!(failed, Err(ProcessingError::Allocation(_))));
    assert_eq!(
        prepared.admitted.c1_accounting_snapshot(),
        before_failure,
        "failed materialization must restore every ledger counter and cursor"
    );
    assert_eq!(denied.denied_calls, 1);
    assert_eq!(
        denied.denied_request,
        Some(DeniedRequest {
            owner_key: deny_key,
            key: owner_key_code(deny_key),
            count: denied_count,
            ordinal: denied_ordinal,
        })
    );
    assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
    let owner_status = RESTORE_OWNER_STATUS.with(Cell::get).unwrap();
    assert_eq!(owner_status.0, expected_prefix.len());
    assert_eq!(owner_status.1, expected_prefix.len());
    assert!(
        owner_status.2,
        "every admitted owner must be freed exactly once"
    );
    let (owners, owner_count) =
        super::super::native::DeallocationObservationGuard::registered_snapshot();
    assert_eq!(owner_count, expected_prefix.len());
    for (owner, &(expected_key, expected_count, expected_ordinal)) in
        owners[..owner_count].iter().zip(expected_prefix)
    {
        assert_eq!(
            (owner.owner_key, owner.key, owner.count, owner.ordinal),
            (
                owner_key_from_registration(expected_key, expected_ordinal),
                expected_key,
                expected_count,
                expected_ordinal
            )
        );
        assert_eq!(owner.deallocations, 1);
    }
    assert!(
        owners[..owner_count]
            .iter()
            .all(|owner| owner.owner_key != deny_key),
        "the denied candidate must not be registered"
    );
    c2_assert_source_unchanged(&source, &source_snapshot);

    let mut ordinary = OrdinaryMaker;
    let actual = prepared.materialize_with(&mut ordinary).unwrap();
    assert_c2b_output_matches(&reference, &actual);
    assert!(matches!(
        prepared.materialize_with(&mut ordinary),
        Err(ProcessingError::Unsupported(_))
    ));
    drop(actual);
    drop(observer);

    // The one-shot TLS arm must not affect an allocation after its guard has
    // been dropped, and the fixed registry must be restored on scope exit.
    let mut cleanup_probe = Vec::<u8>::new();
    cleanup_probe.try_reserve_exact(1).unwrap();
    drop(cleanup_probe);
}

#[test]
fn c2b_allocation_denial_is_thread_local_and_guard_cleanup_is_exact() {
    let source = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let (sender, receiver) = mpsc::channel();
    let worker_ready = Arc::clone(&ready);
    let worker_release = Arc::clone(&release);
    let worker_source = source.clone();
    let worker = std::thread::spawn(move || {
        worker_ready.wait();
        worker_release.wait();
        let options = ColorConvertOptions::new(Destination::linear_rgb(
            SampleDomain::LinearRelative,
            RgbPrimaries::srgb(),
        ));
        sender
            .send(convert_frame(&worker_source, &options, &limits()))
            .unwrap();
    });

    // Wait until the worker is parked, then arm the denial on this thread.
    // The worker has a distinct TLS slot and must not consume it.
    ready.wait();
    let denial = super::super::native::ActualAllocationDenyGuard::arm_once();
    release.wait();
    let mut ordinary = OrdinaryMaker;
    assert!(matches!(
        ordinary.make::<f32>(OwnerKey::Sample(0), 1),
        Err(ProcessingError::Allocation(_))
    ));
    let worker_result = receiver.recv().unwrap();
    assert!(
        worker_result.is_ok(),
        "worker conversion consumed main TLS state"
    );
    worker.join().unwrap();
    drop(denial);

    // Guard cleanup restores the previous state; a subsequent allocation is
    // successful and the worker's cleanup cannot mask a leaked denial.
    let values = ordinary.make::<f32>(OwnerKey::Sample(0), 1).unwrap();
    assert!(values.is_empty());
}

#[test]
fn c2b_real_sample_allocation_denial_frees_prior_owners_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Sample(2),
        &[(0x10, 5, 0), (0x11, 5, 1)],
        5,
        2,
    );
}

#[test]
fn c2c6_sample_zero_denial_has_empty_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(OwnerKey::Sample(0), &[], 5, 0);
}

#[test]
fn c2c6_sample_one_denial_frees_sample_zero_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(OwnerKey::Sample(1), &[(0x10, 5, 0)], 5, 1);
}

#[test]
fn c2c6_sample_three_denial_frees_samples_zero_through_two_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Sample(3),
        &[(0x10, 5, 0), (0x11, 5, 1), (0x12, 5, 2)],
        5,
        3,
    );
}

#[test]
fn c2b_real_unknown_payload_denial_frees_all_prior_owners_and_retries() {
    c2b_denial_preserves_source_and_retries_with_max_icc(
        OwnerKey::UnknownPayload(1),
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
            (5, 1, 13),
            (6, 1, 14),
            (4, 1, 15),
            (5, 1, 16),
            (6, 1, 17),
            (7, 2, 18),
            (8, 1, 19),
            (9, 4, 20),
            (10, 4, 21),
            (11, 3, 22),
            (12, 4093, 23),
            (13, 2, 24),
            (0x40, 13, 25),
        ],
        13,
        26,
        4093,
    );
}

#[test]
fn c2c1_descriptor_outer_denial_frees_only_successful_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::DescriptorOuter,
        &[(0x10, 5, 0), (0x11, 5, 1), (0x12, 5, 2), (0x13, 5, 3)],
        4,
        4,
    );
}

#[test]
fn c2c1_pixel_outer_denial_frees_only_successful_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixelOuter,
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
        ],
        4,
        5,
    );
}

#[test]
fn c2c1_descriptor_layout_plane0_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::DescriptorLayout { plane: 0 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
        ],
        1,
        6,
    );
}

#[test]
fn c2c1_role_plane0_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Role { plane: 0 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
        ],
        1,
        7,
    );
}

#[test]
fn c2c1_pixel_layout_plane0_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixelLayout { plane: 0 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
        ],
        1,
        8,
    );
}

#[test]
fn c2c2_descriptor_layout_plane1_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::DescriptorLayout { plane: 1 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
        ],
        1,
        9,
    );
}

#[test]
fn c2c2_role_plane1_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Role { plane: 1 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
        ],
        1,
        10,
    );
}

#[test]
fn c2c2_pixel_layout_plane1_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixelLayout { plane: 1 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
        ],
        1,
        11,
    );
}

#[test]
fn c2c2_descriptor_layout_plane2_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::DescriptorLayout { plane: 2 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
        ],
        1,
        12,
    );
}

#[test]
fn c2c2_role_plane2_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Role { plane: 2 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
        ],
        1,
        13,
    );
}

#[test]
fn c2c2_pixel_layout_plane2_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixelLayout { plane: 2 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
            (5, 1, 13),
        ],
        1,
        14,
    );
}

#[test]
fn c2c2_descriptor_layout_plane3_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::DescriptorLayout { plane: 3 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
            (5, 1, 13),
            (6, 1, 14),
        ],
        1,
        15,
    );
}

#[test]
fn c2c2_role_plane3_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::Role { plane: 3 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
            (5, 1, 13),
            (6, 1, 14),
            (4, 1, 15),
        ],
        1,
        16,
    );
}

#[test]
fn c2c2_pixel_layout_plane3_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixelLayout { plane: 3 },
        &[
            (0x10, 5, 0),
            (0x11, 5, 1),
            (0x12, 5, 2),
            (0x13, 5, 3),
            (2, 4, 4),
            (3, 4, 5),
            (4, 1, 6),
            (5, 1, 7),
            (6, 1, 8),
            (4, 1, 9),
            (5, 1, 10),
            (6, 1, 11),
            (4, 1, 12),
            (5, 1, 13),
            (6, 1, 14),
            (4, 1, 15),
            (5, 1, 16),
        ],
        1,
        17,
    );
}

const C2C3_OWNER_PREFIX: [(u8, usize, usize); 21] = [
    (0x10, 5, 0),
    (0x11, 5, 1),
    (0x12, 5, 2),
    (0x13, 5, 3),
    (2, 4, 4),
    (3, 4, 5),
    (4, 1, 6),
    (5, 1, 7),
    (6, 1, 8),
    (4, 1, 9),
    (5, 1, 10),
    (6, 1, 11),
    (4, 1, 12),
    (5, 1, 13),
    (6, 1, 14),
    (4, 1, 15),
    (5, 1, 16),
    (6, 1, 17),
    (7, 2, 18),
    (8, 1, 19),
    (9, 4, 20),
];

const C2C4_OWNER_PREFIX: [(u8, usize, usize); 25] = [
    (0x10, 5, 0),
    (0x11, 5, 1),
    (0x12, 5, 2),
    (0x13, 5, 3),
    (2, 4, 4),
    (3, 4, 5),
    (4, 1, 6),
    (5, 1, 7),
    (6, 1, 8),
    (4, 1, 9),
    (5, 1, 10),
    (6, 1, 11),
    (4, 1, 12),
    (5, 1, 13),
    (6, 1, 14),
    (4, 1, 15),
    (5, 1, 16),
    (6, 1, 17),
    (7, 2, 18),
    (8, 1, 19),
    (9, 4, 20),
    (10, 4, 21),
    (11, 3, 22),
    (12, 4093, 23),
    (13, 2, 24),
];

#[test]
fn c2c4_provenance_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(OwnerKey::Provenance, &C2C4_OWNER_PREFIX[..22], 3, 22);
}

#[test]
fn c2c4_icc_denial_is_allocator_failure_at_icc_limit_and_retries() {
    c2b_denial_preserves_source_and_retries_with_max_icc(
        OwnerKey::IccProfile,
        &C2C4_OWNER_PREFIX[..23],
        4093,
        23,
        4093,
    );
}

#[test]
fn c2c4_unknown_outer_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::UnknownOuter,
        &C2C4_OWNER_PREFIX[..24],
        2,
        24,
    );
}

#[test]
fn c2c4_unknown_payload_zero_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::UnknownPayload(0),
        &C2C4_OWNER_PREFIX[..25],
        13,
        25,
    );
}

#[test]
fn c2c3_coded_geometry_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::CodedGeometry,
        &C2C3_OWNER_PREFIX[..18],
        2,
        18,
    );
}

#[test]
fn c2c3_render_geometry_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::RenderGeometry,
        &C2C3_OWNER_PREFIX[..19],
        1,
        19,
    );
}

#[test]
fn c2c3_pixi_bits_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(OwnerKey::PixiBits, &C2C3_OWNER_PREFIX[..20], 4, 20);
}

#[test]
fn c2c3_pixi_extended_denial_frees_prefix_and_retries() {
    c2b_denial_preserves_source_and_retries(
        OwnerKey::PixiExtended,
        &C2C3_OWNER_PREFIX[..21],
        4,
        21,
    );
}

fn c2c1_consume_inline_prefix(
    admitted: &mut super::super::super::output_plan::AdmittedOutputPlan<'_>,
    maker: &mut impl CandidateMaker,
) {
    for plane in 0..3 {
        admitted
            .fresh_with::<f32, _>(OwnerKey::Sample(plane), 1, maker)
            .unwrap();
    }
    admitted
        .fresh_with::<PlaneDescriptor, _>(OwnerKey::DescriptorOuter, 3, maker)
        .unwrap();
    admitted
        .fresh_with::<Plane<f32>, _>(OwnerKey::PixelOuter, 3, maker)
        .unwrap();
    for plane in 0..3 {
        admitted
            .fresh_with::<usize, _>(OwnerKey::DescriptorLayout { plane }, 1, maker)
            .unwrap();
        admitted
            .fresh_with::<ChannelRole, _>(OwnerKey::Role { plane }, 1, maker)
            .unwrap();
        admitted
            .fresh_with::<usize, _>(OwnerKey::PixelLayout { plane }, 1, maker)
            .unwrap();
    }
}

#[test]
fn c2c1_zero_count_coded_geometry_advances_without_allocator_or_registry() {
    use super::super::super::output_plan::OutputOwnershipPlan;
    use super::super::super::output_plan::tests::c2c1_inline_source;

    let source = c2c1_inline_source();
    let limits = limits();
    let plan = OutputOwnershipPlan::inspect(&source, 3, 1, &limits).unwrap();
    let ledger =
        super::super::super::allocation::ConstructionLedger::new_with_ownership(0, 0, 0, &limits)
            .unwrap();
    let mut admitted = plan.admit(ledger, source.metadata()).unwrap();
    let mut maker = OrdinaryMaker;
    c2c1_consume_inline_prefix(&mut admitted, &mut maker);
    let before = admitted.c1_accounting_snapshot();
    let deallocations = super::super::native::DeallocationObservationGuard::begin();
    let allocations = super::super::native::AllocationGuard::begin();
    let candidate = admitted
        .fresh_metadata::<GeometryOperation, _>(OwnerKey::CodedGeometry, 0, &mut maker)
        .unwrap();
    assert!(candidate.is_empty());
    assert_eq!(allocations.allocations(), 0);
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (0, 0, true)
    );
    assert_eq!(
        admitted.c1_accounting_snapshot(),
        (before.0, before.1, before.2, before.3, before.4 + 1)
    );
    drop(allocations);
    drop(deallocations);
}

#[test]
fn c2c1_source_nclx_inline_commit_has_no_allocator_side_effect() {
    use super::super::super::output_plan::OutputOwnershipPlan;
    use super::super::super::output_plan::tests::c2c1_inline_source;

    let source = c2c1_inline_source();
    let limits = limits();
    let plan = OutputOwnershipPlan::inspect(&source, 3, 1, &limits).unwrap();
    let ledger =
        super::super::super::allocation::ConstructionLedger::new_with_ownership(0, 0, 0, &limits)
            .unwrap();
    let mut admitted = plan.admit(ledger, source.metadata()).unwrap();
    let mut maker = OrdinaryMaker;
    c2c1_consume_inline_prefix(&mut admitted, &mut maker);
    admitted
        .fresh_metadata::<GeometryOperation, _>(OwnerKey::CodedGeometry, 0, &mut maker)
        .unwrap();
    admitted
        .fresh_metadata::<GeometryOperation, _>(OwnerKey::RenderGeometry, 0, &mut maker)
        .unwrap();
    admitted
        .fresh_metadata::<ColorProvenance, _>(OwnerKey::Provenance, 1, &mut maker)
        .unwrap();
    admitted
        .fresh_metadata::<UnknownColorInformation, _>(OwnerKey::UnknownOuter, 0, &mut maker)
        .unwrap();
    let before = admitted.c1_accounting_snapshot();
    assert_eq!(before.3, (8, 8));
    let deallocations = super::super::native::DeallocationObservationGuard::begin();
    let allocations = super::super::native::AllocationGuard::begin();
    admitted
        .commit_metadata_event(OwnerKey::SourceNclx, 8)
        .unwrap();
    assert_eq!(allocations.allocations(), 0);
    assert_eq!(
        admitted.c1_accounting_snapshot(),
        (
            before.0 + 8,
            before.1 + 8,
            before.2 + 8,
            (0, 0),
            before.4 + 1,
        )
    );
    assert_eq!(
        super::super::native::DeallocationObservationGuard::registered_status(),
        (0, 0, true)
    );
    drop(allocations);
    drop(deallocations);
}

#[test]
fn c2c5_inline_events_are_exact_transactional_and_allocator_free() {
    use super::super::super::metadata::AV1_COLOR_INFORMATION_BYTES;
    use super::super::super::output_plan::OutputOwnershipPlan;
    use super::super::super::output_plan::tests::{
        C2CapacityMaker, c2_consume_prefix_to_provenance, c2_packed_rich_source,
    };

    let rows = [
        (OwnerKey::SourceAv1, AV1_COLOR_INFORMATION_BYTES, 28),
        (OwnerKey::CodedDimensions, 8, 29),
        (OwnerKey::RenderDimensions, 8, 30),
    ];
    for (key, bytes, ordinal) in rows {
        let source = c2_packed_rich_source();
        let limits = limits();
        let plan = OutputOwnershipPlan::inspect(&source, 4, 5, &limits).unwrap();
        let ledger = super::super::super::allocation::ConstructionLedger::new_with_ownership(
            0, 0, 0, &limits,
        )
        .unwrap();
        let mut admitted = plan.admit(ledger, source.metadata()).unwrap();
        let mut maker = C2CapacityMaker::none();
        c2_consume_prefix_to_provenance(&mut admitted, &mut maker).unwrap();
        admitted
            .fresh_metadata::<ColorProvenance, _>(OwnerKey::Provenance, 3, &mut maker)
            .unwrap();
        admitted
            .fresh_metadata::<u8, _>(OwnerKey::IccProfile, 4093, &mut maker)
            .unwrap();
        admitted
            .fresh_metadata::<UnknownColorInformation, _>(OwnerKey::UnknownOuter, 2, &mut maker)
            .unwrap();
        admitted
            .fresh_metadata::<u8, _>(OwnerKey::UnknownPayload(0), 13, &mut maker)
            .unwrap();
        admitted
            .fresh_metadata::<u8, _>(OwnerKey::UnknownPayload(1), 13, &mut maker)
            .unwrap();
        admitted
            .commit_metadata_event(OwnerKey::SourceNclx, 8)
            .unwrap();
        if key == OwnerKey::CodedDimensions || key == OwnerKey::RenderDimensions {
            admitted
                .commit_metadata_event(OwnerKey::SourceAv1, AV1_COLOR_INFORMATION_BYTES)
                .unwrap();
        }
        if key == OwnerKey::RenderDimensions {
            admitted
                .commit_metadata_event(OwnerKey::CodedDimensions, 8)
                .unwrap();
        }

        let before = admitted.c1_accounting_snapshot();
        assert_eq!(before.4, ordinal);
        let checkpoint = admitted.checkpoint();
        let deallocations = super::super::native::DeallocationObservationGuard::begin();
        let allocations = super::super::native::AllocationGuard::begin();
        let denial = super::super::native::ActualAllocationDenyGuard::arm_once();
        let commit_result = admitted.commit_metadata_event(key, bytes);
        let after = admitted.c1_accounting_snapshot();
        let observed_allocations = allocations.allocations();
        let observed_registry =
            super::super::native::DeallocationObservationGuard::registered_status();
        drop(allocations);
        drop(deallocations);

        // The inline commit cannot consume the one-shot denial.  A separate
        // one-byte reserve must observe it while the guard is still armed.
        let mut denied = Vec::<u8>::new();
        let denied_result = denied.try_reserve_exact(1);
        drop(denied);
        drop(denial);
        let mut cleanup = Vec::<u8>::new();
        let cleanup_result = cleanup.try_reserve_exact(1);
        drop(cleanup);

        assert_eq!(
            after,
            (
                before.0 + bytes,
                before.1 + bytes,
                before.2 + bytes,
                (before.3.0 - bytes, before.3.1 - bytes),
                ordinal + 1,
            )
        );
        assert_eq!(observed_allocations, 0);
        assert_eq!(observed_registry, (0, 0, true));
        assert!(denied_result.is_err());
        assert!(
            cleanup_result.is_ok(),
            "cleanup reserve failed: {cleanup_result:?}"
        );
        assert!(
            commit_result.is_ok(),
            "inline commit failed: {commit_result:?}"
        );

        admitted.restore(checkpoint);
        assert_eq!(admitted.c1_accounting_snapshot(), before);

        let deallocations = super::super::native::DeallocationObservationGuard::begin();
        let allocations = super::super::native::AllocationGuard::begin();
        let denial = super::super::native::ActualAllocationDenyGuard::arm_once();
        let recommit_result = admitted.commit_metadata_event(key, bytes);
        let recommitted = admitted.c1_accounting_snapshot();
        let recommit_allocations = allocations.allocations();
        let recommit_registry =
            super::super::native::DeallocationObservationGuard::registered_status();
        drop(denial);
        drop(allocations);
        drop(deallocations);
        assert_eq!(recommitted, after);
        assert_eq!(recommit_allocations, 0);
        assert_eq!(recommit_registry, (0, 0, true));
        assert!(
            recommit_result.is_ok(),
            "inline recommit failed: {recommit_result:?}"
        );
    }
}

#[test]
fn prepared_failure_observes_real_owner_drop_before_restore_and_retries() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
    let observer = RestoreObserverGuard::begin();
    let mut prepared = PreparedConversion::new(&frame, &options, &limits()).unwrap();
    let mut failing = FailOnceMaker {
        fail_sample_one: true,
    };
    assert!(matches!(
        prepared.materialize_with(&mut failing),
        Err(ProcessingError::Allocation(_))
    ));
    assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
    assert!(observer.boundary_deallocations() > 0);
    let mut ordinary = OrdinaryMaker;
    assert!(prepared.materialize_with(&mut ordinary).is_ok());
}

#[test]
fn prepared_failure_snapshots_sample_owner_drop_at_restore_boundary() {
    let frame = rgb_frame(vec![64, 128, 192], 1, 1, None);
    let options = ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ));
    RESTORE_BOUNDARY_COUNT.with(|count| count.set(0));
    RESTORE_TARGET_SNAPSHOT.with(|snapshot| snapshot.set(None));
    RESTORE_TARGET_DEALLOCATIONS.with(|snapshot| snapshot.set(None));
    let boundary = RestoreObserverGuard::begin();
    let mut prepared = PreparedConversion::new(&frame, &options, &limits()).unwrap();
    let mut failing = ObservedFailOnceMaker {
        fail_sample_one: true,
    };
    assert!(matches!(
        prepared.materialize_with(&mut failing),
        Err(ProcessingError::Allocation(_))
    ));
    assert_eq!(RESTORE_BOUNDARY_COUNT.with(Cell::get), 1);
    assert_eq!(RESTORE_TARGET_SNAPSHOT.with(Cell::get), Some(true));
    assert_eq!(RESTORE_TARGET_DEALLOCATIONS.with(Cell::get), Some(1));
    assert!(boundary.boundary_deallocations() > 0);
    let mut ordinary = OrdinaryMaker;
    assert!(prepared.materialize_with(&mut ordinary).is_ok());
    drop(failing);
    drop(boundary);
}

#[test]
fn tc13_non_identity_inverse_preserves_sign_at_and_around_junction() {
    const JUNCTION: f64 = 0.039_293_370_676_847_54;
    const EXPECTED_HALF: f64 = 0.214_045_842_492_543_21;
    for value in [JUNCTION - 1.0e-12, JUNCTION, JUNCTION + 1.0e-12, 0.5] {
        let positive = super::inverse_cicp_transfer(value, 13, false).unwrap();
        let negative = super::inverse_cicp_transfer(-value, 13, false).unwrap();
        assert!((positive + negative).abs() <= 1.0e-15);
    }
    let negative_half = super::inverse_cicp_transfer(-0.5, 13, false).unwrap();
    assert!((negative_half + EXPECTED_HALF).abs() <= 1.0e-15);
}
