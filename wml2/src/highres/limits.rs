//! Explicit caller-provided resource budgets for owned highres frames.

use super::{HighresError, ImageFrame, ProcessingError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    pub(crate) max_input_bytes: usize,
    pub(crate) max_width: u32,
    pub(crate) max_height: u32,
    pub(crate) max_pixels: usize,
    pub(crate) max_channels: usize,
    pub(crate) max_planes: usize,
    pub(crate) max_plane_bytes: usize,
    pub(crate) max_frame_bytes: usize,
    pub(crate) max_total_live_decoded_bytes: usize,
    pub(crate) max_references: usize,
    pub(crate) max_frame_count: usize,
    pub(crate) max_metadata_bytes: usize,
    pub(crate) max_icc_bytes: usize,
    pub(crate) max_clut_bytes: usize,
    pub(crate) max_parser_entries: usize,
    pub(crate) max_parser_depth: usize,
    pub(crate) max_grid_cells: usize,
    pub(crate) max_derived_work: usize,
    pub(crate) max_derived_depth: usize,
}

#[derive(Debug, Default)]
pub struct ResourceLimitsBuilder {
    values: [Option<usize>; 19],
}

impl ResourceLimits {
    pub fn builder() -> ResourceLimitsBuilder {
        ResourceLimitsBuilder::default()
    }
    pub(crate) fn check_frame(&self, frame: &ImageFrame) -> Result<(), ProcessingError> {
        let descriptor = frame.descriptor();
        let width = descriptor.width();
        let height = descriptor.height();
        let pixels = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .ok_or_else(|| ProcessingError::ResourceLimit("pixel count overflows".into()))?;
        if width > self.max_width || height > self.max_height || pixels > self.max_pixels {
            return Err(ProcessingError::ResourceLimit(
                "image dimensions exceed resource limits".into(),
            ));
        }
        if descriptor.planes().len() > self.max_planes {
            return Err(ProcessingError::ResourceLimit(
                "plane count exceeds resource limits".into(),
            ));
        }
        let channels = descriptor
            .planes()
            .iter()
            .map(|plane| plane.roles().len())
            .sum::<usize>();
        if channels > self.max_channels {
            return Err(ProcessingError::ResourceLimit(
                "channel count exceeds resource limits".into(),
            ));
        }
        let bytes_per_sample = match frame.pixels().format() {
            super::PixelFormat::U8 => 1,
            super::PixelFormat::U16 => 2,
            super::PixelFormat::F32 => 4,
        };
        let mut frame_bytes = 0usize;
        for plane in descriptor.planes() {
            let bytes = plane
                .layout()
                .addressed_samples()
                .checked_mul(bytes_per_sample)
                .ok_or_else(|| {
                    ProcessingError::ResourceLimit("plane byte count overflows".into())
                })?;
            if bytes > self.max_plane_bytes {
                return Err(ProcessingError::ResourceLimit(
                    "plane bytes exceed resource limits".into(),
                ));
            }
            frame_bytes = frame_bytes.checked_add(bytes).ok_or_else(|| {
                ProcessingError::ResourceLimit("frame byte count overflows".into())
            })?;
        }
        if frame_bytes > self.max_frame_bytes || frame_bytes > self.max_total_live_decoded_bytes {
            return Err(ProcessingError::ResourceLimit(
                "frame bytes exceed resource limits".into(),
            ));
        }
        let allocated_pixel_bytes = frame
            .pixels()
            .allocated_sample_count()
            .map_err(ProcessingError::from)?
            .checked_mul(bytes_per_sample)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("allocated frame byte count overflows".into())
            })?;
        if allocated_pixel_bytes > self.max_frame_bytes {
            return Err(ProcessingError::ResourceLimit(
                "allocated frame bytes exceed resource limits".into(),
            ));
        }
        let largest_allocated_plane_bytes = frame
            .pixels()
            .max_allocated_plane_samples()
            .map_err(ProcessingError::from)?
            .checked_mul(bytes_per_sample)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("allocated plane byte count overflows".into())
            })?;
        if largest_allocated_plane_bytes > self.max_plane_bytes {
            return Err(ProcessingError::ResourceLimit(
                "allocated plane bytes exceed resource limits".into(),
            ));
        }
        let source_icc_bytes = frame.metadata().source_color().icc_profile_capacity();
        let active_icc_bytes = frame
            .descriptor()
            .color_information()
            .icc_profile_capacity();
        let source_metadata_bytes = frame
            .metadata()
            .metadata_bytes()
            .map_err(ProcessingError::from)?;
        let active_metadata_bytes = frame
            .descriptor()
            .color_information()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let metadata_bytes = source_metadata_bytes
            .checked_add(active_metadata_bytes)
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("metadata byte count overflows".into())
            })?;
        if source_icc_bytes > self.max_icc_bytes
            || active_icc_bytes > self.max_icc_bytes
            || metadata_bytes > self.max_metadata_bytes
        {
            return Err(ProcessingError::ResourceLimit(
                "ICC/metadata bytes exceed resource limits".into(),
            ));
        }
        let owned_descriptor_bytes = descriptor.owned_bytes().map_err(ProcessingError::from)?;
        let owned_pixel_bytes = frame
            .pixels()
            .owned_bytes()
            .map_err(ProcessingError::from)?;
        let owned_frame_bytes = owned_descriptor_bytes
            .checked_add(owned_pixel_bytes)
            // The descriptor owns the active colour information already;
            // adding `metadata_bytes` here would charge that same allocation
            // a second time.  The metadata sublimit above intentionally
            // includes source and active colours, while final frame ownership
            // counts descriptor-owned bytes plus source metadata only.
            .and_then(|value| value.checked_add(source_metadata_bytes))
            .ok_or_else(|| {
                ProcessingError::ResourceLimit("owned frame byte count overflows".into())
            })?;
        if owned_frame_bytes > self.max_frame_bytes
            || owned_frame_bytes > self.max_total_live_decoded_bytes
        {
            return Err(ProcessingError::ResourceLimit(
                "owned frame allocations exceed resource limits".into(),
            ));
        }
        let live_bytes = allocated_pixel_bytes
            .checked_add(metadata_bytes)
            .ok_or_else(|| ProcessingError::ResourceLimit("live byte count overflows".into()))?;
        if live_bytes > self.max_frame_bytes || live_bytes > self.max_total_live_decoded_bytes {
            return Err(ProcessingError::ResourceLimit(
                "total live decoded bytes exceed resource limits".into(),
            ));
        }
        Ok(())
    }
}

impl ResourceLimitsBuilder {
    fn set(mut self, index: usize, value: usize) -> Self {
        self.values[index] = Some(value);
        self
    }
    pub fn max_input_bytes(self, value: usize) -> Self {
        self.set(0, value)
    }
    pub fn max_width(self, value: u32) -> Self {
        let mut builder = self;
        builder.values[1] = usize::try_from(value).ok();
        builder
    }
    pub fn max_height(self, value: u32) -> Self {
        let mut builder = self;
        builder.values[2] = usize::try_from(value).ok();
        builder
    }
    pub fn max_pixels(self, value: usize) -> Self {
        self.set(3, value)
    }
    pub fn max_channels(self, value: usize) -> Self {
        self.set(4, value)
    }
    pub fn max_planes(self, value: usize) -> Self {
        self.set(5, value)
    }
    pub fn max_plane_bytes(self, value: usize) -> Self {
        self.set(6, value)
    }
    pub fn max_frame_bytes(self, value: usize) -> Self {
        self.set(7, value)
    }
    pub fn max_total_live_decoded_bytes(self, value: usize) -> Self {
        self.set(8, value)
    }
    pub fn max_references(self, value: usize) -> Self {
        self.set(9, value)
    }
    pub fn max_frame_count(self, value: usize) -> Self {
        self.set(10, value)
    }
    pub fn max_metadata_bytes(self, value: usize) -> Self {
        self.set(11, value)
    }
    pub fn max_icc_bytes(self, value: usize) -> Self {
        self.set(12, value)
    }
    pub fn max_clut_bytes(self, value: usize) -> Self {
        self.set(13, value)
    }
    pub fn max_parser_entries(self, value: usize) -> Self {
        self.set(14, value)
    }
    pub fn max_parser_depth(self, value: usize) -> Self {
        self.set(15, value)
    }
    pub fn max_grid_cells(self, value: usize) -> Self {
        self.set(16, value)
    }
    pub fn max_derived_work(self, value: usize) -> Self {
        self.set(17, value)
    }
    pub fn max_derived_depth(self, value: usize) -> Self {
        self.set(18, value)
    }
    pub fn build(self) -> Result<ResourceLimits, HighresError> {
        let v = self.values;
        if v.iter().any(Option::is_none) {
            return Err(HighresError::InvalidMetadata(
                "all resource limits must be explicitly set".into(),
            ));
        }
        let get = |i: usize| v[i].expect("checked above");
        let max_width = u32::try_from(get(1))
            .map_err(|_| HighresError::InvalidMetadata("max width exceeds u32".into()))?;
        let max_height = u32::try_from(get(2))
            .map_err(|_| HighresError::InvalidMetadata("max height exceeds u32".into()))?;
        Ok(ResourceLimits {
            max_input_bytes: get(0),
            max_width,
            max_height,
            max_pixels: get(3),
            max_channels: get(4),
            max_planes: get(5),
            max_plane_bytes: get(6),
            max_frame_bytes: get(7),
            max_total_live_decoded_bytes: get(8),
            max_references: get(9),
            max_frame_count: get(10),
            max_metadata_bytes: get(11),
            max_icc_bytes: get(12),
            max_clut_bytes: get(13),
            max_parser_entries: get(14),
            max_parser_depth: get(15),
            max_grid_cells: get(16),
            max_derived_work: get(17),
            max_derived_depth: get(18),
        })
    }
}
