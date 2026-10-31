//! Caller-tightened admission and adapter-owned live storage accounting.
#[cfg(feature = "color-management")]
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
    #[cfg(feature = "color-management")]
    pub(super) fn release(&mut self, bytes: usize) -> Result<(), Error> {
        self.live = self
            .live
            .checked_sub(bytes)
            .ok_or_else(|| invalid("JPEG XL workspace release mismatch"))?;
        Ok(())
    }
    #[cfg(feature = "color-management")]
    pub(super) fn remaining(&self) -> usize {
        self.limit - self.live
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
mod tests {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    struct FailOutput;
    thread_local! { static FAIL_256: Cell<bool> = const { Cell::new(false) }; }
    fn refused(bytes: usize) -> bool {
        bytes == 256
            && FAIL_256
                .try_with(|flag| flag.replace(false))
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
    fn runtime_output_reservation_propagates_allocator_refusal_and_checked_size() {
        FAIL_256.with(|f| f.set(true));
        let result = super::super::aligned_output(256);
        assert!(result.is_err());
        assert!(!FAIL_256.with(Cell::get)); // the actual allocation was attempted
        assert_eq!(super::super::aligned_output(256).unwrap().len(), 128);
        assert!(super::super::aligned_output(usize::MAX).is_err());
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
}
