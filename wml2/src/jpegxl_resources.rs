//! Caller-tightened admission and adapter-owned live storage accounting.
use super::mul;
use super::{Error, MAX_BROB_METADATA, MAX_PIXELS, add, invalid};
use crate::limits::{self, DecodeLimits};

/// Additional, finite JPEG XL limits for the explicit resource-limited APIs.
///
/// These only tighten the shipping codec envelope. In particular they cannot
/// admit larger canvases or remove the padded-frame screen. `workspace_bytes`
/// bounds the existing conservative admission estimate plus simultaneously live
/// adapter buffers; it is not a global allocator, process RSS or WASM ceiling.
/// The pinned codec still has allocations outside an enforceable byte ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JxlResourceLimits {
    pub workspace_bytes: usize,
    /// Scanned padded/upsampled internal and preview geometry, including minimum-16 allowance.
    pub internal_pixels: usize,
    /// Historical ceil(max(width,16)*max(height,16)*4/3) admission basis.
    /// This is a sample-limit allowance, not measured physical padding.
    pub effective_pixels: usize,
    /// Main coded layers, including invisible/reference layers; excludes skipped preview.
    pub internal_layers: usize,
    pub toc_entries: usize,
    pub icc_coded_bytes: usize,
    pub icc_expanded_bytes: usize,
    /// Cumulative decompressed auxiliary payload, independent of compressed input.
    pub brob_bytes: usize,
}

impl Default for JxlResourceLimits {
    fn default() -> Self {
        Self {
            workspace_bytes: 512 << 20,
            ..Self::legacy()
        }
    }
}

impl JxlResourceLimits {
    pub(super) const fn legacy() -> Self {
        Self {
            workspace_bytes: usize::MAX,
            internal_pixels: MAX_PIXELS,
            effective_pixels: MAX_PIXELS,
            internal_layers: 64,
            toc_entries: 4096,
            icc_coded_bytes: 1 << 24,
            icc_expanded_bytes: 1 << 28,
            brob_bytes: MAX_BROB_METADATA,
        }
    }

    pub(super) fn validate(self) -> Result<(), Error> {
        let ceiling = Self::legacy();
        if self.workspace_bytes == usize::MAX
            || self.internal_pixels > ceiling.internal_pixels
            || self.effective_pixels > ceiling.effective_pixels
            || self.internal_layers > ceiling.internal_layers
            || self.toc_entries > ceiling.toc_entries
            || self.icc_coded_bytes > ceiling.icc_coded_bytes
            || self.icc_expanded_bytes > ceiling.icc_expanded_bytes
            || self.brob_bytes > ceiling.brob_bytes
        {
            return Err(invalid(
                "JPEG XL resource limits must be finite and cannot raise shipping caps",
            ));
        }
        Ok(())
    }

    pub(super) fn tighten(self, limits: DecodeLimits) -> DecodeLimits {
        DecodeLimits {
            expanded_bytes: limits.expanded_bytes.min(self.workspace_bytes),
            ..limits
        }
    }
}

// Published jxl0.8.0 Palette::new owns25 process-global OnceLock entries.
// Each entry owns3 fixed RGB i32 arrays and a3-pointer boxed descriptor. Admit
// every slot independent of process history; do not inspect private cache state.
// This does not cover other codec statics or make upstream allocation fallible.
pub(super) const IMPLICIT_PALETTE_CACHE_BYTES: usize =
    25 * (3 * (143 + 64 + 125) * std::mem::size_of::<i32>() + 3 * std::mem::size_of::<usize>());

pub(super) fn header_workspace(
    metadata_bytes: usize,
    policy: JxlResourceLimits,
) -> Result<usize, Error> {
    let base = add(1024 * 1024, mul(metadata_bytes, 2)?)?;
    // Legacy admission is unchanged. Explicit finite resource APIs always hold
    // the worst-case static allowance, including when the cache is already warm.
    add(
        base,
        if policy.workspace_bytes == usize::MAX {
            0
        } else {
            IMPLICIT_PALETTE_CACHE_BYTES
        },
    )
}

/// Extra live adapter storage, on top of the conservative retained codec/metadata
/// estimate. Absent on legacy APIs so their acceptance and error order stay fixed.
pub(super) struct Workspace {
    live: usize,
    limit: usize,
}
impl Workspace {
    pub(super) fn new(live: usize, limit: usize) -> Result<Self, Error> {
        limits::check(live, limit, "JPEG XL retained workspace")?;
        Ok(Self { live, limit })
    }
    pub(super) fn claim(&mut self, bytes: usize, stage: &str) -> Result<(), Error> {
        let live = add(self.live, bytes)?;
        limits::check(live, self.limit, stage)?;
        self.live = live;
        Ok(())
    }
    pub(super) fn release(&mut self, bytes: usize) -> Result<(), Error> {
        self.live = self
            .live
            .checked_sub(bytes)
            .ok_or_else(|| invalid("JPEG XL workspace release mismatch"))?;
        Ok(())
    }
    pub(super) fn accounted(&self) -> usize {
        self.live
    }
    pub(super) fn remaining(&self) -> usize {
        self.limit - self.live
    }
}

/// Pre-admit retained profile/header payloads before their owned copy. The final
/// retained-metadata estimate replaces this provisional subtotal; it is never
/// added a second time to the later output/CMS ledger.
pub(super) struct MetadataCopies {
    base_workspace: usize,
    container_metadata: usize,
    added: usize,
    limits: DecodeLimits,
}
impl MetadataCopies {
    pub(super) fn new(
        base_workspace: usize,
        container_metadata: usize,
        limits: DecodeLimits,
    ) -> Self {
        Self {
            base_workspace,
            container_metadata,
            added: 0,
            limits,
        }
    }
    pub(super) fn claim(&mut self, key: &str, bytes: usize) -> Result<(), Error> {
        let added = add(self.added, add(key.len(), bytes)?)?;
        limits::check(
            add(self.container_metadata, added)?,
            self.limits.metadata_bytes,
            "JPEG XL metadata copies",
        )?;
        limits::check(
            add(self.base_workspace, super::mul(added, 2)?)?,
            self.limits.expanded_bytes,
            "JPEG XL metadata copy workspace",
        )?;
        self.added = added;
        Ok(())
    }
    pub(super) fn finish(self, retained_added: usize) -> Result<(), Error> {
        if retained_added < self.added {
            return Err(invalid("JPEG XL metadata copy accounting mismatch"));
        }
        Ok(())
    }
}

// icc-profile0.0.7 Profile::parse_with_limits owns one declared payload and a
// (u32,usize,usize) tag-range Vec, not copies of aliased tag data. Reserve a fixed
// allowance for its two small Arc/ProfileInner objects as well. TransformLimits
// separately admits compiled and temporary transform storage in the remainder.
#[cfg(feature = "color-management")]
pub(super) fn cms_profiles(source: &[u8], destination: &[u8]) -> Result<usize, Error> {
    fn one(bytes: &[u8]) -> Result<usize, Error> {
        if bytes.len() < 132 {
            return Err(invalid("truncated JPEG XL CMS profile"));
        }
        let size = u32::from_be_bytes(bytes[..4].try_into()?) as usize;
        let tags = u32::from_be_bytes(bytes[128..132].try_into()?) as usize;
        if size < 132 || size > bytes.len() || tags > 4096 || add(132, mul(tags, 12)?)? > size {
            return Err(invalid("invalid JPEG XL CMS profile size/tag count"));
        }
        add(
            add(size, mul(tags, std::mem::size_of::<(u32, usize, usize)>())?)?,
            4096,
        )
    }
    add(one(source)?, one(destination)?)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    struct FailOutput;
    thread_local! {
        static FAIL_BYTES: Cell<usize> = const { Cell::new(0) };
        static FAILED_BYTES: Cell<usize> = const { Cell::new(0) };
        static COPY_KEY: Cell<Option<&'static str>> = const { Cell::new(None) };
    }
    struct ResetFault;
    impl Drop for ResetFault {
        fn drop(&mut self) {
            COPY_KEY.with(|f| f.set(None));
            FAIL_BYTES.with(|f| f.set(0));
        }
    }
    pub(crate) fn before_metadata_copy(key: &str, bytes: usize) {
        COPY_KEY.with(|site| {
            if site.get() == Some(key) {
                site.set(None);
                FAIL_BYTES.with(|f| f.set(bytes));
            }
        });
    }
    fn refused(bytes: usize) -> bool {
        FAIL_BYTES
            .try_with(|flag| {
                if bytes != 0 && flag.get() == bytes {
                    flag.set(0);
                    FAILED_BYTES.with(|f| f.set(bytes));
                    true
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }
    // SAFETY: all non-injected allocations are forwarded unchanged; the injected
    // null is the documented GlobalAlloc failure result, scoped to one test thread.
    unsafe impl GlobalAlloc for FailOutput {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if refused(layout.size()) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc(layout) }
            }
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            if refused(layout.size()) {
                std::ptr::null_mut()
            } else {
                unsafe { System.alloc_zeroed(layout) }
            }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, bytes: usize) -> *mut u8 {
            if refused(bytes) {
                std::ptr::null_mut()
            } else {
                unsafe { System.realloc(ptr, layout, bytes) }
            }
        }
    }
    #[global_allocator]
    static ALLOCATOR: FailOutput = FailOutput;
    #[test]
    fn float_hdr_output_allocator_refusal_is_recoverable() {
        let _reset = ResetFault;
        for input in [
            include_bytes!("../tests/fixtures/review/jxl-float-hdr-pq-f16-rgba-o6.jxl").as_slice(),
            include_bytes!("../tests/fixtures/review/jxl-float-hdr-hlg-f32-rgba-o6.jxl").as_slice(),
        ] {
            COPY_KEY.with(|f| f.set(Some("JPEG XL F32 output")));
            FAILED_BYTES.with(|f| f.set(0));
            let result = super::super::image_hdr_encoded_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false,
            );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>()
            );
            assert_eq!(FAILED_BYTES.with(Cell::get), 240);
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(
                super::super::image_hdr_encoded_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn extra_formats_admit_before_allocating_and_roll_back_refusal() {
        let _reset = ResetFault;
        let bytes = std::mem::size_of::<Option<jxl_codec::api::JxlDataFormat>>();
        let mut zero = Workspace::new(0, 0).unwrap();
        let empty = super::super::output_extra_formats(0, Some(&mut zero)).unwrap();
        assert_eq!(empty.capacity(), 0);
        assert_eq!(zero.accounted(), 0);
        let mut short = Workspace::new(10, 10 + bytes - 1).unwrap();
        COPY_KEY.with(|f| f.set(Some("JPEG XL extra formats")));
        FAILED_BYTES.with(|f| f.set(0));
        assert!(super::super::output_extra_formats(1, Some(&mut short)).is_err());
        assert_eq!(COPY_KEY.with(Cell::get), Some("JPEG XL extra formats"));
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        assert_eq!(short.accounted(), 10);
        let mut exact = Workspace::new(10, 10 + bytes).unwrap();
        let error = super::super::output_extra_formats(1, Some(&mut exact)).unwrap_err();
        assert!(error.is::<std::collections::TryReserveError>());
        assert_eq!(FAILED_BYTES.with(Cell::get), bytes);
        assert_eq!(exact.accounted(), 10);
        let formats = super::super::output_extra_formats(1, Some(&mut exact)).unwrap();
        assert_eq!(formats.len(), 1);
        assert!(formats[0].is_none());
        assert_eq!(exact.accounted(), 10 + bytes);
        assert!(super::super::output_extra_formats(usize::MAX, None).is_err());
    }

    #[test]
    fn each_extra_format_failure_returns_no_image_and_recovers() {
        let _reset = ResetFault;
        let size = std::mem::size_of::<Option<jxl_codec::api::JxlDataFormat>>();
        let decode = |mode| -> Result<(), Error> {
            let l = DecodeLimits::default();
            let p = JxlResourceLimits::default();
            match mode {
                0 => crate::draw::image_from(include_bytes!(
                    "../tests/fixtures/review/jxl-rgba8.jxl"
                ))
                .map(drop),
                1 => super::super::image_from_with_resource_limits(
                    include_bytes!("../tests/fixtures/review/jxl-rgba8.jxl"),
                    l,
                    p,
                )
                .map(drop),
                2 => super::super::image_f32_from_with_resource_limits_and_cancel(
                    include_bytes!("../tests/fixtures/review/jxl-native-f32-rgba.jxl"),
                    l,
                    p,
                    &|| false,
                )
                .map(drop),
                3 => super::super::image_hdr_encoded_from_with_resource_limits_and_cancel(
                    include_bytes!("../tests/fixtures/review/jxl-float-hdr-hlg-f16-rgba-o6.jxl"),
                    l,
                    p,
                    &|| false,
                )
                .map(drop),
                _ => super::super::preview_from_with_resource_limits(
                    include_bytes!("../tests/fixtures/review/jxl-preview-rgba8.jxl"),
                    l,
                    p,
                )
                .map(drop),
            }
        };
        for mode in 0..5 {
            COPY_KEY.with(|f| f.set(Some("JPEG XL extra formats")));
            FAILED_BYTES.with(|f| f.set(0));
            let e = decode(mode).unwrap_err();
            assert!(e.is::<std::collections::TryReserveError>(), "{mode}: {e}");
            assert_eq!(FAILED_BYTES.with(Cell::get), size);
            assert_eq!(COPY_KEY.with(Cell::get), None);
            decode(mode).unwrap();
        }
    }

    #[test]
    fn container_record_growth_admits_overlap_and_preserves_owners_on_failure() {
        let _reset = ResetFault;
        use super::super::container_metadata::{record_bytes, reserve};
        let mut records = Vec::new();
        let limit = |n| DecodeLimits {
            expanded_bytes: n,
            ..Default::default()
        };
        COPY_KEY.with(|f| f.set(Some("container records")));
        FAILED_BYTES.with(|f| f.set(0));
        assert!(
            reserve(
                &mut records,
                1,
                0,
                0,
                limit(record_bytes(4).unwrap() - 1),
                true
            )
            .is_err()
        );
        assert_eq!(COPY_KEY.with(Cell::get), Some("container records"));
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        assert!(
            reserve(&mut records, 1, 0, 0, limit(record_bytes(4).unwrap()), true)
                .unwrap_err()
                .is::<std::collections::TryReserveError>()
        );
        assert_eq!(records.capacity(), 0);
        assert_eq!(FAILED_BYTES.with(Cell::get), record_bytes(4).unwrap());
        reserve(&mut records, 1, 0, 0, limit(record_bytes(4).unwrap()), true).unwrap();
        for n in 0..4 {
            records.push((n.to_string(), crate::metadata::DataMap::Raw(vec![n as u8])));
        }
        let old = records.clone();
        let overlap = record_bytes(12).unwrap();
        COPY_KEY.with(|f| f.set(Some("container records")));
        FAILED_BYTES.with(|f| f.set(0));
        assert!(reserve(&mut records, 1, 0, 0, limit(overlap - 1), true).is_err());
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        assert!(
            reserve(&mut records, 1, 0, 0, limit(overlap), true)
                .unwrap_err()
                .is::<std::collections::TryReserveError>()
        );
        assert_eq!(records, old);
        assert_eq!(records.capacity(), 4);
        assert_eq!(FAILED_BYTES.with(Cell::get), record_bytes(8).unwrap());
        reserve(&mut records, 1, 0, 0, limit(overlap), true).unwrap();
        assert_eq!(records, old);
        assert_eq!(records.capacity(), 8);
        assert!(
            reserve(
                &mut records,
                usize::MAX,
                0,
                0,
                DecodeLimits::unlimited(),
                true
            )
            .is_err()
        );
        assert!(
            reserve(
                &mut records,
                1,
                usize::MAX,
                0,
                DecodeLimits::unlimited(),
                true
            )
            .is_err()
        );
    }
    #[test]
    fn container_keys_records_and_raw_copy_fail_without_a_published_image() {
        let _reset = ResetFault;
        for (bytes, compressed) in [
            (
                include_bytes!("../tests/fixtures/review/jxl-brob-metadata.jxl").as_slice(),
                true,
            ),
            (
                include_bytes!("../tests/fixtures/review/jxl-brob-metadata-plain.jxl").as_slice(),
                false,
            ),
        ] {
            for finite in [false, true] {
                for site in [
                    "container records",
                    "Source",
                    "Decoded",
                    "container raw payload",
                ] {
                    if site == "Decoded" && !compressed {
                        continue;
                    }
                    COPY_KEY.with(|f| f.set(Some(site)));
                    FAILED_BYTES.with(|f| f.set(0));
                    let result = if finite {
                        super::super::image_from_with_resource_limits(
                            bytes,
                            DecodeLimits::default(),
                            JxlResourceLimits::default(),
                        )
                    } else {
                        crate::draw::image_from(bytes)
                    };
                    let e = result.err().expect("allocation refusal must fail");
                    assert!(e.is::<std::collections::TryReserveError>(), "{site}: {e}");
                    assert!(FAILED_BYTES.with(Cell::get) > 0);
                    assert_eq!(COPY_KEY.with(Cell::get), None);
                    assert!(
                        super::super::image_from_with_resource_limits(
                            bytes,
                            DecodeLimits::default(),
                            JxlResourceLimits::default()
                        )
                        .is_ok()
                    );
                }
            }
            COPY_KEY.with(|f| f.set(Some("Source")));
            FAILED_BYTES.with(|f| f.set(0));
            assert!(
                super::super::image_from_with_resource_limits(
                    bytes,
                    DecodeLimits {
                        metadata_bytes: 0,
                        ..Default::default()
                    },
                    JxlResourceLimits::default()
                )
                .is_err()
            );
            assert_eq!(COPY_KEY.with(Cell::get), Some("Source"));
            assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        }
    }

    #[test]
    fn runtime_output_reservation_propagates_allocator_refusal_and_checked_size() {
        let _reset = ResetFault;
        FAIL_BYTES.with(|f| f.set(256));
        let result = super::super::aligned_output(256);
        assert!(result.is_err());
        assert_eq!(FAIL_BYTES.with(Cell::get), 0); // actual allocation attempted
        assert_eq!(super::super::aligned_output(256).unwrap().len(), 128);
        assert!(super::super::aligned_output(usize::MAX).is_err());
    }

    #[test]
    fn metadata_claims_are_cumulative_and_transferred_without_double_charging() {
        let limits = DecodeLimits {
            expanded_bytes: 140,
            metadata_bytes: 40,
            ..DecodeLimits::unlimited()
        };
        let mut copies = MetadataCopies::new(100, 20, limits);
        copies.claim("key", 10).unwrap(); // +13 retained bytes
        assert!(copies.claim("ab", 6).is_err()); // 21 added bytes exceeds both quotas
        copies.claim("ab", 5).unwrap(); // exactly20, failed claims did not mutate
        copies.finish(20).unwrap();
        let mut output = Workspace::new(140, 150).unwrap();
        output.claim(10, "collector").unwrap(); // no second +40 metadata charge
        assert!(output.claim(1, "over").is_err());
        let mut copies = MetadataCopies::new(0, 0, DecodeLimits::unlimited());
        assert!(copies.claim("x", usize::MAX).is_err());
        let mut copies = MetadataCopies::new(100, 0, limits);
        copies.claim("key", 10).unwrap();
        assert!(copies.finish(12).is_err());
    }

    #[test]
    fn profile_selection_borrows_icc_payload_instead_of_cloning_it() {
        use jxl_codec::api::JxlColorProfile;
        let original = JxlColorProfile::Icc(
            include_bytes!("../tests/fixtures/review/jxl-gamma2.icc").to_vec(),
        );
        let output = super::super::metadata_output_profile(&original, false).unwrap();
        assert!(matches!(output, std::borrow::Cow::Borrowed(_)));
        assert!(std::ptr::eq(output.as_ref(), &original));
        assert!(super::super::metadata_output_profile(&original, true).is_err());
    }

    #[test]
    fn each_runtime_metadata_copy_failure_returns_without_publishing_legacy_output() {
        let _reset = ResetFault;
        use crate::draw::{DecodeOptions, ImageBuffer, image_decoder_with_limits};
        let input = include_bytes!("../tests/fixtures/review/jxl-icc-lossless.jxl");
        for key in [
            "JPEG XL Source ICC Profile",
            "ICC Profile",
            "JPEG XL Source Header",
        ] {
            let mut reader = bin_rs::reader::BytesReader::new(input);
            let mut image = ImageBuffer::new();
            FAILED_BYTES.with(|f| f.set(0));
            COPY_KEY.with(|f| f.set(Some(key)));
            let result = image_decoder_with_limits(
                &mut reader,
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,
                },
                DecodeLimits::default(),
            );
            assert!(
                result
                    .err()
                    .unwrap()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0);
            assert_eq!((image.width, image.height), (0, 0));
            assert!(image.buffer.is_none() && image.metadata.is_none());
        }
        assert!(crate::draw::image_from_with_limits(input, DecodeLimits::default()).is_ok());
    }

    #[test]
    fn equal_simple_profile_copy_refusal_never_publishes_output() {
        let _reset = ResetFault;
        use crate::draw::{DecodeOptions, ImageBuffer, image_decoder_with_limits};
        let input = include_bytes!("../tests/fixtures/review/jxl-rgb8.jxl");
        let mut reader = bin_rs::reader::BytesReader::new(input);
        let mut image = ImageBuffer::new();
        COPY_KEY.with(|f| f.set(Some("ICC Profile")));
        FAILED_BYTES.with(|f| f.set(0));
        let result = image_decoder_with_limits(
            &mut reader,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
            DecodeLimits::default(),
        );
        assert!(
            result
                .err()
                .unwrap()
                .is::<std::collections::TryReserveError>()
        );
        assert!(FAILED_BYTES.with(Cell::get) > 0);
        assert_eq!(COPY_KEY.with(Cell::get), None);
        assert_eq!((image.width, image.height), (0, 0));
        assert!(image.buffer.is_none() && image.metadata.is_none());
        let recovered =
            crate::draw::image_from_with_limits(input, DecodeLimits::default()).unwrap();
        let metadata = recovered.metadata.unwrap();
        assert_eq!(
            metadata.get("JPEG XL Source ICC Profile").unwrap(),
            metadata.get("ICC Profile").unwrap()
        );
    }

    #[test]
    fn resource_quota_denial_precedes_the_source_icc_copy_request() {
        let _reset = ResetFault;
        let bytes = include_bytes!("../tests/fixtures/review/jxl-icc-lossless.jxl");
        let unpacked = super::super::unpack(bytes, DecodeLimits::unlimited()).unwrap();
        let header = super::super::preflight_with_resources(
            &unpacked,
            DecodeLimits::unlimited(),
            JxlResourceLimits::default(),
        )
        .unwrap();
        let policy = JxlResourceLimits {
            workspace_bytes: header.workspace
                + usize::from(header.alpha_bits.is_some())
                    * std::mem::size_of::<Option<jxl_codec::api::JxlDataFormat>>(),
            ..Default::default()
        };
        COPY_KEY.with(|f| f.set(Some("JPEG XL Source ICC Profile")));
        FAILED_BYTES.with(|f| f.set(0));
        let error =
            super::super::image_from_with_resource_limits(bytes, DecodeLimits::unlimited(), policy)
                .err()
                .unwrap();
        assert!(
            error.to_string().contains("metadata copy workspace"),
            "{error}"
        );
        assert_eq!(COPY_KEY.with(Cell::get), Some("JPEG XL Source ICC Profile"));
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        COPY_KEY.with(|f| f.set(None));
    }

    #[cfg(feature = "color-management")]
    #[test]
    fn managed_metadata_copy_failures_never_return_an_image() {
        let _reset = ResetFault;
        let input = include_bytes!("../tests/fixtures/review/jxl-icc-lossless.jxl");
        for key in [
            "JPEG XL Source ICC Profile",
            "JPEG XL Reconstruction ICC Profile",
            "ICC Profile",
            "JPEG XL Source Header",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result = super::super::image_from_srgb_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
            );
            assert!(
                result
                    .err()
                    .unwrap()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0);
        }
    }
    #[test]
    fn live_claims_are_cumulative_checked_and_release_only_dead_storage() {
        let mut w = Workspace::new(100, 150).unwrap();
        w.claim(30, "aligned").unwrap();
        assert!(w.claim(21, "collector").is_err());
        w.claim(20, "collector").unwrap();
        assert!(w.claim(1, "over").is_err());
        #[cfg(feature = "color-management")]
        {
            w.release(30).unwrap();
            assert_eq!(w.remaining(), 30);
            w.claim(30, "next").unwrap();
        }
        let mut w = Workspace::new(usize::MAX - 1, usize::MAX).unwrap();
        assert!(w.claim(2, "overflow").is_err());
    }
    #[cfg(feature = "color-management")]
    #[test]
    fn cms_parse_uses_declared_bytes_and_ranges_without_alias_amplification() {
        let profile = crate::color_convert::SRGB_ICC;
        let expected = 2
            * (profile.len()
                + 4096
                + u32::from_be_bytes(profile[128..132].try_into().unwrap()) as usize
                    * std::mem::size_of::<(u32, usize, usize)>());
        assert_eq!(cms_profiles(profile, profile).unwrap(), expected);
        assert!(cms_profiles(&profile[..131], profile).is_err());
        let mut bad = profile.to_vec();
        bad[128..132].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(cms_profiles(&bad, profile).is_err());
    }
    #[test]
    fn each_extra_output_reservation_refuses_without_publishing_either_plane() {
        let _reset = ResetFault;
        let input = include_bytes!("../tests/fixtures/review/jxl-depth-3c8d16o5.jxl");
        for key in [
            "extra formats",
            "extra source header",
            "extra source ICC",
            "extra output ICC",
            "extra color buffer",
            "extra Depth buffer",
            "extra output buffers",
            "extra plane descriptors",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result =
                super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false,
                );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn each_selection_output_reservation_refuses_without_publishing_either_plane() {
        let _reset = ResetFault;
        let input = include_bytes!("../tests/fixtures/review/jxl-selection-3c8d16o5.jxl");
        for key in [
            "extra formats",
            "extra source header",
            "extra source ICC",
            "extra output ICC",
            "extra color buffer",
            "extra Depth buffer",
            "extra output buffers",
            "extra plane descriptors",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result =
                super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false,
                );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn each_two_plane_reservation_refuses_without_publishing_either_plane() {
        let _reset = ResetFault;
        let input = include_bytes!("../tests/fixtures/review/jxl-two-3c12-MD-8-16-o5.jxl");
        for key in [
            "extra formats",
            "extra source header",
            "extra source ICC",
            "extra output ICC",
            "extra color buffer",
            "extra Depth buffer",
            "extra second plane buffer",
            "extra output buffers",
            "extra plane descriptors",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result =
                super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false,
                );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn each_main5_alpha_reservation_refuses_without_publishing_either_plane() {
        let _reset = ResetFault;
        let input = include_bytes!("../tests/fixtures/review/jxl-main5-3c8-AMDM-12-16-8-9-o4.jxl");
        for key in [
            "extra formats",
            "extra source header",
            "extra source ICC",
            "extra output ICC",
            "extra color buffer",
            "extra Depth buffer",
            "extra second plane buffer",
            "extra third plane buffer",
            "extra fourth plane buffer",
            "extra output buffers",
            "extra plane descriptors",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result =
                super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false,
                );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn each_reconstructed_extra_reservation_refuses_without_publishing_either_plane() {
        let _reset = ResetFault;
        let input =
            include_bytes!("../tests/fixtures/review/jxl-extra-alpha-097-associated-s2.jxl");
        for key in [
            "extra formats",
            "extra source header",
            "extra source ICC",
            "extra output ICC",
            "extra color buffer",
            "extra Depth buffer",
            "extra second plane buffer",
            "extra third plane buffer",
            "extra fourth plane buffer",
            "extra output buffers",
            "extra plane descriptors",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result =
                super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default(),
                    &|| false,
                );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_with_extra_channels_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn every_buffered_animation_reservation_refuses_without_partial_result() {
        let _reset = ResetFault;
        let input =
            include_bytes!("../tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");
        for key in [
            "animation timing plan",
            "animation formats",
            "animation source header",
            "animation source ICC",
            "animation output ICC",
            "animation frame descriptors",
            "animation frame0",
            "animation frame1",
            "animation later frame",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result = super::super::animation_rgba16_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false,
            );
            assert!(
                result
                    .unwrap_err()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::animation_rgba16_from_with_resource_limits_and_cancel(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
                &|| false
            )
            .is_ok()
        );
    }
    #[test]
    fn resident_animation_metadata_is_reserved_before_native_work_and_rejoined() {
        let _reset = ResetFault;
        use super::super::animation::{animation_with_workspace, incoming_metadata_storage};
        let mut key = String::with_capacity(128);
        key.push_str("key");
        let mut raw = Vec::with_capacity(1024);
        raw.extend_from_slice(b"abc");
        let mut metadata = Vec::with_capacity(4);
        metadata.push((key, crate::metadata::DataMap::Raw(raw)));
        let resident = incoming_metadata_storage(&metadata).unwrap();
        assert_eq!(
            resident,
            super::super::container_metadata::record_bytes(metadata.capacity()).unwrap()
                + metadata[0].0.capacity()
                + match &metadata[0].1 {
                    crate::metadata::DataMap::Raw(v) => v.capacity(),
                    _ => unreachable!(),
                }
        );
        let input =
            include_bytes!("../tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");
        let (_, normal) = animation_with_workspace(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
            0,
        )
        .unwrap();
        let (_, held) = animation_with_workspace(
            input,
            DecodeLimits::default(),
            JxlResourceLimits::default(),
            &|| false,
            resident,
        )
        .unwrap();
        assert_eq!(held.accounted(), normal.accounted() + resident);
        assert_eq!(held.remaining() + resident, normal.remaining());
        COPY_KEY.with(|f| f.set(Some("animation frame descriptors")));
        FAILED_BYTES.with(|f| f.set(0));
        let error = animation_with_workspace(
            input,
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: resident - 1,
                ..Default::default()
            },
            &|| false,
            resident,
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("resident callback metadata"));
        assert_eq!(
            COPY_KEY.with(Cell::get),
            Some("animation frame descriptors")
        );
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        let e = animation_with_workspace(
            &[],
            DecodeLimits::default(),
            JxlResourceLimits {
                workspace_bytes: usize::MAX,
                ..Default::default()
            },
            &|| true,
            usize::MAX,
        )
        .err()
        .unwrap();
        assert!(e.is::<super::super::DecodeCancelled>());
    }
    #[test]
    fn callback_record_replacement_and_key_refusal_preserve_the_ledger() {
        let _reset = ResetFault;
        use super::super::animation::{callback_key, reserve_callback_metadata};
        let mut metadata = Vec::with_capacity(4);
        metadata.push(("x".into(), crate::metadata::DataMap::Raw(vec![1])));
        let old = super::super::container_metadata::record_bytes(metadata.capacity()).unwrap();
        let next = super::super::container_metadata::record_bytes(metadata.len() + 16).unwrap();
        let mut short = Workspace::new(old, old + next - 1).unwrap();
        COPY_KEY.with(|f| f.set(Some("animation callback metadata")));
        FAILED_BYTES.with(|f| f.set(0));
        assert!(reserve_callback_metadata(&mut metadata, &mut short).is_err());
        assert_eq!(FAILED_BYTES.with(Cell::get), 0);
        let mut exact = Workspace::new(old, old + next).unwrap();
        assert!(
            reserve_callback_metadata(&mut metadata, &mut exact)
                .unwrap_err()
                .is::<std::collections::TryReserveError>()
        );
        assert_eq!(metadata.capacity(), 4);
        assert_eq!(metadata.len(), 1);
        assert_eq!(exact.accounted(), old);
        reserve_callback_metadata(&mut metadata, &mut exact).unwrap();
        assert_eq!(exact.accounted(), next);
        assert_eq!(metadata[0].0, "x");
        let live = exact.accounted();
        COPY_KEY.with(|f| f.set(Some("test key")));
        FAILED_BYTES.with(|f| f.set(0));
        let e = callback_key("test key", &mut exact).unwrap_err();
        assert!(e.is::<std::collections::TryReserveError>());
        assert_eq!(exact.accounted(), live);
        assert_eq!(callback_key("test key", &mut exact).unwrap(), "test key");
        assert_eq!(exact.accounted(), live + 8);
    }
    #[test]
    fn each_fixed_animation_key_allocation_error_publishes_no_image() {
        let _reset = ResetFault;
        let input =
            include_bytes!("../tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");
        for key in [
            "ICC Profile",
            "JPEG XL Source ICC Profile",
            "JPEG XL Reconstruction ICC Profile",
            "JPEG XL Source Header",
            "JPEG XL Source Orientation",
            "JPEG XL Source Bits Per Sample",
            "JPEG XL Animation Ticks Numerator",
            "JPEG XL Animation Ticks Denominator",
            "coded_width",
            "coded_height",
            "output_width",
            "output_height",
            "JPEG XL Source Alpha Bits Per Sample",
            "orientation_applied",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let e = super::super::image_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
            )
            .err()
            .unwrap();
            assert!(e.is::<std::collections::TryReserveError>(), "{key}: {e}");
            assert_eq!(FAILED_BYTES.with(Cell::get), key.len());
            assert!(
                super::super::image_from_with_resource_limits(
                    input,
                    DecodeLimits::default(),
                    JxlResourceLimits::default()
                )
                .is_ok()
            );
        }
    }
    #[test]
    fn every_animation_delivery_reserve_refuses_without_publication() {
        let _reset = ResetFault;
        let input =
            include_bytes!("../tests/fixtures/review/jxl-animation-main5-icc16-variable.jxl");
        for key in [
            "animation delays",
            "animation prepared records",
            "animation prepared pixels",
            "animation prepared second pixels",
            "animation prepared later pixels",
            "animation callback metadata",
            "animation callback ICC",
        ] {
            COPY_KEY.with(|f| f.set(Some(key)));
            FAILED_BYTES.with(|f| f.set(0));
            let result = super::super::image_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default(),
            );
            assert!(
                result
                    .err()
                    .unwrap()
                    .is::<std::collections::TryReserveError>(),
                "{key}"
            );
            assert_eq!(COPY_KEY.with(Cell::get), None);
            assert!(FAILED_BYTES.with(Cell::get) > 0, "{key}");
        }
        assert!(
            super::super::image_from_with_resource_limits(
                input,
                DecodeLimits::default(),
                JxlResourceLimits::default()
            )
            .is_ok()
        );
    }
    #[cfg(feature = "image-buffer-ex")]
    #[test]
    fn ex_animation_storage_and_precision_allocation_failures_preserve_frame_state() {
        use crate::draw::*;
        let _reset = ResetFault;
        for (storage, failed_bytes) in [(PixelFormatEx::Rgba16, 280), (PixelFormatEx::Rgba8, 140)] {
            let mut image = ImageBufferEx::with_storage(storage, PrecisionConversion::Round);
            image
                .init_ex(
                    7,
                    5,
                    Some(InitOptionsEx {
                        source_format: PixelFormatEx::Rgba16,
                        source_stride: None,
                        legacy: Some(InitOptions {
                            loop_count: 1,
                            background: None,
                            animation: true,
                        }),
                    }),
                )
                .unwrap();
            if storage == PixelFormatEx::Rgba16 {
                FAIL_BYTES.with(|f| f.set(failed_bytes));
                assert!(
                    image
                        .next(Some(NextOptions::wait(50)))
                        .err()
                        .unwrap()
                        .is::<std::collections::TryReserveError>()
                );
                assert!(image.animation().unwrap().is_empty());
            } else {
                image.next(Some(NextOptions::wait(50))).unwrap();
                FAIL_BYTES.with(|f| f.set(failed_bytes));
                assert!(
                    image
                        .draw(0, 0, 7, 5, &[128; 280], None)
                        .err()
                        .unwrap()
                        .is::<std::collections::TryReserveError>()
                );
                assert!(
                    image.animation().unwrap()[0]
                        .pixels
                        .bytes()
                        .iter()
                        .all(|&v| v == 0)
                );
            }
            assert_eq!(FAIL_BYTES.with(Cell::get), 0);
        }
        // Source RGBA16 can exceed the per-frame workspace even where RGBA8
        // geometry and the independent animation-byte quota both fit.
        let mut image =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
        let mut reader = bin_rs::reader::BytesReader::new(&[0u8; 8]);
        let error = crate::decode_guard::run_ex(
            &mut reader,
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut image,
            },
            DecodeLimits {
                expanded_bytes: 100,
                animation_bytes: 1000,
                ..Default::default()
            },
            |_, o| {
                o.drawer.init_ex(
                    1,
                    1,
                    Some(InitOptionsEx {
                        source_format: PixelFormatEx::Rgba16,
                        source_stride: None,
                        legacy: Some(InitOptions {
                            animation: true,
                            loop_count: 1,
                            background: None,
                        }),
                    }),
                )?;
                o.drawer.next(Some(NextOptions {
                    image_rect: Some(ImageRect {
                        start_x: 0,
                        start_y: 0,
                        width: 4,
                        height: 4,
                    }),
                    ..NextOptions::new()
                }))?;
                Ok(None)
            },
        )
        .err()
        .unwrap();
        assert!(error.to_string().contains("animation source frame"));
    }
    #[test]
    fn finite_header_admission_reserves_all_process_palette_slots() {
        let legacy = JxlResourceLimits::legacy();
        let finite = JxlResourceLimits::default();
        let per_slot = 3 * (143 + 64 + 125) * 4 + 3 * std::mem::size_of::<usize>();
        assert_eq!(IMPLICIT_PALETTE_CACHE_BYTES, 25 * per_slot);
        assert_eq!(
            IMPLICIT_PALETTE_CACHE_BYTES,
            if usize::BITS == 64 { 100200 } else { 99900 }
        );
        for bytes in [0, 1024, 65536] {
            assert_eq!(
                header_workspace(bytes, legacy).unwrap(),
                1024 * 1024 + 2 * bytes
            );
            assert_eq!(
                header_workspace(bytes, finite).unwrap() - header_workspace(bytes, legacy).unwrap(),
                IMPLICIT_PALETTE_CACHE_BYTES
            );
        }
        assert!(header_workspace(usize::MAX, finite).is_err());
    }
}
