//! Codec-free fallible ownership accounting for high-resolution construction.
//!
//! AVIF mapping imports this module as an adapter, while the ledger itself
//! intentionally has no codec-specific types or parsing responsibilities.

#[cfg(all(test, miri))]
use std::mem::align_of;
use std::mem::size_of;

use crate::highres::{ProcessingError, ResourceLimits};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static CONSTRUCTION_FAILPOINT: Cell<usize> = const { Cell::new(usize::MAX) };
    // Installed only by bounded tests; this observes the real owner drops at
    // the conversion restore boundary without adding another allocator.
    static RESTORE_BOUNDARY_OBSERVER: Cell<Option<fn()>> = const { Cell::new(None) };
    // Miri cannot safely observe Windows System allocator deallocations through
    // a wrapping GlobalAlloc. The bounded Miri seam records only the captured
    // candidate identity after Vec ownership has been destroyed.
    #[cfg(miri)]
    static CANDIDATE_DROP_OBSERVER: Cell<Option<fn(usize, usize, usize)>> =
        const { Cell::new(None) };
}

pub(crate) struct ConstructionLedger {
    pub(crate) frame_bytes: usize,
    pub(crate) live_bytes: usize,
    metadata_bytes: usize,
    pending_frame_bytes: usize,
    pending_metadata_bytes: usize,
    max_frame_bytes: usize,
    max_live_bytes: usize,
    max_metadata_bytes: usize,
    max_icc_bytes: usize,
}

impl ConstructionLedger {
    #[cfg(all(feature = "avif", test))]
    pub(crate) fn new(
        retained_bytes: usize,
        native_outer_bytes: usize,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        Self::new_with_ownership(retained_bytes, retained_bytes, native_outer_bytes, limits)
    }

    /// Seed the ledger with the three distinct ownership classes that can
    /// coexist while mapping a decoded frame. `frame_bytes` is retained by
    /// the eventual frame and is therefore part of the live total;
    /// `metadata_bytes` is the same retained owner as seen by the metadata
    /// budget; `live_only_bytes` is borrowed/native storage. Only the native
    /// outer allocation is destroyed and released before the active colour
    /// clone; borrowed Rich metadata remains live until return.
    pub(crate) fn new_with_ownership(
        frame_bytes: usize,
        metadata_bytes: usize,
        live_only_bytes: usize,
        limits: &ResourceLimits,
    ) -> Result<Self, ProcessingError> {
        let live_bytes = frame_bytes.checked_add(live_only_bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction live bytes overflow".into())
        })?;
        let ledger = Self {
            frame_bytes,
            live_bytes,
            metadata_bytes,
            pending_frame_bytes: 0,
            pending_metadata_bytes: 0,
            max_frame_bytes: limits.max_frame_bytes,
            max_live_bytes: limits.max_total_live_decoded_bytes,
            max_metadata_bytes: limits.max_metadata_bytes,
            max_icc_bytes: limits.max_icc_bytes,
        };
        ledger.check()
    }

    fn check(self) -> Result<Self, ProcessingError> {
        if self.frame_bytes > self.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds frame budget".into(),
            ));
        }
        if self.live_bytes > self.max_live_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds live budget".into(),
            ));
        }
        if self.metadata_bytes > self.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds metadata budget".into(),
            ));
        }
        Ok(self)
    }

    #[cfg(any(feature = "avif", test))]
    pub(crate) fn charge(&mut self, bytes: usize) -> Result<(), ProcessingError> {
        let (frame_bytes, live_bytes) = self.checked_add(bytes)?;
        self.frame_bytes = frame_bytes;
        self.live_bytes = live_bytes;
        Ok(())
    }

    pub(crate) fn reserve_output(
        &mut self,
        frame_bytes: usize,
        metadata_bytes: usize,
    ) -> Result<(), ProcessingError> {
        let pending_metadata = self
            .pending_metadata_bytes
            .checked_add(metadata_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("pending metadata overflows".into()))?;
        let pending_frame = self
            .pending_frame_bytes
            .checked_add(frame_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("pending frame overflows".into()))?;
        self.checked_add(pending_frame)?;
        let metadata = self
            .metadata_bytes
            .checked_add(pending_metadata)
            .ok_or_else(|| ProcessingError::ResourceLimit("pending metadata overflows".into()))?;
        if metadata > self.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "pending metadata exceeds metadata budget".into(),
            ));
        }
        self.pending_frame_bytes = pending_frame;
        self.pending_metadata_bytes = pending_metadata;
        Ok(())
    }

    pub(crate) fn commit_pending_inline(
        &mut self,
        frame_bytes: usize,
        metadata_bytes: usize,
    ) -> Result<(), ProcessingError> {
        if frame_bytes > self.pending_frame_bytes || metadata_bytes > self.pending_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "inline output owner is not planned".into(),
            ));
        }
        let remaining_frame = self.pending_frame_bytes - frame_bytes;
        let remaining_metadata = self.pending_metadata_bytes - metadata_bytes;
        let next_frame = self
            .frame_bytes
            .checked_add(frame_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("inline frame bytes overflow".into()))?;
        let next_live = self
            .live_bytes
            .checked_add(frame_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("inline live bytes overflow".into()))?;
        let next_metadata = self
            .metadata_bytes
            .checked_add(metadata_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("inline metadata bytes overflow".into())
            })?;
        if next_frame
            .checked_add(remaining_frame)
            .is_none_or(|total| total > self.max_frame_bytes)
            || next_live
                .checked_add(remaining_frame)
                .is_none_or(|total| total > self.max_live_bytes)
            || next_metadata
                .checked_add(remaining_metadata)
                .is_none_or(|total| total > self.max_metadata_bytes)
        {
            return Err(ProcessingError::ResourceLimit(
                "inline output owner exceeds construction limits".into(),
            ));
        }
        self.frame_bytes = next_frame;
        self.live_bytes = next_live;
        self.metadata_bytes = next_metadata;
        self.pending_frame_bytes = remaining_frame;
        self.pending_metadata_bytes = remaining_metadata;
        Ok(())
    }

    pub(crate) fn checkpoint(&self) -> ConstructionCheckpoint {
        ConstructionCheckpoint {
            frame_bytes: self.frame_bytes,
            live_bytes: self.live_bytes,
            metadata_bytes: self.metadata_bytes,
            pending_frame_bytes: self.pending_frame_bytes,
            pending_metadata_bytes: self.pending_metadata_bytes,
        }
    }

    pub(crate) fn restore(&mut self, checkpoint: ConstructionCheckpoint) {
        self.frame_bytes = checkpoint.frame_bytes;
        self.live_bytes = checkpoint.live_bytes;
        self.metadata_bytes = checkpoint.metadata_bytes;
        self.pending_frame_bytes = checkpoint.pending_frame_bytes;
        self.pending_metadata_bytes = checkpoint.pending_metadata_bytes;
    }

    pub(crate) fn pending_frame_bytes(&self) -> usize {
        self.pending_frame_bytes
    }

    pub(crate) fn pending_metadata_bytes(&self) -> usize {
        self.pending_metadata_bytes
    }

    fn checked_add(&self, bytes: usize) -> Result<(usize, usize), ProcessingError> {
        let frame_bytes = self
            .frame_bytes
            .checked_add(bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("construction bytes overflow".into()))?;
        let live_bytes = self.live_bytes.checked_add(bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction live bytes overflow".into())
        })?;
        if frame_bytes > self.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds frame budget".into(),
            ));
        }
        if live_bytes > self.max_live_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds live budget".into(),
            ));
        }
        Ok((frame_bytes, live_bytes))
    }

    #[cfg(any(feature = "avif", test))]
    pub(crate) fn reconcile(
        &mut self,
        requested: usize,
        actual: usize,
    ) -> Result<(), ProcessingError> {
        if actual >= requested {
            self.charge(actual - requested)
        } else {
            let released = requested - actual;
            self.frame_bytes = self.frame_bytes.checked_sub(released).ok_or_else(|| {
                ProcessingError::ResourceLimit("construction frame debit underflow".into())
            })?;
            self.live_bytes = self.live_bytes.checked_sub(released).ok_or_else(|| {
                ProcessingError::ResourceLimit("construction live debit underflow".into())
            })?;
            Ok(())
        }
    }

    #[cfg(feature = "avif")]
    pub(crate) fn rollback(&mut self, bytes: usize) -> Result<(), ProcessingError> {
        self.frame_bytes = self.frame_bytes.checked_sub(bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction frame debit underflow".into())
        })?;
        self.live_bytes = self.live_bytes.checked_sub(bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction live debit underflow".into())
        })?;
        Ok(())
    }

    #[cfg(feature = "avif")]
    pub(crate) fn release_live(&mut self, bytes: usize) -> Result<(), ProcessingError> {
        self.live_bytes = self.live_bytes.checked_sub(bytes).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction live release underflow".into())
        })?;
        Ok(())
    }

    #[cfg(feature = "avif")]
    pub(crate) fn charge_metadata(&mut self, bytes: usize) -> Result<(), ProcessingError> {
        self.charge(bytes)?;
        match self.metadata_bytes.checked_add(bytes) {
            Some(value) if value <= self.max_metadata_bytes => {
                self.metadata_bytes = value;
                Ok(())
            }
            _ => {
                self.rollback(bytes)?;
                Err(ProcessingError::ResourceLimit(
                    "construction exceeds metadata budget".into(),
                ))
            }
        }
    }

    pub(crate) fn metadata_bytes_for_test(&self) -> usize {
        self.metadata_bytes
    }

    pub(crate) fn max_icc_bytes(&self) -> usize {
        self.max_icc_bytes
    }

    #[cfg(all(feature = "avif", test))]
    pub(crate) fn try_new_metadata_vec_with<T, F>(
        &mut self,
        count: usize,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        self.try_new_metadata_vec_with_limit(count, None, make_candidate)
    }

    pub(crate) fn try_new_metadata_vec_with_limit<T, F>(
        &mut self,
        count: usize,
        owner_limit: Option<usize>,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        if self.pending_frame_bytes != 0 || self.pending_metadata_bytes != 0 {
            return self.try_new_pending_vec(count, true, owner_limit, make_candidate);
        }
        let requested_bytes = count.checked_mul(size_of::<T>()).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction metadata vector bytes overflow".into())
        })?;
        if owner_limit.is_some_and(|limit| requested_bytes > limit) {
            return Err(ProcessingError::ResourceLimit(
                "metadata owner exceeds resource limit".into(),
            ));
        }
        self.checked_add(requested_bytes)?;
        let requested_metadata = self
            .metadata_bytes
            .checked_add(requested_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("construction metadata bytes overflow".into())
            })?;
        if requested_metadata > self.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds metadata budget".into(),
            ));
        }

        let candidate = make_candidate(count)?;
        if !candidate.is_empty() || candidate.capacity() < count {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata candidate shape is invalid".into(),
            ));
        }
        let Some(actual_bytes) = candidate.capacity().checked_mul(size_of::<T>()) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "construction metadata bytes overflow".into(),
            ));
        };
        if owner_limit.is_some_and(|limit| actual_bytes > limit) {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata owner exceeds resource limit".into(),
            ));
        }
        let Ok((actual_frame, actual_live)) = self.checked_add(actual_bytes) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "construction metadata allocation exceeds limits".into(),
            ));
        };
        let Some(actual_metadata) = self.metadata_bytes.checked_add(actual_bytes) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "construction metadata bytes overflow".into(),
            ));
        };
        if actual_metadata > self.max_metadata_bytes {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds metadata budget".into(),
            ));
        }
        self.frame_bytes = actual_frame;
        self.live_bytes = actual_live;
        self.metadata_bytes = actual_metadata;
        Ok(candidate)
    }

    /// Replace a metadata outer vector transactionally.  The old owner stays
    /// untouched while the fresh candidate is admitted and allocated; this is
    /// important for nested metadata whose payload pointers must survive every
    /// failed growth attempt.
    #[cfg(feature = "avif")]
    pub(crate) fn try_grow_metadata_vec<T, F>(
        &mut self,
        owner: &mut Vec<T>,
        additional: usize,
        inline_delta: usize,
        make_candidate: F,
    ) -> Result<(), ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        let element_size = size_of::<T>();
        let old_len = owner.len();
        let new_len = old_len.checked_add(additional).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata vector length overflows".into())
        })?;
        let old_bytes = owner.capacity().checked_mul(element_size).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata vector bytes overflow".into())
        })?;
        let requested_bytes = new_len.checked_mul(element_size).ok_or_else(|| {
            ProcessingError::ResourceLimit("metadata vector bytes overflow".into())
        })?;
        let spare = owner.capacity().saturating_sub(old_len);
        if additional <= spare {
            if inline_delta != 0 {
                self.charge_metadata(inline_delta)?;
            }
            return Ok(());
        }

        // The final owner replaces the old outer allocation.  Candidate
        // admission is checked separately below while both owners coexist.
        let final_frame = self
            .frame_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(requested_bytes))
            .and_then(|value| value.checked_add(inline_delta))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata bytes overflow".into()))?;
        let final_metadata = self
            .metadata_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(requested_bytes))
            .and_then(|value| value.checked_add(inline_delta))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata bytes overflow".into()))?;
        let candidate_live = self
            .live_bytes
            .checked_add(requested_bytes)
            .and_then(|value| value.checked_add(inline_delta))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata live bytes overflow".into()))?;
        let post_live = self
            .live_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(requested_bytes))
            .and_then(|value| value.checked_add(inline_delta))
            .ok_or_else(|| ProcessingError::ResourceLimit("metadata live bytes overflow".into()))?;
        self.check_replacement_limits(final_frame, candidate_live, post_live, final_metadata)?;

        let mut candidate = make_candidate(new_len)?;
        if !candidate.is_empty() || candidate.capacity() < new_len {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata candidate shape is invalid".into(),
            ));
        }
        let Some(actual_bytes) = candidate.capacity().checked_mul(element_size) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata candidate bytes overflow".into(),
            ));
        };
        let Some(actual_frame) = self
            .frame_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(actual_bytes))
            .and_then(|value| value.checked_add(inline_delta))
        else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata bytes overflow".into(),
            ));
        };
        let Some(actual_metadata) = self
            .metadata_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(actual_bytes))
            .and_then(|value| value.checked_add(inline_delta))
        else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata bytes overflow".into(),
            ));
        };
        let Some(actual_live) = self
            .live_bytes
            .checked_add(actual_bytes)
            .and_then(|value| value.checked_add(inline_delta))
        else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata live bytes overflow".into(),
            ));
        };
        let Some(actual_post_live) = self
            .live_bytes
            .checked_sub(old_bytes)
            .and_then(|value| value.checked_add(actual_bytes))
            .and_then(|value| value.checked_add(inline_delta))
        else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "metadata live bytes overflow".into(),
            ));
        };
        if let Err(error) = self.check_replacement_limits(
            actual_frame,
            actual_live,
            actual_post_live,
            actual_metadata,
        ) {
            drop_candidate(candidate);
            return Err(error);
        }

        candidate.append(owner);
        *owner = candidate;
        self.frame_bytes = actual_frame;
        self.live_bytes = actual_post_live;
        self.metadata_bytes = actual_metadata;
        Ok(())
    }

    #[cfg(feature = "avif")]
    fn check_replacement_limits(
        &self,
        final_frame: usize,
        candidate_live: usize,
        post_live: usize,
        final_metadata: usize,
    ) -> Result<(), ProcessingError> {
        if final_frame > self.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds frame budget".into(),
            ));
        }
        if candidate_live > self.max_live_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds live budget".into(),
            ));
        }
        if post_live > self.max_live_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds live budget".into(),
            ));
        }
        if final_metadata > self.max_metadata_bytes {
            return Err(ProcessingError::ResourceLimit(
                "construction exceeds metadata budget".into(),
            ));
        }
        Ok(())
    }

    #[cfg(all(feature = "avif", test))]
    pub(crate) fn try_copy_icc(&mut self, source: &[u8]) -> Result<Vec<u8>, ProcessingError> {
        self.try_copy_icc_with(source, try_new_metadata_candidate::<u8>)
    }

    #[cfg(all(feature = "avif", test))]
    pub(crate) fn try_copy_icc_with<F>(
        &mut self,
        source: &[u8],
        make_candidate: F,
    ) -> Result<Vec<u8>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<u8>, ProcessingError>,
    {
        let mut copy = self.try_new_metadata_vec_with_limit(
            source.len(),
            Some(self.max_icc_bytes),
            make_candidate,
        )?;
        copy.extend_from_slice(source);
        Ok(copy)
    }

    #[cfg(any(feature = "avif", test))]
    pub(crate) fn try_new_vec<T>(&mut self, count: usize) -> Result<Vec<T>, ProcessingError> {
        if self.pending_frame_bytes != 0 {
            return self.try_new_pending_vec(count, false, None, |count| {
                #[cfg(test)]
                construction_failpoint_checkpoint()?;
                let mut values = Vec::new();
                values.try_reserve_exact(count).map_err(|error| {
                    ProcessingError::Allocation(format!(
                        "construction vector allocation failed: {error}"
                    ))
                })?;
                Ok(values)
            });
        }
        let requested_bytes = count.checked_mul(size_of::<T>()).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction vector bytes overflow".into())
        })?;
        let previous_frame_bytes = self.frame_bytes;
        let previous_live_bytes = self.live_bytes;
        let (provisional_frame_bytes, provisional_live_bytes) =
            self.checked_add(requested_bytes)?;
        self.frame_bytes = provisional_frame_bytes;
        self.live_bytes = provisional_live_bytes;

        #[cfg(test)]
        if let Err(error) = construction_failpoint_checkpoint() {
            self.frame_bytes = previous_frame_bytes;
            self.live_bytes = previous_live_bytes;
            return Err(error);
        }

        let mut values = Vec::<T>::new();
        if let Err(error) = values.try_reserve_exact(count) {
            self.frame_bytes = previous_frame_bytes;
            self.live_bytes = previous_live_bytes;
            return Err(ProcessingError::Allocation(format!(
                "construction vector allocation failed: {error}"
            )));
        }
        let actual_bytes = values
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or_else(|| {
                self.frame_bytes = previous_frame_bytes;
                self.live_bytes = previous_live_bytes;
                ProcessingError::ResourceLimit("construction vector bytes overflow".into())
            })?;
        match self.reconcile(requested_bytes, actual_bytes) {
            Ok(()) => Ok(values),
            Err(error) => {
                self.frame_bytes = previous_frame_bytes;
                self.live_bytes = previous_live_bytes;
                Err(error)
            }
        }
    }

    pub(crate) fn try_new_reserved_vec_with<T, F>(
        &mut self,
        count: usize,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        self.try_new_pending_vec(count, false, None, make_candidate)
    }

    pub(crate) fn try_new_reserved_vec_with_limit<T, F>(
        &mut self,
        count: usize,
        owner_limit: usize,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        self.try_new_pending_vec(count, false, Some(owner_limit), make_candidate)
    }

    fn try_new_pending_vec<T, F>(
        &mut self,
        count: usize,
        metadata: bool,
        owner_limit: Option<usize>,
        make_candidate: F,
    ) -> Result<Vec<T>, ProcessingError>
    where
        F: FnOnce(usize) -> Result<Vec<T>, ProcessingError>,
    {
        let requested_bytes = count.checked_mul(size_of::<T>()).ok_or_else(|| {
            ProcessingError::ResourceLimit("construction reserved vector bytes overflow".into())
        })?;
        if owner_limit.is_some_and(|limit| requested_bytes > limit) {
            return Err(ProcessingError::ResourceLimit(
                "reserved owner exceeds resource limit".into(),
            ));
        }
        if requested_bytes > self.pending_frame_bytes
            || (metadata && requested_bytes > self.pending_metadata_bytes)
        {
            return Err(ProcessingError::ResourceLimit(
                "reserved output owner is not planned".into(),
            ));
        }
        let remaining_frame = self.pending_frame_bytes - requested_bytes;
        let remaining_metadata = if metadata {
            self.pending_metadata_bytes - requested_bytes
        } else {
            self.pending_metadata_bytes
        };
        let candidate = make_candidate(count)?;
        if !candidate.is_empty() || candidate.capacity() < count {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved candidate shape is invalid".into(),
            ));
        }
        let Some(actual_bytes) = candidate.capacity().checked_mul(size_of::<T>()) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved candidate bytes overflow".into(),
            ));
        };
        if owner_limit.is_some_and(|limit| actual_bytes > limit) {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved owner exceeds resource limit".into(),
            ));
        }
        let Some(next_frame) = self.frame_bytes.checked_add(actual_bytes) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved frame bytes overflow".into(),
            ));
        };
        let Some(next_live) = self.live_bytes.checked_add(actual_bytes) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved live bytes overflow".into(),
            ));
        };
        let Some(frame_with_pending) = next_frame.checked_add(remaining_frame) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved frame bytes overflow".into(),
            ));
        };
        let Some(live_with_pending) = next_live.checked_add(remaining_frame) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved live bytes overflow".into(),
            ));
        };
        if frame_with_pending > self.max_frame_bytes || live_with_pending > self.max_live_bytes {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved output exceeds construction limits".into(),
            ));
        }
        let next_metadata = if metadata {
            let Some(value) = self.metadata_bytes.checked_add(actual_bytes) else {
                drop_candidate(candidate);
                return Err(ProcessingError::ResourceLimit(
                    "reserved metadata bytes overflow".into(),
                ));
            };
            value
        } else {
            self.metadata_bytes
        };
        let Some(metadata_with_pending) = next_metadata.checked_add(remaining_metadata) else {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved metadata bytes overflow".into(),
            ));
        };
        if metadata_with_pending > self.max_metadata_bytes {
            drop_candidate(candidate);
            return Err(ProcessingError::ResourceLimit(
                "reserved output exceeds metadata budget".into(),
            ));
        }
        self.frame_bytes = next_frame;
        self.live_bytes = next_live;
        self.metadata_bytes = next_metadata;
        self.pending_frame_bytes = remaining_frame;
        self.pending_metadata_bytes = remaining_metadata;
        Ok(candidate)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ConstructionCheckpoint {
    frame_bytes: usize,
    live_bytes: usize,
    metadata_bytes: usize,
    pending_frame_bytes: usize,
    pending_metadata_bytes: usize,
}

#[cfg(all(test, miri))]
fn drop_candidate<T>(candidate: Vec<T>) {
    // Capture all identity fields while the Vec still owns the allocation.
    // The callback is deliberately invoked only after drop and never uses the
    // pointer as an address; this is the Miri-only replacement for observing
    // the raw allocator's deallocation hook.
    let pointer = candidate.as_ptr() as usize;
    let bytes = candidate
        .capacity()
        .checked_mul(size_of::<T>())
        .unwrap_or(usize::MAX);
    let align = align_of::<T>();
    drop(candidate);
    observe_candidate_drop(pointer, bytes, align);
}

#[cfg(all(test, not(miri)))]
fn drop_candidate<T>(candidate: Vec<T>) {
    drop(candidate);
}

#[cfg(not(test))]
fn drop_candidate<T>(candidate: Vec<T>) {
    drop(candidate);
}

#[cfg(feature = "avif")]
pub(super) fn try_new_metadata_candidate<T>(count: usize) -> Result<Vec<T>, ProcessingError> {
    #[cfg(test)]
    construction_failpoint_checkpoint()?;
    let mut candidate = Vec::new();
    candidate.try_reserve_exact(count).map_err(|error| {
        ProcessingError::Allocation(format!("metadata candidate allocation failed: {error}"))
    })?;
    Ok(candidate)
}

#[cfg(all(feature = "avif", test))]
pub(super) struct ConstructionFailpointGuard {
    previous: usize,
}

#[cfg(all(feature = "avif", test))]
pub(super) fn construction_failpoint(remaining: usize) -> ConstructionFailpointGuard {
    let previous = CONSTRUCTION_FAILPOINT.with(|failpoint| {
        let previous = failpoint.get();
        failpoint.set(remaining);
        previous
    });
    ConstructionFailpointGuard { previous }
}

#[cfg(test)]
pub(super) fn construction_failpoint_checkpoint() -> Result<(), ProcessingError> {
    CONSTRUCTION_FAILPOINT.with(|failpoint| {
        let remaining = failpoint.get();
        if remaining == 0 {
            return Err(ProcessingError::Allocation(
                "construction allocation failpoint".into(),
            ));
        }
        if remaining != usize::MAX {
            failpoint.set(remaining - 1);
        }
        Ok(())
    })
}

#[cfg(test)]
pub(super) fn swap_restore_boundary_observer(observer: Option<fn()>) -> Option<fn()> {
    RESTORE_BOUNDARY_OBSERVER.with(|slot| {
        let previous = slot.get();
        slot.set(observer);
        previous
    })
}

#[cfg(test)]
pub(super) fn observe_restore_boundary() {
    RESTORE_BOUNDARY_OBSERVER.with(|slot| {
        if let Some(observer) = slot.get() {
            observer();
        }
    });
}

#[cfg(all(test, miri))]
pub(super) fn swap_candidate_drop_observer(
    observer: Option<fn(usize, usize, usize)>,
) -> Option<fn(usize, usize, usize)> {
    CANDIDATE_DROP_OBSERVER.with(|slot| {
        let previous = slot.get();
        slot.set(observer);
        previous
    })
}

#[cfg(all(test, miri))]
fn observe_candidate_drop(pointer: usize, bytes: usize, align: usize) {
    CANDIDATE_DROP_OBSERVER.with(|slot| {
        if let Some(observer) = slot.get() {
            observer(pointer, bytes, align);
        }
    });
}

#[cfg(all(feature = "avif", test))]
impl Drop for ConstructionFailpointGuard {
    fn drop(&mut self) {
        CONSTRUCTION_FAILPOINT.with(|failpoint| failpoint.set(self.previous));
    }
}
