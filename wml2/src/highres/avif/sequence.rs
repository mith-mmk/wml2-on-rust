//! Transactional native AVIS sequence access for high-resolution consumers.
//!
//! This module deliberately does not introduce a drawing callback. Each
//! native frame is mapped only after the strict codec preparation has been
//! converted into a commit token, so a mapping or resource-limit error leaves
//! the sequence cursor and decoder state retryable.

use super::{DecodeError, NativeDecodeLimits, consume_native_frame_with_external_live};
use crate::highres::{FrameTiming, ImageFrame, PixelAspectRatio, ResourceLimits};

/// Sequence-level information retained without narrowing container integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvifSequenceInformation {
    frame_count: u64,
    timescale: u64,
    duration_in_timescales: u64,
    repetition: avif_codec::AvifRepetitionCount,
}

impl AvifSequenceInformation {
    pub const fn frame_count(self) -> u64 {
        self.frame_count
    }

    pub const fn timescale(self) -> u64 {
        self.timescale
    }

    pub const fn duration_in_timescales(self) -> u64 {
        self.duration_in_timescales
    }

    pub const fn repetition(self) -> avif_codec::AvifRepetitionCount {
        self.repetition
    }
}

/// One native AVIS frame and its zero-based sequence index.
#[derive(Debug, PartialEq)]
pub struct AvifSequenceFrame {
    index: u64,
    frame: ImageFrame,
}

impl AvifSequenceFrame {
    pub const fn index(&self) -> u64 {
        self.index
    }

    pub const fn frame(&self) -> &ImageFrame {
        &self.frame
    }

    pub fn into_frame(self) -> ImageFrame {
        self.frame
    }
}

/// Stateful, transactional native AVIS reader for `highres` consumers.
///
/// The legacy AVIF sequence API and WML2 draw callbacks are not involved. A
/// failed mapping does not consume a frame; callers can retry `next_frame`.
pub struct AvifSequenceReader {
    decoder: avif_codec::StrictAvifSequenceDecoder,
    resource_limits: ResourceLimits,
    information: AvifSequenceInformation,
}

impl AvifSequenceReader {
    pub(super) fn effective_native_limits(
        native: NativeDecodeLimits,
        resource: ResourceLimits,
    ) -> Result<NativeDecodeLimits, DecodeError> {
        // A zero aggregate budget cannot admit even the native retained
        // container owners.  Report this as a highres resource error before
        // invoking the codec, rather than exposing a codec parameter error.
        if resource.max_total_live_decoded_bytes == 0 {
            return Err(DecodeError::Processing(
                super::super::ProcessingError::ResourceLimit(
                    "AVIS live allocation limit must be positive".into(),
                ),
            ));
        }

        // Only fields with the same meaning in both layers are projected.
        // Channels, planes, frame bytes, parser-entry/depth, CLUT, and
        // derived-work limits remain mapping-layer checks because the native
        // codec has no equivalent limit field.  `intersect` then preserves
        // every caller-native limit, including finite values not represented
        // by the outer resource budget.
        let resource_ceiling = NativeDecodeLimits::new(
            resource.max_input_bytes,
            usize::try_from(resource.max_width).unwrap_or(usize::MAX),
            usize::try_from(resource.max_height).unwrap_or(usize::MAX),
            resource.max_pixels,
            resource.max_plane_bytes,
            resource.max_metadata_bytes,
            resource.max_icc_bytes,
            usize::MAX,
            usize::MAX,
            resource.max_grid_cells,
            resource.max_derived_depth,
            resource.max_frame_count,
        );
        let resource_ceiling = resource_ceiling
            .tighten_max_live_allocation_bytes(resource.max_total_live_decoded_bytes)
            .map_err(DecodeError::from)?;
        Ok(native.intersect(resource_ceiling))
    }

    pub fn new(
        data: &[u8],
        native_limits: NativeDecodeLimits,
        resource_limits: ResourceLimits,
    ) -> Result<Self, DecodeError> {
        if data.len() > resource_limits.max_input_bytes {
            return Err(DecodeError::Processing(
                super::super::ProcessingError::ResourceLimit(
                    "AVIS input exceeds highres resource limits".into(),
                ),
            ));
        }
        let native_limits = Self::effective_native_limits(native_limits, resource_limits)?;
        let decoder = avif_codec::StrictAvifSequenceDecoder::new(data, native_limits)?;
        let information = AvifSequenceInformation {
            frame_count: decoder.frame_count() as u64,
            timescale: decoder.timescale(),
            duration_in_timescales: decoder.duration_in_timescales(),
            repetition: decoder.repetition_count(),
        };
        Ok(Self {
            decoder,
            resource_limits,
            information,
        })
    }

    pub const fn information(&self) -> AvifSequenceInformation {
        self.information
    }

    /// Decodes and maps the next native frame, or returns `None` at stable EOF.
    pub fn next_frame(&mut self) -> Result<Option<AvifSequenceFrame>, DecodeError> {
        let Some(prepared) = self.decoder.prepare_next_frame()? else {
            return Ok(None);
        };
        let index = prepared.frame_index() as u64;
        let native_timing = prepared.timing();
        let pixel_aspect_ratio = prepared.information().pixel_aspect_ratio();
        let timing = FrameTiming::new(
            native_timing.timescale,
            native_timing.pts_in_timescales,
            native_timing.duration_in_timescales,
        )?;
        let retained_live_bytes = prepared.retained_live_bytes();
        let ready = prepared.prepare_commit()?;
        let limits = self.resource_limits;
        let frame = ready.try_map_and_commit(|frame, rich, _, _| {
            let mut output = consume_native_frame_with_external_live(
                frame,
                rich,
                &limits,
                Some(timing),
                retained_live_bytes,
            )
            .map_err(DecodeError::from)?;
            if let Some((horizontal, vertical)) = pixel_aspect_ratio {
                let ratio = PixelAspectRatio::new(horizontal, vertical)?;
                output.metadata_mut().set_pixel_aspect_ratio(Some(ratio));
            }
            Ok::<ImageFrame, DecodeError>(output)
        })?;
        Ok(Some(AvifSequenceFrame { index, frame }))
    }
}
