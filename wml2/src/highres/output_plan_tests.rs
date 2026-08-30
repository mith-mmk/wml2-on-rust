use super::{AdmittedOutputPlan, CandidateMaker, OutputOwnershipPlan, OwnerElement, OwnerKey};
use crate::highres::allocation::ConstructionLedger;
use crate::highres::{
    AlphaAssociation, Av1ColorInformation, Av1Description, ChannelModel, ChannelRole,
    ColorInformationSet, ColorProvenance, FrameMetadata, FrameTiming, GeometryOperation,
    IccColorType, ImageDescriptor, ImageFrame, NclxColorInformation, PixelBuffer,
    PixelChannelInformation, PixelInformation, Plane, PlaneDescriptor, PlaneLayout,
    ProcessingError, ResourceLimits, RgbPrimaries, Rotation, SampleDomain, Subsampling,
    UnknownColorInformation,
};
use std::mem::size_of;

impl<'a> AdmittedOutputPlan<'a> {
    pub(crate) fn c1_accounting_snapshot(&self) -> (usize, usize, usize, (usize, usize), usize) {
        (
            self.ledger.frame_bytes,
            self.ledger.metadata_bytes_for_test(),
            self.ledger.live_bytes,
            self.pending_bytes(),
            self.owner_cursor,
        )
    }
}

fn limits(frame_bytes: usize, live_bytes: usize) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(8)
        .max_height(8)
        .max_pixels(64)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 20)
        .max_frame_bytes(frame_bytes)
        .max_total_live_decoded_bytes(live_bytes)
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

fn c2_limits(
    frame_bytes: usize,
    live_bytes: usize,
    metadata_bytes: usize,
    max_plane_bytes: usize,
    max_icc_bytes: usize,
) -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(8)
        .max_height(8)
        .max_pixels(64)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(max_plane_bytes)
        .max_frame_bytes(frame_bytes)
        .max_total_live_decoded_bytes(live_bytes)
        .max_references(8)
        .max_frame_count(8)
        .max_metadata_bytes(metadata_bytes)
        .max_icc_bytes(max_icc_bytes)
        .max_clut_bytes(1 << 20)
        .max_parser_entries(128)
        .max_parser_depth(16)
        .max_grid_cells(64)
        .max_derived_work(64)
        .max_derived_depth(8)
        .build()
        .unwrap()
}

fn c2_boundary_limits(
    inventory: C2RichInventory,
    frame_bytes: usize,
    live_bytes: usize,
    metadata_bytes: usize,
    max_plane_bytes: usize,
    max_icc_bytes: usize,
) -> ResourceLimits {
    c2_limits(
        frame_bytes.max(inventory.source_live),
        live_bytes.max(inventory.source_live),
        metadata_bytes.max(inventory.source_metadata),
        max_plane_bytes,
        max_icc_bytes,
    )
}

fn source() -> ImageFrame {
    let layout = PlaneLayout::planar(1, 1, Subsampling::FULL).unwrap();
    let descriptors = [
        (ChannelRole::Red, 1u8),
        (ChannelRole::Green, 2),
        (ChannelRole::Blue, 3),
    ]
    .into_iter()
    .map(|(role, _)| PlaneDescriptor::planar(layout.clone(), role, 8).unwrap())
    .collect();
    let planes = [1u8, 2, 3]
        .into_iter()
        .map(|sample| Plane::new(layout.clone(), vec![sample]).unwrap())
        .collect();
    let descriptor = ImageDescriptor::new(1, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(AlphaAssociation::None)
        .unwrap()
        .with_domain(SampleDomain::LinearRelative)
        .with_primaries(RgbPrimaries::srgb());
    ImageFrame::new(descriptor, PixelBuffer::u8(planes).unwrap()).unwrap()
}

/// A small source whose metadata contains only the inline nclx event.  C2c-1
/// uses it for the zero-count and inline-owner controls; all heap owners have
/// an empty logical count, so those controls do not depend on fixture slack.
pub(crate) fn c2c1_inline_source() -> ImageFrame {
    source().with_metadata(FrameMetadata::new(
        ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 8, 0, true)),
    ))
}

fn c1_vec<T>(values: impl IntoIterator<Item = T>, capacity: usize) -> Vec<T> {
    let mut result = Vec::with_capacity(capacity);
    result.extend(values);
    assert!(result.capacity() >= capacity);
    result
}

/// The C1 rich source deliberately gives every heap owner a distinct shape.
/// The optional ICC spare capacity is retained in the source, while output
/// admission is based on the requested clone length.
pub(crate) fn c1_rich_source(icc_spare: bool) -> ImageFrame {
    let mut descriptors = c1_vec(std::iter::empty(), 4);
    let mut pixels = c1_vec(std::iter::empty(), 4);
    for role in [
        ChannelRole::Red,
        ChannelRole::Green,
        ChannelRole::Blue,
        ChannelRole::Alpha,
    ] {
        let descriptor_offsets = c1_vec([0usize], 1);
        let descriptor_layout =
            PlaneLayout::new(5, 1, 5, 1, descriptor_offsets, Subsampling::FULL).unwrap();
        let roles = c1_vec([role], 1);
        descriptors.push(PlaneDescriptor::new(descriptor_layout, roles, 32).unwrap());

        let pixel_offsets = c1_vec([0usize], 1);
        let pixel_layout = PlaneLayout::new(5, 1, 5, 1, pixel_offsets, Subsampling::FULL).unwrap();
        let samples = c1_vec([0.0f32, 0.1, 0.25, 0.5, 1.0], 5);
        pixels.push(Plane::new(pixel_layout, samples).unwrap());
    }

    let descriptor = ImageDescriptor::new(5, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(AlphaAssociation::Straight)
        .unwrap()
        .with_domain(SampleDomain::Encoded)
        .with_primaries(RgbPrimaries::srgb());
    let frame = ImageFrame::new(descriptor, PixelBuffer::f32(pixels).unwrap()).unwrap();

    let provenance = c1_vec(
        [
            ColorProvenance::EmbeddedIcc,
            ColorProvenance::ContainerNclx,
            ColorProvenance::Av1,
        ],
        8,
    );
    let mut icc = c1_vec([0x2au8; 4093], 4093);
    if icc_spare {
        icc.reserve_exact(67);
    }
    let unknown = c1_vec(
        [
            UnknownColorInformation {
                color_type: *b"test",
                payload: c1_vec([7u8; 13], 32),
            },
            UnknownColorInformation {
                color_type: *b"more",
                payload: c1_vec([8u8; 13], 64),
            },
        ],
        5,
    );
    let colors = ColorInformationSet::from_owned_parts(
        provenance,
        Some(icc),
        Some(IccColorType::Prof),
        Some(NclxColorInformation::new(1, 8, 0, true)),
        Some(Av1ColorInformation::new(1, 8, 0, true)),
        unknown,
    );
    let mut metadata = FrameMetadata::new(colors);
    metadata.set_coded_geometry(c1_vec(
        [
            GeometryOperation::Rotate(Rotation::Degrees90),
            GeometryOperation::MirrorHorizontal,
        ],
        6,
    ));
    metadata.set_render_geometry(c1_vec([GeometryOperation::MirrorVertical], 4));
    metadata.set_pixel_information(Some(
        PixelInformation::new(
            c1_vec([32u8; 4], 12),
            Some(c1_vec(
                (0..4).map(|id| PixelChannelInformation::new(id, 0, None)),
                8,
            )),
        )
        .unwrap(),
    ));
    metadata.set_coded_dimensions(Some((5, 1)));
    metadata.set_render_dimensions(Some((5, 1)));
    metadata.set_av1_description(Some(Av1Description::new(true).with_flags(
        Some(false),
        Some(true),
        Some(false),
        Some(false),
        None,
    )));
    frame
        .with_metadata(metadata)
        .with_timing(FrameTiming::new(1000, 21, 33).unwrap())
}

/// A tightly packed counterpart to `c1_rich_source`.  C2 tests need the
/// source capacities to be the logical lengths so that every capacity delta
/// is caused by the candidate being admitted, rather than by fixture slack.
pub(crate) fn c2_packed_rich_source() -> ImageFrame {
    let mut descriptors = c1_vec(std::iter::empty(), 4);
    let mut pixels = c1_vec(std::iter::empty(), 4);
    for role in [
        ChannelRole::Red,
        ChannelRole::Green,
        ChannelRole::Blue,
        ChannelRole::Alpha,
    ] {
        let layout = PlaneLayout::new(5, 1, 5, 1, c1_vec([0usize], 1), Subsampling::FULL).unwrap();
        let roles = c1_vec([role], 1);
        descriptors.push(PlaneDescriptor::new(layout.clone(), roles, 32).unwrap());
        pixels.push(Plane::new(layout, c1_vec([0.0f32, 0.1, 0.25, 0.5, 1.0], 5)).unwrap());
    }

    let descriptor = ImageDescriptor::new(5, 1, ChannelModel::RGB, descriptors)
        .unwrap()
        .with_alpha(AlphaAssociation::Straight)
        .unwrap()
        .with_domain(SampleDomain::Encoded)
        .with_primaries(RgbPrimaries::srgb());
    let frame = ImageFrame::new(descriptor, PixelBuffer::f32(pixels).unwrap()).unwrap();

    let mut metadata = FrameMetadata::new(ColorInformationSet::from_owned_parts(
        c1_vec(
            [
                ColorProvenance::EmbeddedIcc,
                ColorProvenance::ContainerNclx,
                ColorProvenance::Av1,
            ],
            3,
        ),
        Some(c1_vec([0x2au8; 4093], 4093)),
        Some(IccColorType::Prof),
        Some(NclxColorInformation::new(1, 8, 0, true)),
        Some(Av1ColorInformation::new(1, 8, 0, true)),
        c1_vec(
            [
                UnknownColorInformation {
                    color_type: *b"one1",
                    payload: c1_vec([7u8; 13], 13),
                },
                UnknownColorInformation {
                    color_type: *b"two2",
                    payload: c1_vec([8u8; 13], 13),
                },
            ],
            2,
        ),
    ));
    metadata.set_coded_geometry(c1_vec(
        [
            GeometryOperation::Rotate(Rotation::Degrees90),
            GeometryOperation::MirrorHorizontal,
        ],
        2,
    ));
    metadata.set_render_geometry(c1_vec([GeometryOperation::MirrorVertical], 1));
    metadata.set_pixel_information(Some(
        PixelInformation::new(
            c1_vec([32u8; 4], 4),
            Some(c1_vec(
                (0..4).map(|id| PixelChannelInformation::new(id, 0, None)),
                4,
            )),
        )
        .unwrap(),
    ));
    metadata.set_coded_dimensions(Some((5, 1)));
    metadata.set_render_dimensions(Some((5, 1)));
    metadata.set_av1_description(Some(Av1Description::new(true).with_flags(
        Some(false),
        Some(true),
        Some(false),
        Some(false),
        None,
    )));
    frame
        .with_metadata(metadata)
        .with_timing(FrameTiming::new(1000, 21, 33).unwrap())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct C1RichInventory {
    pub(crate) source_live: usize,
    pub(crate) requested_metadata: usize,
    pub(crate) requested_output: usize,
}

fn checked_add(total: usize, value: usize) -> usize {
    total.checked_add(value).unwrap()
}

fn checked_mul(left: usize, right: usize) -> usize {
    left.checked_mul(right).unwrap()
}

fn checked_sub(left: usize, right: usize) -> usize {
    left.checked_sub(right).unwrap()
}

/// Independently derives C1 S/M/Q from public owner sizes and actual source
/// capacities; it intentionally does not inspect OutputOwnershipPlan totals.
pub(crate) fn c1_rich_inventory(source: &ImageFrame) -> C1RichInventory {
    let descriptor = source.descriptor();
    let mut source_live = checked_mul(
        descriptor.planes_capacity_for_test(),
        size_of::<PlaneDescriptor>(),
    );
    for plane in descriptor.planes() {
        source_live = checked_add(
            source_live,
            checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        source_live = checked_add(
            source_live,
            checked_mul(plane.roles_capacity_for_test(), size_of::<ChannelRole>()),
        );
    }
    let pixels = source.pixels().f32_planes().unwrap();
    source_live = checked_add(
        source_live,
        checked_mul(
            source.pixels().f32_planes_capacity_for_test().unwrap(),
            size_of::<Plane<f32>>(),
        ),
    );
    for plane in pixels {
        source_live = checked_add(
            source_live,
            checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        source_live = checked_add(
            source_live,
            checked_mul(plane.sample_capacity(), size_of::<f32>()),
        );
    }

    let metadata = source.metadata();
    let color = metadata.source_color();
    source_live = checked_add(
        source_live,
        checked_mul(
            color.provenance_capacity_for_test(),
            size_of::<ColorProvenance>(),
        ),
    );
    source_live = checked_add(source_live, color.icc_profile_capacity());
    source_live = checked_add(
        source_live,
        checked_mul(
            color.unknown_colr_capacity_for_test(),
            size_of::<UnknownColorInformation>(),
        ),
    );
    for item in color.unknown_colr() {
        source_live = checked_add(source_live, item.payload.capacity());
    }
    if color.nclx().is_some() {
        source_live = checked_add(source_live, 8);
    }
    if color.av1().is_some() {
        source_live = checked_add(source_live, size_of::<Av1ColorInformation>());
    }
    source_live = checked_add(
        source_live,
        checked_mul(
            metadata.coded_geometry_capacity_for_test(),
            size_of::<GeometryOperation>(),
        ),
    );
    source_live = checked_add(
        source_live,
        checked_mul(
            metadata.render_geometry_capacity_for_test(),
            size_of::<GeometryOperation>(),
        ),
    );
    if let Some(info) = metadata.pixel_information() {
        source_live = checked_add(source_live, info.bits_capacity_for_test());
        if info.extended_channels().is_some() {
            source_live = checked_add(
                source_live,
                checked_mul(
                    info.extended_capacity_for_test().unwrap(),
                    size_of::<PixelChannelInformation>(),
                ),
            );
        }
    }
    for dimensions in [metadata.coded_dimensions(), metadata.render_dimensions()] {
        if dimensions.is_some() {
            source_live = checked_add(source_live, 8);
        }
    }

    let mut requested_metadata =
        checked_mul(color.provenance().len(), size_of::<ColorProvenance>());
    requested_metadata = checked_add(
        requested_metadata,
        color.icc_profile().map_or(0, <[u8]>::len),
    );
    requested_metadata = checked_add(
        requested_metadata,
        checked_mul(
            color.unknown_colr().len(),
            size_of::<UnknownColorInformation>(),
        ),
    );
    for item in color.unknown_colr() {
        requested_metadata = checked_add(requested_metadata, item.payload.len());
    }
    let geometry_count = checked_add(
        metadata.coded_geometry().len(),
        metadata.render_geometry().len(),
    );
    requested_metadata = checked_add(
        requested_metadata,
        checked_mul(geometry_count, size_of::<GeometryOperation>()),
    );
    if let Some(info) = metadata.pixel_information() {
        requested_metadata = checked_add(requested_metadata, info.bits_per_channel().len());
        if let Some(channels) = info.extended_channels() {
            requested_metadata = checked_add(
                requested_metadata,
                checked_mul(channels.len(), size_of::<PixelChannelInformation>()),
            );
        }
    }
    if color.nclx().is_some() {
        requested_metadata = checked_add(requested_metadata, 8);
    }
    if color.av1().is_some() {
        requested_metadata = checked_add(requested_metadata, size_of::<Av1ColorInformation>());
    }
    for dimensions in [metadata.coded_dimensions(), metadata.render_dimensions()] {
        if dimensions.is_some() {
            requested_metadata = checked_add(requested_metadata, 8);
        }
    }

    let planes = descriptor.planes().len();
    let pixel_count = checked_mul(
        usize::try_from(descriptor.width()).unwrap(),
        usize::try_from(descriptor.height()).unwrap(),
    );
    let per_plane = checked_add(
        checked_mul(pixel_count, size_of::<f32>()),
        checked_add(
            size_of::<PlaneDescriptor>(),
            checked_add(
                size_of::<Plane<f32>>(),
                checked_add(checked_mul(2, size_of::<usize>()), size_of::<ChannelRole>()),
            ),
        ),
    );
    let requested_output = checked_add(checked_mul(planes, per_plane), requested_metadata);
    C1RichInventory {
        source_live,
        requested_metadata,
        requested_output,
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct C2RichInventory {
    pub(crate) source_live: usize,
    pub(crate) requested_metadata: usize,
    pub(crate) requested_output: usize,
    pub(crate) active_color: usize,
    pub(crate) source_metadata: usize,
}

fn c2_color_bytes(color: &ColorInformationSet, capacity: bool) -> usize {
    let mut total = if capacity {
        color
            .provenance_capacity_for_test()
            .checked_mul(size_of::<ColorProvenance>())
            .unwrap()
    } else {
        color
            .provenance()
            .len()
            .checked_mul(size_of::<ColorProvenance>())
            .unwrap()
    };
    total = checked_add(
        total,
        color.icc_profile().map_or(0, |profile| {
            if capacity {
                color.icc_profile_capacity()
            } else {
                profile.len()
            }
        }),
    );
    if color.nclx().is_some() {
        total = checked_add(total, 8);
    }
    if color.av1().is_some() {
        total = checked_add(total, size_of::<Av1ColorInformation>());
    }
    let unknown_count = if capacity {
        color.unknown_colr_capacity_for_test()
    } else {
        color.unknown_colr().len()
    };
    total = checked_add(
        total,
        unknown_count
            .checked_mul(size_of::<UnknownColorInformation>())
            .unwrap(),
    );
    for item in color.unknown_colr() {
        total = checked_add(
            total,
            if capacity {
                item.payload.capacity()
            } else {
                item.payload.len()
            },
        );
    }
    total
}

#[derive(Clone)]
struct C2VecSnapshot<T> {
    pointer: usize,
    length: usize,
    capacity: usize,
    values: Vec<T>,
}

fn c2_vec_snapshot<T: Clone>(values: &[T], capacity: usize) -> C2VecSnapshot<T> {
    C2VecSnapshot {
        pointer: values.as_ptr() as usize,
        length: values.len(),
        capacity,
        values: values.to_vec(),
    }
}

pub(crate) struct C2SourceSnapshot {
    descriptor_outer: C2VecSnapshot<PlaneDescriptor>,
    descriptors: Vec<C2DescriptorSnapshot>,
    pixel_outer: C2VecSnapshot<Plane<f32>>,
    pixels: Vec<C2PixelSnapshot>,
    provenance: C2VecSnapshot<ColorProvenance>,
    icc: Option<C2VecSnapshot<u8>>,
    unknown: C2VecSnapshot<UnknownColorInformation>,
    unknown_payloads: Vec<C2VecSnapshot<u8>>,
    pixi_bits: Option<C2VecSnapshot<u8>>,
    pixi_extended: Option<C2VecSnapshot<PixelChannelInformation>>,
    coded_geometry: C2VecSnapshot<GeometryOperation>,
    render_geometry: C2VecSnapshot<GeometryOperation>,
    metadata: FrameMetadata,
    timing: Option<FrameTiming>,
}

#[derive(Clone)]
struct C2DescriptorSnapshot {
    layout: C2LayoutSnapshot,
    roles: C2VecSnapshot<ChannelRole>,
    meaningful_bits: u8,
}

#[derive(Clone)]
struct C2PixelSnapshot {
    layout: C2LayoutSnapshot,
    samples: C2VecSnapshot<f32>,
}

#[derive(Clone)]
struct C2LayoutSnapshot {
    width: u32,
    height: u32,
    row_stride: usize,
    pixel_stride: usize,
    offsets: C2VecSnapshot<usize>,
    subsampling: Subsampling,
}

fn c2_layout_snapshot(layout: &PlaneLayout) -> C2LayoutSnapshot {
    C2LayoutSnapshot {
        width: layout.width(),
        height: layout.height(),
        row_stride: layout.row_stride(),
        pixel_stride: layout.pixel_stride(),
        offsets: c2_vec_snapshot(
            layout.channel_offsets(),
            layout.channel_offsets_capacity_for_test(),
        ),
        subsampling: layout.subsampling(),
    }
}

fn c2_descriptor_snapshot(descriptor: &PlaneDescriptor) -> C2DescriptorSnapshot {
    C2DescriptorSnapshot {
        layout: c2_layout_snapshot(descriptor.layout()),
        roles: c2_vec_snapshot(descriptor.roles(), descriptor.roles_capacity_for_test()),
        meaningful_bits: descriptor.meaningful_bits(),
    }
}

fn c2_pixel_snapshot(plane: &Plane<f32>) -> C2PixelSnapshot {
    C2PixelSnapshot {
        layout: c2_layout_snapshot(plane.layout()),
        samples: c2_vec_snapshot(plane.samples(), plane.sample_capacity()),
    }
}

pub(crate) fn c2_source_snapshot(source: &ImageFrame) -> C2SourceSnapshot {
    let descriptors = source.descriptor().planes();
    let pixels = source.pixels().f32_planes().unwrap();
    let pixi = source.metadata().pixel_information();
    let color = source.metadata().source_color();
    C2SourceSnapshot {
        descriptor_outer: c2_vec_snapshot(
            descriptors,
            source.descriptor().planes_capacity_for_test(),
        ),
        descriptors: descriptors.iter().map(c2_descriptor_snapshot).collect(),
        pixel_outer: c2_vec_snapshot(
            pixels,
            source.pixels().f32_planes_capacity_for_test().unwrap(),
        ),
        pixels: pixels.iter().map(c2_pixel_snapshot).collect(),
        provenance: c2_vec_snapshot(color.provenance(), color.provenance_capacity_for_test()),
        icc: color
            .icc_profile()
            .map(|profile| c2_vec_snapshot(profile, color.icc_profile_capacity())),
        unknown: c2_vec_snapshot(color.unknown_colr(), color.unknown_colr_capacity_for_test()),
        unknown_payloads: color
            .unknown_colr()
            .iter()
            .map(|item| c2_vec_snapshot(&item.payload, item.payload.capacity()))
            .collect(),
        pixi_bits: pixi
            .map(|info| c2_vec_snapshot(info.bits_per_channel(), info.bits_capacity_for_test())),
        pixi_extended: pixi.and_then(|info| {
            info.extended_channels().map(|channels| {
                c2_vec_snapshot(channels, info.extended_capacity_for_test().unwrap_or(0))
            })
        }),
        coded_geometry: c2_vec_snapshot(
            source.metadata().coded_geometry(),
            source.metadata().coded_geometry_capacity_for_test(),
        ),
        render_geometry: c2_vec_snapshot(
            source.metadata().render_geometry(),
            source.metadata().render_geometry_capacity_for_test(),
        ),
        metadata: source.metadata().clone(),
        timing: source.timing(),
    }
}

pub(crate) fn c2_assert_source_unchanged(source: &ImageFrame, snapshot: &C2SourceSnapshot) {
    let descriptors = source.descriptor().planes();
    let pixels = source.pixels().f32_planes().unwrap();
    assert_eq!(
        c2_vec_snapshot(descriptors, source.descriptor().planes_capacity_for_test()).pointer,
        snapshot.descriptor_outer.pointer
    );
    assert_eq!(descriptors.len(), snapshot.descriptor_outer.length);
    assert_eq!(
        source.descriptor().planes_capacity_for_test(),
        snapshot.descriptor_outer.capacity
    );
    assert_eq!(descriptors, snapshot.descriptor_outer.values);
    assert_eq!(descriptors.len(), snapshot.descriptors.len());
    for (descriptor, expected) in descriptors.iter().zip(&snapshot.descriptors) {
        let layout = c2_layout_snapshot(descriptor.layout());
        assert_eq!(layout.width, expected.layout.width);
        assert_eq!(layout.height, expected.layout.height);
        assert_eq!(layout.row_stride, expected.layout.row_stride);
        assert_eq!(layout.pixel_stride, expected.layout.pixel_stride);
        assert_eq!(layout.subsampling, expected.layout.subsampling);
        assert_eq!(layout.offsets.pointer, expected.layout.offsets.pointer);
        assert_eq!(layout.offsets.length, expected.layout.offsets.length);
        assert_eq!(layout.offsets.capacity, expected.layout.offsets.capacity);
        assert_eq!(layout.offsets.values, expected.layout.offsets.values);
        let roles = c2_vec_snapshot(descriptor.roles(), descriptor.roles_capacity_for_test());
        assert_eq!(roles.pointer, expected.roles.pointer);
        assert_eq!(roles.length, expected.roles.length);
        assert_eq!(roles.capacity, expected.roles.capacity);
        assert_eq!(roles.values, expected.roles.values);
        assert_eq!(descriptor.meaningful_bits(), expected.meaningful_bits);
    }
    assert_eq!(pixels.len(), snapshot.pixel_outer.length);
    assert_eq!(
        source.pixels().f32_planes_capacity_for_test().unwrap(),
        snapshot.pixel_outer.capacity
    );
    assert_eq!(pixels.as_ptr() as usize, snapshot.pixel_outer.pointer);
    assert_eq!(pixels, snapshot.pixel_outer.values);
    assert_eq!(pixels.len(), snapshot.pixels.len());
    for (plane, expected) in pixels.iter().zip(&snapshot.pixels) {
        let layout = c2_layout_snapshot(plane.layout());
        assert_eq!(layout.width, expected.layout.width);
        assert_eq!(layout.height, expected.layout.height);
        assert_eq!(layout.row_stride, expected.layout.row_stride);
        assert_eq!(layout.pixel_stride, expected.layout.pixel_stride);
        assert_eq!(layout.subsampling, expected.layout.subsampling);
        assert_eq!(layout.offsets.pointer, expected.layout.offsets.pointer);
        assert_eq!(layout.offsets.length, expected.layout.offsets.length);
        assert_eq!(layout.offsets.capacity, expected.layout.offsets.capacity);
        assert_eq!(layout.offsets.values, expected.layout.offsets.values);
        let samples = c2_vec_snapshot(plane.samples(), plane.sample_capacity());
        assert_eq!(samples.pointer, expected.samples.pointer);
        assert_eq!(samples.length, expected.samples.length);
        assert_eq!(samples.capacity, expected.samples.capacity);
        assert_eq!(samples.values, expected.samples.values);
    }
    let color = source.metadata().source_color();
    let current = c2_vec_snapshot(color.provenance(), color.provenance_capacity_for_test());
    assert_eq!(current.pointer, snapshot.provenance.pointer);
    assert_eq!(current.length, snapshot.provenance.length);
    assert_eq!(current.capacity, snapshot.provenance.capacity);
    assert_eq!(current.values, snapshot.provenance.values);
    match (&snapshot.icc, color.icc_profile()) {
        (None, None) => {}
        (Some(expected), Some(profile)) => {
            let current = c2_vec_snapshot(profile, color.icc_profile_capacity());
            assert_eq!(current.pointer, expected.pointer);
            assert_eq!(current.length, expected.length);
            assert_eq!(current.capacity, expected.capacity);
            assert_eq!(current.values, expected.values);
        }
        _ => panic!("source ICC presence changed"),
    }
    let current = c2_vec_snapshot(color.unknown_colr(), color.unknown_colr_capacity_for_test());
    assert_eq!(current.pointer, snapshot.unknown.pointer);
    assert_eq!(current.length, snapshot.unknown.length);
    assert_eq!(current.capacity, snapshot.unknown.capacity);
    assert_eq!(current.values, snapshot.unknown.values);
    assert_eq!(current.values.len(), snapshot.unknown_payloads.len());
    for (item, expected) in color.unknown_colr().iter().zip(&snapshot.unknown_payloads) {
        let current = c2_vec_snapshot(&item.payload, item.payload.capacity());
        assert_eq!(current.pointer, expected.pointer);
        assert_eq!(current.length, expected.length);
        assert_eq!(current.capacity, expected.capacity);
        assert_eq!(current.values, expected.values);
    }
    let pixi = source.metadata().pixel_information();
    match (&snapshot.pixi_bits, pixi) {
        (None, None) => {}
        (Some(expected), Some(info)) => {
            let current = c2_vec_snapshot(info.bits_per_channel(), info.bits_capacity_for_test());
            assert_eq!(current.pointer, expected.pointer);
            assert_eq!(current.length, expected.length);
            assert_eq!(current.capacity, expected.capacity);
            assert_eq!(current.values, expected.values);
        }
        _ => panic!("source pixel information presence changed"),
    }
    match (
        &snapshot.pixi_extended,
        pixi.and_then(PixelInformation::extended_channels),
    ) {
        (None, None) => {}
        (Some(expected), Some(channels)) => {
            let current = c2_vec_snapshot(
                channels,
                pixi.unwrap().extended_capacity_for_test().unwrap_or(0),
            );
            assert_eq!(current.pointer, expected.pointer);
            assert_eq!(current.length, expected.length);
            assert_eq!(current.capacity, expected.capacity);
            assert_eq!(current.values, expected.values);
        }
        _ => panic!("source extended pixel information presence changed"),
    }
    let current = c2_vec_snapshot(
        source.metadata().coded_geometry(),
        source.metadata().coded_geometry_capacity_for_test(),
    );
    assert_eq!(current.pointer, snapshot.coded_geometry.pointer);
    assert_eq!(current.length, snapshot.coded_geometry.length);
    assert_eq!(current.capacity, snapshot.coded_geometry.capacity);
    assert_eq!(current.values, snapshot.coded_geometry.values);
    let current = c2_vec_snapshot(
        source.metadata().render_geometry(),
        source.metadata().render_geometry_capacity_for_test(),
    );
    assert_eq!(current.pointer, snapshot.render_geometry.pointer);
    assert_eq!(current.length, snapshot.render_geometry.length);
    assert_eq!(current.capacity, snapshot.render_geometry.capacity);
    assert_eq!(current.values, snapshot.render_geometry.values);
    assert_eq!(source.metadata(), &snapshot.metadata);
    assert_eq!(source.timing(), snapshot.timing);
}

/// Independently computes the C2 source/live and requested clone/output
/// ledgers.  These values intentionally walk public test observability
/// accessors instead of reading `OutputOwnershipPlan` totals.
pub(crate) fn c2_rich_inventory(source: &ImageFrame, pixel_count: usize) -> C2RichInventory {
    let descriptor = source.descriptor();
    let active_color = c2_color_bytes(descriptor.color_information(), true);
    let mut source_live = checked_mul(
        descriptor.planes_capacity_for_test(),
        size_of::<PlaneDescriptor>(),
    );
    for plane in descriptor.planes() {
        source_live = checked_add(
            source_live,
            checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        source_live = checked_add(
            source_live,
            checked_mul(plane.roles_capacity_for_test(), size_of::<ChannelRole>()),
        );
    }
    source_live = checked_add(source_live, active_color);

    let pixels = source.pixels().f32_planes().unwrap();
    source_live = checked_add(
        source_live,
        checked_mul(
            source.pixels().f32_planes_capacity_for_test().unwrap(),
            size_of::<Plane<f32>>(),
        ),
    );
    for plane in pixels {
        source_live = checked_add(
            source_live,
            checked_mul(
                plane.layout().channel_offsets_capacity_for_test(),
                size_of::<usize>(),
            ),
        );
        source_live = checked_add(
            source_live,
            checked_mul(plane.sample_capacity(), size_of::<f32>()),
        );
    }

    let metadata = source.metadata();
    let source_color = metadata.source_color();
    let source_metadata = [
        c2_color_bytes(source_color, true),
        checked_add(
            checked_mul(
                metadata.coded_geometry_capacity_for_test(),
                size_of::<GeometryOperation>(),
            ),
            checked_mul(
                metadata.render_geometry_capacity_for_test(),
                size_of::<GeometryOperation>(),
            ),
        ),
        metadata.pixel_information().map_or(0, |info| {
            checked_add(
                info.bits_capacity_for_test(),
                checked_mul(
                    info.extended_capacity_for_test().unwrap_or(0),
                    size_of::<PixelChannelInformation>(),
                ),
            )
        }),
        checked_mul(
            [metadata.coded_dimensions(), metadata.render_dimensions()]
                .into_iter()
                .filter(|value| value.is_some())
                .count(),
            8,
        ),
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap();
    source_live = checked_add(source_live, source_metadata);

    let requested_metadata = [
        c2_color_bytes(source_color, false),
        checked_add(
            checked_mul(
                metadata.coded_geometry().len(),
                size_of::<GeometryOperation>(),
            ),
            checked_mul(
                metadata.render_geometry().len(),
                size_of::<GeometryOperation>(),
            ),
        ),
        metadata.pixel_information().map_or(0, |info| {
            checked_add(
                info.bits_per_channel().len(),
                info.extended_channels().map_or(0, |channels| {
                    checked_mul(channels.len(), size_of::<PixelChannelInformation>())
                }),
            )
        }),
        checked_mul(
            [metadata.coded_dimensions(), metadata.render_dimensions()]
                .into_iter()
                .filter(|value| value.is_some())
                .count(),
            8,
        ),
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap();
    let per_plane = [
        checked_mul(pixel_count, size_of::<f32>()),
        size_of::<PlaneDescriptor>(),
        size_of::<Plane<f32>>(),
        checked_mul(2, size_of::<usize>()),
        size_of::<ChannelRole>(),
    ]
    .into_iter()
    .try_fold(0usize, |total, value| total.checked_add(value))
    .unwrap();
    let requested_output = checked_add(checked_mul(4, per_plane), requested_metadata);
    C2RichInventory {
        source_live,
        requested_metadata,
        requested_output,
        active_color,
        source_metadata,
    }
}

#[test]
fn c2_packed_rich_source_has_no_fixture_capacity_slack() {
    let source = c2_packed_rich_source();
    let descriptor = source.descriptor();
    assert_eq!(descriptor.planes().len(), 4);
    assert_eq!(descriptor.planes_capacity_for_test(), 4);
    for plane in descriptor.planes() {
        assert_eq!(plane.layout().channel_offsets_capacity_for_test(), 1);
        assert_eq!(plane.roles_capacity_for_test(), 1);
    }
    let pixels = source.pixels().f32_planes().unwrap();
    assert_eq!(pixels.len(), 4);
    assert_eq!(source.pixels().f32_planes_capacity_for_test().unwrap(), 4);
    for plane in pixels {
        assert_eq!(plane.layout().channel_offsets_capacity_for_test(), 1);
        assert_eq!(plane.sample_capacity(), 5);
    }
    let color = descriptor.color_information();
    assert!(color.provenance().is_empty());
    assert_eq!(color.provenance_capacity_for_test(), 0);
    assert!(color.icc_profile().is_none());
    assert_eq!(color.icc_profile_capacity(), 0);
    assert!(color.unknown_colr().is_empty());
    assert_eq!(color.unknown_colr_capacity_for_test(), 0);
    let metadata = source.metadata();
    assert_eq!(metadata.source_color().provenance_capacity_for_test(), 3);
    assert_eq!(metadata.source_color().icc_profile_capacity(), 4093);
    assert_eq!(metadata.source_color().unknown_colr_capacity_for_test(), 2);
    assert_eq!(
        metadata.source_color().unknown_colr()[0].payload.capacity(),
        13
    );
    assert_eq!(
        metadata.source_color().unknown_colr()[1].payload.capacity(),
        13
    );
    assert_eq!(metadata.coded_geometry_capacity_for_test(), 2);
    assert_eq!(metadata.render_geometry_capacity_for_test(), 1);
    let pixi = metadata.pixel_information().unwrap();
    assert_eq!(pixi.bits_capacity_for_test(), 4);
    assert_eq!(pixi.extended_capacity_for_test(), Some(4));
    assert_eq!(
        source.timing().unwrap(),
        FrameTiming::new(1000, 21, 33).unwrap()
    );
}

#[derive(Clone, Copy)]
pub(crate) struct C2CapacityMaker {
    sample: bool,
    icc: bool,
    icc_source: bool,
    icc_destination: bool,
    provenance: bool,
    unknown: bool,
    empty_geometry: bool,
}

impl C2CapacityMaker {
    pub(crate) const fn none() -> Self {
        Self {
            sample: false,
            icc: false,
            icc_source: false,
            icc_destination: false,
            provenance: false,
            unknown: false,
            empty_geometry: false,
        }
    }
    const fn all() -> Self {
        Self {
            sample: true,
            icc: true,
            icc_source: true,
            icc_destination: true,
            provenance: true,
            unknown: true,
            empty_geometry: true,
        }
    }
}

impl CandidateMaker for C2CapacityMaker {
    fn make<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let target = match key {
            OwnerKey::Sample(0) if self.sample && count == 5 => 8,
            OwnerKey::IccProfile if self.icc && count == 4093 => 4160,
            OwnerKey::IccSourceProfile if self.icc_source && count == 4 => 8,
            OwnerKey::IccDestinationProfile if self.icc_destination && count == 5 => 9,
            OwnerKey::Provenance if self.provenance && count == 3 => 5,
            OwnerKey::UnknownPayload(1) if self.unknown && count == 13 => 4160,
            OwnerKey::CodedGeometry if self.empty_geometry && count == 0 => 1,
            _ => count,
        };
        let mut values = Vec::new();
        values.try_reserve_exact(count).map_err(|error| {
            ProcessingError::Allocation(format!("C2 candidate allocation failed: {error}"))
        })?;
        if target > values.capacity() {
            values
                // The vector is intentionally still empty here.  Reserve
                // against its length, not its current capacity; passing the
                // capacity delta would permit the existing allocation to
                // satisfy the request without creating the controlled slack.
                .try_reserve_exact(target)
                .map_err(|error| {
                    ProcessingError::Allocation(format!("C2 candidate growth failed: {error}"))
                })?;
        }
        if values.capacity() != target {
            return Err(ProcessingError::ResourceLimit(format!(
                "C2 candidate capacity is not deterministic: {key:?} count={count} target={target} actual={}",
                values.capacity()
            )));
        }
        Ok(values)
    }
}

pub(crate) fn c2_consume_prefix_to_provenance(
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut C2CapacityMaker,
) -> Result<(), ProcessingError> {
    for plane in 0..4 {
        admitted.fresh_with::<f32, _>(OwnerKey::Sample(plane), 5, maker)?;
    }
    admitted.fresh_with::<PlaneDescriptor, _>(OwnerKey::DescriptorOuter, 4, maker)?;
    admitted.fresh_with::<Plane<f32>, _>(OwnerKey::PixelOuter, 4, maker)?;
    for plane in 0..4 {
        admitted.fresh_with::<usize, _>(OwnerKey::DescriptorLayout { plane }, 1, maker)?;
        admitted.fresh_with::<ChannelRole, _>(OwnerKey::Role { plane }, 1, maker)?;
        admitted.fresh_with::<usize, _>(OwnerKey::PixelLayout { plane }, 1, maker)?;
    }
    admitted.fresh_metadata::<GeometryOperation, _>(OwnerKey::CodedGeometry, 2, maker)?;
    admitted.fresh_metadata::<GeometryOperation, _>(OwnerKey::RenderGeometry, 1, maker)?;
    admitted.fresh_metadata::<u8, _>(OwnerKey::PixiBits, 4, maker)?;
    admitted.fresh_metadata::<PixelChannelInformation, _>(OwnerKey::PixiExtended, 4, maker)?;
    Ok(())
}

fn c2_consume_all(
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut C2CapacityMaker,
    sample_count: usize,
) -> Result<(), ProcessingError> {
    c2_consume_all_with_sample_limit(admitted, maker, sample_count, None)
}

fn c2_consume_all_with_sample_limit(
    admitted: &mut AdmittedOutputPlan<'_>,
    maker: &mut C2CapacityMaker,
    sample_count: usize,
    sample_limit: Option<usize>,
) -> Result<(), ProcessingError> {
    for plane in 0..4 {
        if plane == 0 {
            if let Some(limit) = sample_limit {
                admitted.fresh_with_limit::<f32, _>(
                    OwnerKey::Sample(plane),
                    sample_count,
                    limit,
                    maker,
                )?;
                continue;
            }
        }
        admitted.fresh_with::<f32, _>(OwnerKey::Sample(plane), sample_count, maker)?;
    }
    admitted.fresh_with::<PlaneDescriptor, _>(OwnerKey::DescriptorOuter, 4, maker)?;
    admitted.fresh_with::<Plane<f32>, _>(OwnerKey::PixelOuter, 4, maker)?;
    for plane in 0..4 {
        admitted.fresh_with::<usize, _>(OwnerKey::DescriptorLayout { plane }, 1, maker)?;
        admitted.fresh_with::<ChannelRole, _>(OwnerKey::Role { plane }, 1, maker)?;
        admitted.fresh_with::<usize, _>(OwnerKey::PixelLayout { plane }, 1, maker)?;
    }
    admitted.fresh_metadata::<GeometryOperation, _>(OwnerKey::CodedGeometry, 2, maker)?;
    admitted.fresh_metadata::<GeometryOperation, _>(OwnerKey::RenderGeometry, 1, maker)?;
    admitted.fresh_metadata::<u8, _>(OwnerKey::PixiBits, 4, maker)?;
    admitted.fresh_metadata::<PixelChannelInformation, _>(OwnerKey::PixiExtended, 4, maker)?;
    admitted.fresh_metadata::<ColorProvenance, _>(OwnerKey::Provenance, 3, maker)?;
    admitted.fresh_metadata::<u8, _>(OwnerKey::IccProfile, 4093, maker)?;
    admitted.fresh_metadata::<UnknownColorInformation, _>(OwnerKey::UnknownOuter, 2, maker)?;
    admitted.fresh_metadata::<u8, _>(OwnerKey::UnknownPayload(0), 13, maker)?;
    admitted.fresh_metadata::<u8, _>(OwnerKey::UnknownPayload(1), 13, maker)?;
    admitted.commit_metadata_event(OwnerKey::SourceNclx, 8)?;
    admitted.commit_metadata_event(OwnerKey::SourceAv1, size_of::<Av1ColorInformation>())?;
    admitted.commit_metadata_event(OwnerKey::CodedDimensions, 8)?;
    admitted.commit_metadata_event(OwnerKey::RenderDimensions, 8)?;
    Ok(())
}

#[test]
fn c2_actual_capacity_deltas_are_charged_from_independent_prefixes() {
    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let sample_delta = 12usize;
    let icc_delta = 67usize;
    let provenance_delta = checked_mul(2, size_of::<ColorProvenance>());
    let unknown_delta = 4160usize.checked_sub(13).unwrap();
    let total_delta = [sample_delta, icc_delta, provenance_delta, unknown_delta]
        .into_iter()
        .try_fold(0usize, |total, value| total.checked_add(value))
        .unwrap();
    let exact = c2_limits(
        std::cmp::max(
            inventory.source_live,
            checked_add(inventory.requested_output, total_delta),
        ),
        checked_add(
            checked_add(inventory.source_live, inventory.requested_output),
            total_delta,
        ),
        std::cmp::max(
            checked_add(inventory.source_metadata, inventory.active_color),
            checked_add(
                inventory.requested_metadata,
                [icc_delta, provenance_delta, unknown_delta]
                    .into_iter()
                    .try_fold(0usize, |total, value| total.checked_add(value))
                    .unwrap(),
            ),
        ),
        checked_mul(32, size_of::<f32>()),
        4160,
    );
    assert!(source.validate_with_limits(&exact).is_ok());
    let output = OutputOwnershipPlan::inspect(&source, 4, 5, &exact).unwrap();
    let ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &exact).unwrap();
    let mut admitted = output.admit(ledger, source.metadata()).unwrap();
    let mut maker = C2CapacityMaker::all();
    admitted
        .materialize_with(|admitted| c2_consume_all(admitted, &mut maker, 5))
        .unwrap();
    let snapshot = admitted.c1_accounting_snapshot();
    assert_eq!(
        snapshot.0,
        checked_add(inventory.requested_output, total_delta)
    );
    assert_eq!(
        snapshot.1,
        checked_add(
            inventory.requested_metadata,
            [icc_delta, provenance_delta, unknown_delta]
                .into_iter()
                .try_fold(0usize, |total, value| total.checked_add(value))
                .unwrap(),
        ),
    );
    assert_eq!(
        snapshot.2,
        checked_add(
            checked_add(inventory.source_live, inventory.requested_output),
            total_delta,
        )
    );
    assert_eq!(snapshot.3, (0, 0));
    assert_eq!(snapshot.4, 31);
}

#[test]
fn c2_sample_capacity_uses_frame_live_tight_limits_and_plane_cap() {
    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let delta = 12;
    let ample = c2_boundary_limits(
        inventory,
        inventory.requested_output.checked_add(delta).unwrap(),
        inventory
            .source_live
            .checked_add(inventory.requested_output)
            .and_then(|value| value.checked_add(delta))
            .unwrap(),
        inventory.requested_metadata.checked_add(1).unwrap(),
        32,
        4160,
    );
    assert!(source.validate_with_limits(&ample).is_ok());
    let output = OutputOwnershipPlan::inspect(&source, 4, 5, &ample).unwrap();
    let ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &ample).unwrap();
    let mut admitted = output.admit(ledger, source.metadata()).unwrap();
    let before = admitted.c1_accounting_snapshot();
    let mut maker = C2CapacityMaker {
        sample: true,
        ..C2CapacityMaker::none()
    };
    let sample = admitted
        .fresh_with_limit::<f32, _>(OwnerKey::Sample(0), 5, 32, &mut maker)
        .unwrap();
    assert!(sample.is_empty());
    assert_eq!(sample.capacity(), 8);
    assert_eq!(
        admitted.c1_accounting_snapshot(),
        (
            32,
            0,
            inventory.source_live.checked_add(32).unwrap(),
            (
                inventory.requested_output.checked_sub(20).unwrap(),
                inventory.requested_metadata,
            ),
            1,
        )
    );
    assert_eq!(
        before.3,
        (inventory.requested_output, inventory.requested_metadata)
    );

    for (frame_limit, live_limit, max_plane, expected) in [
        (
            inventory.requested_output.checked_add(11).unwrap(),
            inventory
                .source_live
                .checked_add(inventory.requested_output)
                .and_then(|value| value.checked_add(delta))
                .unwrap(),
            32,
            false,
        ),
        (
            inventory.requested_output.checked_add(delta).unwrap(),
            inventory
                .source_live
                .checked_add(inventory.requested_output)
                .and_then(|value| value.checked_add(11))
                .unwrap(),
            32,
            false,
        ),
        (
            inventory.requested_output.checked_add(delta).unwrap(),
            inventory
                .source_live
                .checked_add(inventory.requested_output)
                .and_then(|value| value.checked_add(delta))
                .unwrap(),
            31,
            false,
        ),
    ] {
        let source_state = c2_source_snapshot(&source);
        let limits = c2_boundary_limits(
            inventory,
            frame_limit,
            live_limit,
            inventory.requested_metadata.checked_add(delta).unwrap(),
            max_plane,
            4160,
        );
        assert!(source.validate_with_limits(&limits).is_ok());
        let output = OutputOwnershipPlan::inspect(&source, 4, 5, &limits).unwrap();
        let ledger =
            ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &limits).unwrap();
        let mut admitted = output.admit(ledger, source.metadata()).unwrap();
        let before = admitted.c1_accounting_snapshot();
        let mut maker = C2CapacityMaker {
            sample: true,
            ..C2CapacityMaker::none()
        };
        let result = admitted.materialize_with(|admitted| {
            c2_consume_all_with_sample_limit(admitted, &mut maker, 5, Some(max_plane))
        });
        assert_eq!(result.is_ok(), expected);
        if !expected {
            assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
            assert_eq!(admitted.c1_accounting_snapshot(), before);
            c2_assert_source_unchanged(&source, &source_state);
        }
    }
}

#[test]
fn c2_icc_and_metadata_deltas_respect_owner_limits_and_restore_prefix() {
    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let delta = 67;
    for (frame_extra, live_extra, metadata_extra, icc_limit, expected_ok) in [
        (delta, delta, delta, 4160, true),
        (66, delta, delta, 4160, false),
        (delta, 66, delta, 4160, false),
        (delta, delta, 66, 4160, false),
        (delta, delta, delta, 4159, false),
    ] {
        let source_state = c2_source_snapshot(&source);
        let frame_limit = inventory.requested_output.checked_add(frame_extra).unwrap();
        let live_limit = inventory
            .source_live
            .checked_add(inventory.requested_output)
            .and_then(|value| value.checked_add(live_extra))
            .unwrap();
        let metadata_limit = inventory
            .requested_metadata
            .checked_add(metadata_extra)
            .unwrap();
        let limits = c2_boundary_limits(
            inventory,
            frame_limit,
            live_limit,
            metadata_limit,
            32,
            icc_limit,
        );
        assert!(
            source.validate_with_limits(&limits).is_ok(),
            "source validation failed: {:?}",
            source.validate_with_limits(&limits)
        );
        let output = OutputOwnershipPlan::inspect(&source, 4, 5, &limits).unwrap();
        let ledger =
            ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &limits).unwrap();
        let mut admitted = output.admit(ledger, source.metadata()).unwrap();
        let before = admitted.c1_accounting_snapshot();
        let mut maker = C2CapacityMaker {
            icc: true,
            ..C2CapacityMaker::none()
        };
        let result = admitted.materialize_with(|admitted| c2_consume_all(admitted, &mut maker, 5));
        assert_eq!(result.is_ok(), expected_ok);
        if expected_ok {
            let expected_frame = inventory.requested_output.checked_add(delta).unwrap();
            let expected_metadata = inventory.requested_metadata.checked_add(delta).unwrap();
            assert_eq!(admitted.c1_accounting_snapshot().0, expected_frame);
            assert_eq!(admitted.c1_accounting_snapshot().1, expected_metadata);
            assert_eq!(
                admitted.c1_accounting_snapshot().2,
                inventory
                    .source_live
                    .checked_add(inventory.requested_output)
                    .and_then(|value| value.checked_add(delta))
                    .unwrap()
            );
            assert_eq!(admitted.c1_accounting_snapshot().3, (0, 0));
            assert_eq!(admitted.c1_accounting_snapshot().4, 31);
        } else {
            assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
            assert_eq!(admitted.c1_accounting_snapshot(), before);
            c2_assert_source_unchanged(&source, &source_state);
        }
    }
}

#[test]
fn c2_directional_icc_owner_capacity_is_limited_and_retry_restores() {
    // Keep this fixture deliberately small: the test is about the keyed ICC
    // owner admission seam, not about the richer metadata inventory above.
    for (target, count) in [
        (OwnerKey::IccSourceProfile, 4usize),
        (OwnerKey::IccDestinationProfile, 5usize),
    ] {
        let source = source();
        let limits = c2_limits(
            1 << 20,
            1 << 20,
            1 << 20,
            1 << 20,
            5,
        );
        let output = OutputOwnershipPlan::inspect_with_icc_profiles(
            &source,
            3,
            1,
            4,
            5,
            true,
            &limits,
        )
        .unwrap();
        let ledger = ConstructionLedger::new_with_ownership(0, 0, 0, &limits).unwrap();
        let mut admitted = output.admit(ledger, source.metadata()).unwrap();

        for plane in 0..3 {
            admitted
                .fresh_with::<f32, _>(
                    OwnerKey::Sample(plane),
                    1,
                    &mut C2CapacityMaker::none(),
                )
                .unwrap();
        }
        admitted
            .fresh_with::<PlaneDescriptor, _>(
                OwnerKey::DescriptorOuter,
                3,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();
        admitted
            .fresh_with::<Plane<f32>, _>(
                OwnerKey::PixelOuter,
                3,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();
        for plane in 0..3 {
            admitted
                .fresh_with::<usize, _>(
                    OwnerKey::DescriptorLayout { plane },
                    1,
                    &mut C2CapacityMaker::none(),
                )
                .unwrap();
            admitted
                .fresh_with::<ChannelRole, _>(
                    OwnerKey::Role { plane },
                    1,
                    &mut C2CapacityMaker::none(),
                )
                .unwrap();
            admitted
                .fresh_with::<usize, _>(
                    OwnerKey::PixelLayout { plane },
                    1,
                    &mut C2CapacityMaker::none(),
                )
                .unwrap();
        }

        admitted
            .fresh_metadata::<GeometryOperation, _>(
                OwnerKey::CodedGeometry,
                0,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();
        admitted
            .fresh_metadata::<GeometryOperation, _>(
                OwnerKey::RenderGeometry,
                0,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();
        admitted
            .fresh_metadata::<ColorProvenance, _>(
                OwnerKey::Provenance,
                1,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();
        admitted
            .fresh_metadata::<UnknownColorInformation, _>(
                OwnerKey::UnknownOuter,
                0,
                &mut C2CapacityMaker::none(),
            )
            .unwrap();

        let before = admitted.c1_accounting_snapshot();
        let mut oversized = C2CapacityMaker {
            icc_source: target == OwnerKey::IccSourceProfile,
            icc_destination: target == OwnerKey::IccDestinationProfile,
            ..C2CapacityMaker::none()
        };
        let failed = admitted.materialize_with(|admitted| {
            admitted.fresh_metadata::<u8, _>(target, count, &mut oversized)?;
            Ok(())
        });
        assert!(matches!(failed, Err(ProcessingError::ResourceLimit(_))));
        assert_eq!(admitted.c1_accounting_snapshot(), before);

        // The failed oversized candidate must have been dropped and the same
        // owner cursor/ledger reservation must be usable for a retry.
        let mut ordinary = C2CapacityMaker::none();
        let retried = admitted.materialize_with(|admitted| {
            admitted.fresh_metadata::<u8, _>(
                OwnerKey::IccSourceProfile,
                4,
                &mut ordinary,
            )?;
            admitted.fresh_metadata::<u8, _>(
                OwnerKey::IccDestinationProfile,
                5,
                &mut ordinary,
            )?;
            admitted.fresh_metadata::<ColorProvenance, _>(
                OwnerKey::IccDestinationProvenance,
                1,
                &mut ordinary,
            )?;
            Ok(())
        });
        assert!(retried.is_ok(), "retry failed for {target:?}: {retried:?}");
        assert_eq!(admitted.pending_bytes(), (0, 0));
    }
}

#[test]
fn c2_provenance_surplus_charges_all_actual_counters() {
    let source = c2_packed_rich_source();
    let inventory = c2_rich_inventory(&source, 5);
    let generous = c2_limits(1 << 20, 1 << 20, 1 << 20, 1 << 20, 1 << 20);
    let output = OutputOwnershipPlan::inspect(&source, 4, 5, &generous).unwrap();
    let ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &generous).unwrap();
    let mut admitted = output.admit(ledger, source.metadata()).unwrap();
    let mut ordinary = C2CapacityMaker::none();
    c2_consume_prefix_to_provenance(&mut admitted, &mut ordinary).unwrap();
    let before = admitted.c1_accounting_snapshot();
    let mut maker = C2CapacityMaker {
        provenance: true,
        ..C2CapacityMaker::none()
    };
    let candidate = admitted
        .fresh_metadata::<ColorProvenance, _>(OwnerKey::Provenance, 3, &mut maker)
        .unwrap();
    assert!(candidate.is_empty());
    assert_eq!(candidate.capacity(), 5);
    let requested = checked_mul(3, size_of::<ColorProvenance>());
    let surplus = checked_mul(2, size_of::<ColorProvenance>());
    let actual = checked_add(requested, surplus);
    let after = admitted.c1_accounting_snapshot();
    assert_eq!(after.0, checked_add(before.0, actual));
    assert_eq!(after.1, checked_add(before.1, actual));
    assert_eq!(after.2, checked_add(before.2, actual));
    assert_eq!(
        after.3,
        (
            checked_sub(before.3.0, checked_mul(3, size_of::<ColorProvenance>())),
            checked_sub(before.3.1, checked_mul(3, size_of::<ColorProvenance>())),
        )
    );
    assert_eq!(after.4, checked_add(before.4, 1));
}

#[test]
fn c2_unknown_payload_capacity_is_not_limited_by_max_icc() {
    let source = c2_packed_rich_source();
    let source_state = c2_source_snapshot(&source);
    let inventory = c2_rich_inventory(&source, 5);
    let unknown_delta = 4160usize.checked_sub(13).unwrap();
    let limits = c2_limits(
        std::cmp::max(
            inventory.source_live,
            checked_add(inventory.requested_output, unknown_delta),
        ),
        checked_add(
            checked_add(inventory.source_live, inventory.requested_output),
            unknown_delta,
        ),
        std::cmp::max(
            checked_add(inventory.source_metadata, inventory.active_color),
            checked_add(inventory.requested_metadata, unknown_delta),
        ),
        32,
        4093,
    );
    assert!(source.validate_with_limits(&limits).is_ok());
    let output = OutputOwnershipPlan::inspect(&source, 4, 5, &limits).unwrap();
    let ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &limits).unwrap();
    let mut admitted = output.admit(ledger, source.metadata()).unwrap();
    let mut maker = C2CapacityMaker {
        unknown: true,
        ..C2CapacityMaker::none()
    };
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
    let before = admitted.c1_accounting_snapshot();
    let payload = admitted
        .fresh_metadata::<u8, _>(OwnerKey::UnknownPayload(1), 13, &mut maker)
        .unwrap();
    assert!(payload.is_empty());
    assert_eq!(payload.capacity(), 4160);
    let after = admitted.c1_accounting_snapshot();
    assert_eq!(after.0, checked_add(before.0, 4160));
    assert_eq!(after.1, checked_add(before.1, 4160));
    assert_eq!(after.2, checked_add(before.2, 4160));
    assert_eq!(
        after.3,
        (checked_sub(before.3.0, 13), checked_sub(before.3.1, 13))
    );
    assert_eq!(after.4, checked_add(before.4, 1));
    c2_assert_source_unchanged(&source, &source_state);
}

#[test]
fn c2d_metadata_admission_has_exact_and_one_under_private_boundaries() {
    let source = c2_packed_rich_source();
    let source_state = c2_source_snapshot(&source);
    let inventory = c2_rich_inventory(&source, 5);
    let metadata = inventory.requested_metadata;

    // Public controls are deliberately separate from the private admission
    // proof.  The unchanged source fits at M, while M-1 rejects the source
    // itself before any output plan can be admitted.
    let source_fit = c2_limits(
        inventory.source_live,
        inventory.source_live,
        metadata,
        1 << 20,
        1 << 20,
    );
    assert!(source.validate_with_limits(&source_fit).is_ok());
    let source_one_under = c2_limits(
        inventory.source_live,
        inventory.source_live,
        metadata.checked_sub(1).unwrap(),
        1 << 20,
        1 << 20,
    );
    assert!(matches!(
        source.validate_with_limits(&source_one_under),
        Err(ProcessingError::ResourceLimit(_))
    ));
    c2_assert_source_unchanged(&source, &source_state);

    // Inspect once under a source-fitting plan limit, then use the same
    // already-inspected plan with independent ledgers.  No candidate maker
    // or output owner is involved in either admission boundary.
    let plan_limits = c2_limits(1 << 20, 1 << 20, metadata, 1 << 20, 1 << 20);
    let plan = OutputOwnershipPlan::inspect(&source, 4, 5, &plan_limits).unwrap();
    let exact_limits = c2_limits(
        inventory.requested_output,
        inventory
            .source_live
            .checked_add(inventory.requested_output)
            .unwrap(),
        metadata,
        1 << 20,
        1 << 20,
    );
    let exact_ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &exact_limits).unwrap();
    let admitted = plan.admit(exact_ledger, source.metadata()).unwrap();
    assert_eq!(
        admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.requested_output, metadata),
            0,
        )
    );
    c2_assert_source_unchanged(&source, &source_state);

    let one_under_limits = c2_limits(
        inventory.requested_output,
        inventory
            .source_live
            .checked_add(inventory.requested_output)
            .unwrap(),
        metadata.checked_sub(1).unwrap(),
        1 << 20,
        1 << 20,
    );
    let one_under_ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &one_under_limits)
            .unwrap();
    assert!(matches!(
        plan.admit(one_under_ledger, source.metadata()),
        Err(ProcessingError::ResourceLimit(_))
    ));
    c2_assert_source_unchanged(&source, &source_state);
}

#[test]
fn c2_empty_geometry_candidate_capacity_is_charged_and_advances_cursor() {
    let source = source();
    let generous = limits(1 << 20, 1 << 20);
    let output = OutputOwnershipPlan::inspect(&source, 3, 1, &generous).unwrap();
    let ledger = ConstructionLedger::new_with_ownership(0, 0, 0, &generous).unwrap();
    let mut admitted = output.admit(ledger, source.metadata()).unwrap();
    let mut maker = C2CapacityMaker {
        empty_geometry: true,
        ..C2CapacityMaker::none()
    };
    for plane in 0..3 {
        admitted
            .fresh_with::<f32, _>(OwnerKey::Sample(plane), 1, &mut maker)
            .unwrap();
    }
    admitted
        .fresh_with::<PlaneDescriptor, _>(OwnerKey::DescriptorOuter, 3, &mut maker)
        .unwrap();
    admitted
        .fresh_with::<Plane<f32>, _>(OwnerKey::PixelOuter, 3, &mut maker)
        .unwrap();
    for plane in 0..3 {
        admitted
            .fresh_with::<usize, _>(OwnerKey::DescriptorLayout { plane }, 1, &mut maker)
            .unwrap();
        admitted
            .fresh_with::<ChannelRole, _>(OwnerKey::Role { plane }, 1, &mut maker)
            .unwrap();
        admitted
            .fresh_with::<usize, _>(OwnerKey::PixelLayout { plane }, 1, &mut maker)
            .unwrap();
    }
    let before = admitted.c1_accounting_snapshot();
    let candidate = admitted
        .fresh_metadata::<GeometryOperation, _>(OwnerKey::CodedGeometry, 0, &mut maker)
        .unwrap();
    assert!(candidate.is_empty());
    assert_eq!(candidate.capacity(), 1);
    let after = admitted.c1_accounting_snapshot();
    let geometry_bytes = size_of::<GeometryOperation>();
    assert_eq!(after.0, checked_add(before.0, geometry_bytes));
    assert_eq!(after.1, checked_add(before.1, geometry_bytes));
    assert_eq!(after.2, checked_add(before.2, geometry_bytes));
    assert_eq!(after.3, before.3);
    assert_eq!(after.4, checked_add(before.4, 1));
}

#[test]
fn c1_rich_inventory_and_admission_snapshot_are_independent() {
    let source = c1_rich_source(true);
    let inventory = c1_rich_inventory(&source);
    assert_eq!(source.descriptor().planes().len(), 4);
    assert!(source.descriptor().planes_capacity_for_test() >= 4);
    assert_eq!(source.pixels().f32_planes().unwrap().len(), 4);
    assert!(source.pixels().f32_planes_capacity_for_test().unwrap() >= 4);
    for (descriptor, plane) in source
        .descriptor()
        .planes()
        .iter()
        .zip(source.pixels().f32_planes().unwrap())
    {
        assert!(descriptor.layout().channel_offsets_capacity_for_test() >= 1);
        assert!(descriptor.roles_capacity_for_test() >= 1);
        assert_eq!(plane.sample_len(), 5);
        assert!(plane.sample_capacity() >= 5);
        assert!(plane.layout().channel_offsets_capacity_for_test() >= 1);
    }
    assert_eq!(
        source.pixels().f32_planes().unwrap()[0].sample_capacity(),
        5
    );
    assert_eq!(
        source
            .metadata()
            .source_color()
            .icc_profile()
            .unwrap()
            .len(),
        4093
    );
    let source_color = source.metadata().source_color();
    assert!(source_color.provenance_capacity_for_test() >= 8);
    assert_eq!(source_color.provenance().len(), 3);
    assert!(source_color.icc_profile_capacity() >= 4093);
    assert!(source_color.icc_profile_capacity() > source_color.icc_profile().unwrap().len());
    assert_eq!(source_color.unknown_colr().len(), 2);
    assert!(source_color.unknown_colr_capacity_for_test() >= 5);
    assert_eq!(source_color.unknown_colr()[0].payload.len(), 13);
    assert_eq!(source_color.unknown_colr()[1].payload.len(), 13);
    assert!(source_color.unknown_colr()[0].payload.capacity() >= 32);
    assert!(source_color.unknown_colr()[1].payload.capacity() >= 64);
    assert_eq!(source.metadata().coded_geometry().len(), 2);
    assert!(source.metadata().coded_geometry_capacity_for_test() >= 6);
    assert_eq!(source.metadata().render_geometry().len(), 1);
    assert!(source.metadata().render_geometry_capacity_for_test() >= 4);
    let pixi = source.metadata().pixel_information().unwrap();
    assert_eq!(pixi.bits_per_channel().len(), 4);
    assert!(pixi.bits_capacity_for_test() >= 12);
    assert_eq!(pixi.extended_channels().unwrap().len(), 4);
    assert!(pixi.extended_capacity_for_test().unwrap() >= 8);
    let frame_limit = inventory.source_live.max(inventory.requested_output);
    let expected_live = inventory
        .source_live
        .checked_add(inventory.requested_output)
        .unwrap();
    let bounds = limits(frame_limit, expected_live);
    assert!(source.validate_with_limits(&bounds).is_ok());
    let output = OutputOwnershipPlan::inspect(&source, 4, 5, &bounds).unwrap();
    let ledger =
        ConstructionLedger::new_with_ownership(0, 0, inventory.source_live, &bounds).unwrap();
    let admitted = output.admit(ledger, source.metadata()).unwrap();
    assert_eq!(
        admitted.c1_accounting_snapshot(),
        (
            0,
            0,
            inventory.source_live,
            (inventory.requested_output, inventory.requested_metadata),
            0,
        )
    );
}

fn admitted() -> AdmittedOutputPlan<'static> {
    let frame: &'static ImageFrame = Box::leak(Box::new(source()));
    let resource_limits = limits(1 << 20, 1 << 20);
    let output = OutputOwnershipPlan::inspect(&frame, 3, 1, &resource_limits).unwrap();
    let ledger = ConstructionLedger::new_with_ownership(0, 0, 0, &resource_limits).unwrap();
    output.admit(ledger, frame.metadata()).unwrap()
}

fn admitted_at_exact_limit() -> AdmittedOutputPlan<'static> {
    let frame: &'static ImageFrame = Box::leak(Box::new(source()));
    let generous = limits(1 << 20, 1 << 20);
    let size = OutputOwnershipPlan::inspect(&frame, 3, 1, &generous)
        .unwrap()
        .total_bytes;
    let exact = limits(size, size);
    let output = OutputOwnershipPlan::inspect(&frame, 3, 1, &exact).unwrap();
    let ledger = ConstructionLedger::new_with_ownership(0, 0, 0, &exact).unwrap();
    output.admit(ledger, frame.metadata()).unwrap()
}

fn consume_all_owners(
    admitted: &mut AdmittedOutputPlan,
) -> Result<(), crate::highres::ProcessingError> {
    for _ in 0..3 {
        let _ = admitted.try_new_vec::<f32>(1)?;
    }
    let _ = admitted.try_new_vec::<PlaneDescriptor>(3)?;
    let _ = admitted.try_new_vec::<Plane<f32>>(3)?;
    for _ in 0..3 {
        let _ = admitted.try_new_vec::<usize>(1)?;
        let _ = admitted.try_new_vec::<ChannelRole>(1)?;
        let _ = admitted.try_new_vec::<usize>(1)?;
    }
    let _ = admitted.try_new_vec::<GeometryOperation>(0)?;
    let _ = admitted.try_new_vec::<GeometryOperation>(0)?;
    let _ = admitted.try_new_vec::<ColorProvenance>(0)?;
    let _ = admitted.try_new_vec::<UnknownColorInformation>(0)?;
    Ok(())
}

#[test]
fn admitted_materializer_restores_and_retries_the_same_plan() {
    let mut admitted = admitted();
    let before_pending = admitted.pending_bytes();
    let failed: Result<(), ProcessingError> = admitted.materialize_with(|admitted| {
        let _ = admitted.try_new_vec::<f32>(1)?;
        Err(ProcessingError::Unsupported(
            "test candidate failure".into(),
        ))
    });
    assert!(failed.is_err());
    assert_eq!(admitted.pending_bytes(), before_pending);

    let result = admitted.materialize_with(consume_all_owners);
    assert!(result.is_ok());
    assert_eq!(admitted.pending_bytes(), (0, 0));
}

#[test]
fn admitted_actual_capacity_is_checked_before_candidate_commit() {
    let mut admitted = admitted_at_exact_limit();
    let before_pending = admitted.pending_bytes();
    let result: Result<(), ProcessingError> = admitted.materialize_with(|admitted| {
        let _ = admitted.try_new_vec_with::<u8, _>(4, |_| {
            let mut candidate = Vec::new();
            candidate
                .try_reserve_exact(32)
                .map_err(|_| ProcessingError::Allocation("candidate allocation failed".into()))?;
            Ok(candidate)
        })?;
        Ok(())
    });
    assert!(matches!(result, Err(ProcessingError::ResourceLimit(_))));
    assert_eq!(admitted.pending_bytes(), before_pending);
}
