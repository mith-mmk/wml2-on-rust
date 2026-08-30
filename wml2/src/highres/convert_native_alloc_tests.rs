#[cfg(not(miri))]
use std::alloc::System;
#[cfg(not(miri))]
use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

const MAX_REGISTERED_OWNERS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegisteredOwner {
    pub(crate) pointer: usize,
    pub(crate) bytes: usize,
    pub(crate) align: usize,
    pub(crate) key: u8,
    pub(crate) owner_key: super::super::super::output_plan::OwnerKey,
    pub(crate) count: usize,
    pub(crate) ordinal: usize,
    pub(crate) deallocations: usize,
}

use super::super::super::{
    ChannelModel, ChannelRole, ColorConvertOptions, ColorInformationSet, Destination,
    ImageDescriptor, ImageFrame, MatrixCoefficients, NativeSampleEncoding, NclxColorInformation,
    PixelBuffer, Plane, PlaneDescriptor, PlaneLayout, ResourceLimits, RgbPrimaries, SampleDomain,
    SampleRange, Subsampling,
};
use super::NativePixelReader;

thread_local! {
    static OBSERVING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static DEALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static TARGET_POINTER: Cell<usize> = const { Cell::new(0) };
    static TARGET_BYTES: Cell<usize> = const { Cell::new(0) };
    static TARGET_ALIGN: Cell<usize> = const { Cell::new(0) };
    static TARGET_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static TARGET_DEALLOCATED: Cell<bool> = const { Cell::new(false) };
    static TARGET_DEALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static DENY_NEXT_ALLOCATION: Cell<bool> = const { Cell::new(false) };
    static REGISTERED_OWNERS: Cell<[RegisteredOwner; MAX_REGISTERED_OWNERS]> =
        const { Cell::new([RegisteredOwner { pointer: 0, bytes: 0, align: 0, key: 0, owner_key: super::super::super::output_plan::OwnerKey::DescriptorOuter, count: 0, ordinal: 0, deallocations: 0 }; MAX_REGISTERED_OWNERS]) };
}

#[cfg(not(miri))]
struct CountingAllocator;

#[cfg(not(miri))]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if deny_next_allocation() {
            record_allocation(true);
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc(layout) };
        record_allocation(pointer.is_null());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if deny_next_allocation() {
            record_allocation(true);
            return std::ptr::null_mut();
        }
        let pointer = unsafe { System.alloc_zeroed(layout) };
        record_allocation(pointer.is_null());
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        OBSERVING.with(|observing| {
            if observing.get() {
                DEALLOCATIONS.with(|deallocations| {
                    deallocations.set(deallocations.get().saturating_add(1));
                });
                let matches_target = TARGET_ACTIVE.with(Cell::get)
                    && TARGET_POINTER.with(|target| target.get() == pointer as usize)
                    && TARGET_BYTES.with(|target| target.get() == layout.size())
                    && TARGET_ALIGN.with(|target| target.get() == layout.align());
                if matches_target {
                    TARGET_DEALLOCATED.with(|deallocated| deallocated.set(true));
                    TARGET_DEALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
                }
                REGISTERED_OWNERS.with(|slot| {
                    let mut owners = slot.get();
                    for owner in &mut owners {
                        if owner.pointer == pointer as usize
                            && owner.bytes == layout.size()
                            && owner.align == layout.align()
                        {
                            owner.deallocations = owner.deallocations.saturating_add(1);
                            break;
                        }
                    }
                    slot.set(owners);
                });
            }
        });
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if deny_next_allocation() {
            record_allocation(true);
            return std::ptr::null_mut();
        }
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        record_allocation(replacement.is_null());
        replacement
    }
}

pub(crate) struct DeallocationObservationGuard {
    previous: bool,
    previous_deallocations: usize,
    previous_target: (usize, usize, usize, bool, bool, usize),
    previous_owners: [RegisteredOwner; MAX_REGISTERED_OWNERS],
    #[cfg(miri)]
    previous_candidate_drop_observer: Option<fn(usize, usize, usize)>,
}

impl DeallocationObservationGuard {
    pub(crate) fn begin() -> Self {
        let previous = OBSERVING.with(|observing| {
            let previous = observing.get();
            observing.set(true);
            previous
        });
        let previous_deallocations = DEALLOCATIONS.with(|deallocations| {
            let previous = deallocations.get();
            deallocations.set(0);
            previous
        });
        let previous_target = (
            TARGET_POINTER.with(Cell::get),
            TARGET_BYTES.with(Cell::get),
            TARGET_ALIGN.with(Cell::get),
            TARGET_ACTIVE.with(Cell::get),
            TARGET_DEALLOCATED.with(Cell::get),
            TARGET_DEALLOCATIONS.with(Cell::get),
        );
        let previous_owners = REGISTERED_OWNERS.with(Cell::get);
        #[cfg(miri)]
        let previous_candidate_drop_observer =
            super::super::super::allocation::swap_candidate_drop_observer(Some(
                mark_miri_candidate_drop,
            ));
        TARGET_POINTER.with(|target| target.set(0));
        TARGET_BYTES.with(|target| target.set(0));
        TARGET_ALIGN.with(|target| target.set(0));
        TARGET_ACTIVE.with(|target| target.set(false));
        TARGET_DEALLOCATED.with(|target| target.set(false));
        TARGET_DEALLOCATIONS.with(|target| target.set(0));
        REGISTERED_OWNERS.with(|owners| {
            owners.set(
                [RegisteredOwner {
                    pointer: 0,
                    bytes: 0,
                    align: 0,
                    key: 0,
                    owner_key: super::super::super::output_plan::OwnerKey::DescriptorOuter,
                    count: 0,
                    ordinal: 0,
                    deallocations: 0,
                }; MAX_REGISTERED_OWNERS],
            )
        });
        Self {
            previous,
            previous_deallocations,
            previous_target,
            previous_owners,
            #[cfg(miri)]
            previous_candidate_drop_observer,
        }
    }

    pub(crate) fn deallocations(&self) -> usize {
        DEALLOCATIONS.with(Cell::get)
    }

    pub(crate) fn register_target(pointer: *const u8, bytes: usize, align: usize) {
        TARGET_POINTER.with(|target| target.set(pointer as usize));
        TARGET_BYTES.with(|target| target.set(bytes));
        TARGET_ALIGN.with(|target| target.set(align));
        TARGET_ACTIVE.with(|target| target.set(true));
        TARGET_DEALLOCATED.with(|target| target.set(false));
        TARGET_DEALLOCATIONS.with(|target| target.set(0));
    }

    pub(crate) fn register_owner(
        pointer: *const u8,
        bytes: usize,
        align: usize,
        owner_key: super::super::super::output_plan::OwnerKey,
        key: u8,
        count: usize,
        ordinal: usize,
    ) {
        REGISTERED_OWNERS.with(|slot| {
            let mut owners = slot.get();
            for owner in &mut owners {
                if owner.pointer == 0 {
                    *owner = RegisteredOwner {
                        pointer: pointer as usize,
                        bytes,
                        align,
                        owner_key,
                        key,
                        count,
                        ordinal,
                        deallocations: 0,
                    };
                    break;
                }
            }
            slot.set(owners);
        });
    }

    /// Return the fixed registry snapshot in registration order.
    ///
    /// This deliberately returns the inline array rather than collecting into
    /// a `Vec`, so the observation itself cannot allocate or perturb the
    /// allocation/deallocation proof.
    pub(crate) fn registered_snapshot() -> ([RegisteredOwner; MAX_REGISTERED_OWNERS], usize) {
        REGISTERED_OWNERS.with(|owners| {
            let snapshot = owners.get();
            let count = snapshot.iter().filter(|owner| owner.pointer != 0).count();
            (snapshot, count)
        })
    }

    pub(crate) fn registered_status() -> (usize, usize, bool) {
        REGISTERED_OWNERS.with(|owners| {
            let owners = owners.get();
            let mut count = 0;
            let mut deallocations = 0;
            let mut exactly_once = true;
            for owner in owners {
                if owner.pointer != 0 {
                    count += 1;
                    deallocations += owner.deallocations;
                    exactly_once &= owner.deallocations == 1;
                }
            }
            (count, deallocations, exactly_once)
        })
    }

    pub(crate) fn target_deallocated() -> bool {
        TARGET_DEALLOCATED.with(Cell::get)
    }

    pub(crate) fn target_deallocations() -> usize {
        TARGET_DEALLOCATIONS.with(Cell::get)
    }
}

#[cfg(miri)]
fn mark_miri_candidate_drop(pointer: usize, bytes: usize, align: usize) {
    let active = TARGET_ACTIVE.with(Cell::get);
    let matches_target = active
        && TARGET_POINTER.with(|target| target.get() == pointer)
        && TARGET_BYTES.with(|target| target.get() == bytes)
        && TARGET_ALIGN.with(|target| target.get() == align);
    if matches_target {
        assert!(
            !TARGET_DEALLOCATED.with(Cell::get),
            "Miri target was marked before or more than once"
        );
        assert_eq!(TARGET_DEALLOCATIONS.with(Cell::get), 0);
        TARGET_DEALLOCATED.with(|deallocated| deallocated.set(true));
        TARGET_DEALLOCATIONS.with(|count| count.set(1));
    }
}

#[cfg(miri)]
fn mark_miri_target_after_drop(identity: (usize, usize, usize)) {
    assert!(
        TARGET_ACTIVE.with(Cell::get),
        "Miri target was not registered"
    );
    assert_eq!(TARGET_POINTER.with(Cell::get), identity.0);
    assert_eq!(TARGET_BYTES.with(Cell::get), identity.1);
    assert_eq!(TARGET_ALIGN.with(Cell::get), identity.2);
    assert!(
        !TARGET_DEALLOCATED.with(Cell::get),
        "Miri target was marked before or more than once"
    );
    assert_eq!(TARGET_DEALLOCATIONS.with(Cell::get), 0);
    TARGET_DEALLOCATED.with(|deallocated| deallocated.set(true));
    TARGET_DEALLOCATIONS.with(|count| count.set(1));
}

/// Drop a value after capturing the registered target identity, then record
/// the logical post-drop event for Miri. Host tests intentionally do not use
/// this helper: their physical deallocation remains owned by CountingAllocator.
#[cfg(miri)]
pub(crate) fn drop_registered_value<T>(value: T) {
    let identity = (
        TARGET_POINTER.with(Cell::get),
        TARGET_BYTES.with(Cell::get),
        TARGET_ALIGN.with(Cell::get),
    );
    assert!(
        TARGET_ACTIVE.with(Cell::get),
        "Miri target was not registered"
    );
    drop(value);
    mark_miri_target_after_drop(identity);
}

impl Drop for DeallocationObservationGuard {
    fn drop(&mut self) {
        OBSERVING.with(|observing| observing.set(self.previous));
        DEALLOCATIONS.with(|deallocations| deallocations.set(self.previous_deallocations));
        TARGET_POINTER.with(|target| target.set(self.previous_target.0));
        TARGET_BYTES.with(|target| target.set(self.previous_target.1));
        TARGET_ALIGN.with(|target| target.set(self.previous_target.2));
        TARGET_ACTIVE.with(|target| target.set(self.previous_target.3));
        TARGET_DEALLOCATED.with(|target| target.set(self.previous_target.4));
        TARGET_DEALLOCATIONS.with(|target| target.set(self.previous_target.5));
        REGISTERED_OWNERS.with(|owners| owners.set(self.previous_owners));
        #[cfg(miri)]
        super::super::super::allocation::swap_candidate_drop_observer(
            self.previous_candidate_drop_observer,
        );
    }
}

#[cfg(miri)]
#[test]
fn miri_observer_marks_a_registered_target_only_after_drop_and_cleans_up() {
    assert!(!DeallocationObservationGuard::target_deallocated());
    let guard = DeallocationObservationGuard::begin();
    let mut values = Vec::<u8>::new();
    values.try_reserve_exact(8).unwrap();
    DeallocationObservationGuard::register_target(
        values.as_ptr(),
        values.capacity(),
        std::mem::align_of::<u8>(),
    );
    assert!(!DeallocationObservationGuard::target_deallocated());
    drop_registered_value(values);
    assert!(DeallocationObservationGuard::target_deallocated());
    assert_eq!(DeallocationObservationGuard::target_deallocations(), 1);
    drop(guard);
    assert!(!DeallocationObservationGuard::target_deallocated());
    assert_eq!(DeallocationObservationGuard::target_deallocations(), 0);
}

#[cfg(not(miri))]
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[cfg(not(miri))]
fn record_allocation(failed: bool) {
    OBSERVING.with(|observing| {
        if observing.get() && !failed {
            ALLOCATIONS.with(|allocations| {
                allocations.set(allocations.get().saturating_add(1));
            });
        }
    });
}

#[cfg(not(miri))]
fn deny_next_allocation() -> bool {
    DENY_NEXT_ALLOCATION.with(|deny| {
        if deny.get() {
            deny.set(false);
            true
        } else {
            false
        }
    })
}

pub(crate) struct ActualAllocationDenyGuard {
    previous: bool,
}

impl ActualAllocationDenyGuard {
    pub(crate) fn arm_once() -> Self {
        let previous = DENY_NEXT_ALLOCATION.with(|deny| {
            let previous = deny.get();
            deny.set(true);
            previous
        });
        Self { previous }
    }
}

impl Drop for ActualAllocationDenyGuard {
    fn drop(&mut self) {
        DENY_NEXT_ALLOCATION.with(|deny| deny.set(self.previous));
    }
}

pub(crate) struct AllocationGuard {
    previous: bool,
}

impl AllocationGuard {
    pub(crate) fn begin() -> Self {
        let previous = OBSERVING.with(|observing| {
            let previous = observing.get();
            observing.set(true);
            previous
        });
        ALLOCATIONS.with(|allocations| allocations.set(0));
        Self { previous }
    }

    pub(crate) fn allocations(&self) -> usize {
        ALLOCATIONS.with(Cell::get)
    }
}

impl Drop for AllocationGuard {
    fn drop(&mut self) {
        OBSERVING.with(|observing| observing.set(self.previous));
    }
}

fn limits() -> ResourceLimits {
    ResourceLimits::builder()
        .max_input_bytes(1 << 20)
        .max_width(4096)
        .max_height(4096)
        .max_pixels(16_777_216)
        .max_channels(16)
        .max_planes(8)
        .max_plane_bytes(1 << 24)
        .max_frame_bytes(1 << 26)
        .max_total_live_decoded_bytes(1 << 27)
        .max_references(64)
        .max_frame_count(64)
        .max_metadata_bytes(1 << 20)
        .max_icc_bytes(1 << 20)
        .max_clut_bytes(1 << 24)
        .max_parser_entries(1 << 16)
        .max_parser_depth(64)
        .max_grid_cells(1 << 16)
        .max_derived_work(1 << 20)
        .max_derived_depth(32)
        .build()
        .unwrap()
}

fn cold_rgb_frame() -> ImageFrame {
    let layout = PlaneLayout::interleaved(2, 2, 4, Subsampling::FULL).unwrap();
    let descriptor = ImageDescriptor::new(
        2,
        2,
        ChannelModel::RGB,
        vec![
            PlaneDescriptor::new(
                layout.clone(),
                vec![
                    ChannelRole::Red,
                    ChannelRole::Green,
                    ChannelRole::Blue,
                    ChannelRole::Alpha,
                ],
                8,
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_alpha(super::super::super::AlphaAssociation::Straight)
    .unwrap()
    .with_color_information(
        ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, true)),
    )
    .with_domain(SampleDomain::Encoded);
    let pixels = PixelBuffer::u8(vec![
        Plane::new(
            layout,
            vec![
                10, 20, 30, 0, 40, 50, 60, 128, 70, 80, 90, 200, 100, 110, 120, 255,
            ],
        )
        .unwrap(),
    ])
    .unwrap();
    ImageFrame::new(descriptor, pixels).unwrap()
}

fn encoded_options(
    matrix: MatrixCoefficients,
    location: Option<super::super::super::ChromaLocation>,
) -> ColorConvertOptions<'static> {
    ColorConvertOptions::new(Destination::linear_rgb(
        SampleDomain::LinearRelative,
        RgbPrimaries::srgb(),
    ))
    .with_native_sample_encoding(NativeSampleEncoding::new(
        SampleRange::Full,
        matrix,
        location,
    ))
}

#[test]
fn cold_native_reader_inspect_and_all_role_pixels_allocate_zero_times() {
    let frame = cold_rgb_frame();
    let options = encoded_options(MatrixCoefficients::Identity, None);
    let limits = limits();

    let guard = AllocationGuard::begin();
    let reader = NativePixelReader::inspect(&frame, &options, &limits).unwrap();
    let pixels = [
        reader.pixel(0, 0).unwrap(),
        reader.pixel(1, 0).unwrap(),
        reader.pixel(0, 1).unwrap(),
        reader.pixel(1, 1).unwrap(),
    ];
    assert_eq!(pixels.len(), 4);
    assert_eq!(guard.allocations(), 0);

    drop(guard);
    let _ = limits;
}

#[test]
fn allocation_guard_counts_a_real_allocation_and_resets_after_scope() {
    let mut values = Vec::<u8>::new();
    let guard = AllocationGuard::begin();
    values.reserve_exact(1);
    std::hint::black_box(&values);
    assert!(guard.allocations() >= 1);
    drop(guard);
    drop(values);

    let guard = AllocationGuard::begin();
    std::hint::black_box(0usize);
    assert_eq!(guard.allocations(), 0);
}

#[test]
fn cold_gray_reader_and_ycbcr_interpolation_allocate_zero_times() {
    let gray_descriptor = ImageDescriptor::gray(2, 2, 8)
        .unwrap()
        .with_color_information(
            ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 0, true)),
        )
        .with_domain(SampleDomain::Encoded);
    let gray_layout = PlaneLayout::planar(2, 2, Subsampling::FULL).unwrap();
    let gray_frame = ImageFrame::new(
        gray_descriptor,
        PixelBuffer::u8(vec![
            Plane::new(gray_layout, vec![0, 64, 128, 255]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let gray_options = encoded_options(MatrixCoefficients::Identity, None);
    let guard = AllocationGuard::begin();
    let gray_reader = NativePixelReader::inspect(&gray_frame, &gray_options, &limits()).unwrap();
    let _ = [
        gray_reader.pixel(0, 0).unwrap(),
        gray_reader.pixel(1, 0).unwrap(),
        gray_reader.pixel(0, 1).unwrap(),
        gray_reader.pixel(1, 1).unwrap(),
    ];
    assert_eq!(guard.allocations(), 0);
    drop(guard);

    let subsampling = Subsampling::new(2, 2).unwrap();
    let ycbcr_descriptor =
        ImageDescriptor::ycbcr(2, 2, 8, [Subsampling::FULL, subsampling, subsampling])
            .unwrap()
            .with_color_information(
                ColorInformationSet::new().with_nclx(NclxColorInformation::new(1, 13, 1, true)),
            )
            .with_domain(SampleDomain::Encoded);
    let y_layout = PlaneLayout::planar(2, 2, Subsampling::FULL).unwrap();
    let c_layout = PlaneLayout::planar(1, 1, subsampling).unwrap();
    let ycbcr_frame = ImageFrame::new(
        ycbcr_descriptor,
        PixelBuffer::u8(vec![
            Plane::new(y_layout, vec![128; 4]).unwrap(),
            Plane::new(c_layout.clone(), vec![200]).unwrap(),
            Plane::new(c_layout, vec![100]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    let location = super::super::super::ChromaLocation::from_h273_code(0).unwrap();
    let ycbcr_options = encoded_options(MatrixCoefficients::Bt709, Some(location));
    let guard = AllocationGuard::begin();
    let ycbcr_reader = NativePixelReader::inspect(&ycbcr_frame, &ycbcr_options, &limits()).unwrap();
    let _ = [
        ycbcr_reader.pixel(0, 0).unwrap(),
        ycbcr_reader.pixel(1, 0).unwrap(),
        ycbcr_reader.pixel(0, 1).unwrap(),
        ycbcr_reader.pixel(1, 1).unwrap(),
    ];
    assert_eq!(guard.allocations(), 0);
}
