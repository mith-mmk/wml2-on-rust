//! Codec-free fallible ownership helpers for high-resolution metadata.

#[cfg(feature = "avif")]
use super::ColorInformationSet;
#[cfg(feature = "avif")]
use super::ColorProvenance;
use super::ProcessingError;
#[cfg(feature = "avif")]
use super::UnknownColorInformation;
#[cfg(feature = "avif")]
use super::allocation::ConstructionLedger;
#[cfg(feature = "avif")]
use super::allocation::try_new_metadata_candidate;
use super::output_plan::{OwnerElement, OwnerKey, OwnerSink};
pub(super) struct OrdinaryOwnerSink;

fn ordinary_error(key: OwnerKey) -> &'static str {
    match key {
        OwnerKey::Provenance => "colour provenance allocation failed",
        OwnerKey::IccProfile => "ICC profile allocation failed",
        OwnerKey::UnknownOuter => "unknown colour allocation failed",
        OwnerKey::UnknownPayload(_) => "unknown colour payload allocation failed",
        _ => "metadata copy allocation failed",
    }
}

impl OwnerSink for OrdinaryOwnerSink {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| ProcessingError::Allocation(ordinary_error(key).into()))?;
        Ok(values)
    }

    fn commit_inline(&mut self, _key: OwnerKey, _bytes: usize) -> Result<(), ProcessingError> {
        Ok(())
    }
}

#[cfg(feature = "avif")]
pub(super) struct LedgerOwnerSink<'a> {
    pub(super) ledger: &'a mut ConstructionLedger,
}

#[cfg(feature = "avif")]
impl OwnerSink for LedgerOwnerSink<'_> {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        let limit = (key == OwnerKey::IccProfile).then(|| self.ledger.max_icc_bytes());
        self.ledger
            .try_new_metadata_vec_with_limit(count, limit, |count| {
                let mut values = Vec::new();
                values.try_reserve_exact(count).map_err(|_| {
                    ProcessingError::Allocation("metadata copy allocation failed".into())
                })?;
                Ok(values)
            })
    }

    fn commit_inline(&mut self, _key: OwnerKey, _bytes: usize) -> Result<(), ProcessingError> {
        Ok(())
    }
}

#[cfg(feature = "avif")]
pub(super) struct OptionalOwnerSink<'a> {
    pub(super) ledger: Option<&'a mut ConstructionLedger>,
}

#[cfg(feature = "avif")]
impl OwnerSink for OptionalOwnerSink<'_> {
    fn fresh<T: OwnerElement>(
        &mut self,
        key: OwnerKey,
        count: usize,
    ) -> Result<Vec<T>, ProcessingError> {
        if let Some(ledger) = self.ledger.as_mut() {
            let limit = (key == OwnerKey::IccProfile).then(|| ledger.max_icc_bytes());
            ledger.try_new_metadata_vec_with_limit(count, limit, |count| {
                let mut values = Vec::new();
                values
                    .try_reserve_exact(count)
                    .map_err(|_| ProcessingError::Allocation(ordinary_error(key).into()))?;
                Ok(values)
            })
        } else {
            OrdinaryOwnerSink.fresh(key, count)
        }
    }

    fn commit_inline(&mut self, _key: OwnerKey, _bytes: usize) -> Result<(), ProcessingError> {
        Ok(())
    }
}

#[cfg(feature = "avif")]
pub(super) fn copy_owner_with_ledger<T: Copy + OwnerElement>(
    key: OwnerKey,
    ledger: &mut Option<&mut ConstructionLedger>,
    source: &[T],
) -> Result<Vec<T>, ProcessingError> {
    let mut sink = OptionalOwnerSink {
        ledger: ledger.take(),
    };
    let result = sink.fresh(key, source.len());
    *ledger = sink.ledger;
    let mut copy = result?;
    copy.extend_from_slice(source);
    Ok(copy)
}

#[cfg(feature = "avif")]
pub(super) fn fresh_owner_with_ledger<T: OwnerElement>(
    key: OwnerKey,
    ledger: &mut Option<&mut ConstructionLedger>,
    count: usize,
) -> Result<Vec<T>, ProcessingError> {
    let mut sink = OptionalOwnerSink {
        ledger: ledger.take(),
    };
    let result = sink.fresh(key, count);
    *ledger = sink.ledger;
    result
}

/// Clone a complete colour-information set through the shared construction
/// ledger.  This helper is codec-free; AVIF only supplies the source mapping.
#[cfg(feature = "avif")]
pub(super) fn clone_color_information_with_ledger(
    source: &ColorInformationSet,
    ledger: &mut ConstructionLedger,
) -> Result<ColorInformationSet, ProcessingError> {
    source.try_clone_owned_with_ledger(ledger)
}

#[cfg(feature = "avif")]
pub(super) fn reserve_provenance_with_ledger(
    colors: &mut ColorInformationSet,
    ledger: &mut Option<&mut ConstructionLedger>,
) -> Result<(), ProcessingError> {
    if let Some(ledger) = ledger.as_mut() {
        return ledger.try_grow_metadata_vec(
            colors.provenance_mut_bridge(),
            1,
            0,
            try_new_metadata_candidate::<ColorProvenance>,
        );
    }
    colors
        .try_reserve_provenance(1)
        .map_err(ProcessingError::from)
}

#[cfg(feature = "avif")]
pub(super) fn reserve_unknown_with_ledger(
    colors: &mut ColorInformationSet,
    additional: usize,
    ledger: &mut Option<&mut ConstructionLedger>,
) -> Result<(), ProcessingError> {
    if additional == 0 {
        return Ok(());
    }
    if let Some(ledger) = ledger.as_mut() {
        return ledger.try_grow_metadata_vec(
            colors.unknown_colr_mut_bridge(),
            additional,
            0,
            try_new_metadata_candidate::<UnknownColorInformation>,
        );
    }
    colors.try_reserve_unknown_colr_bridge(additional)
}
