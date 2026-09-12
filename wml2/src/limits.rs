//! Resource budgets for the callback decode entry points.
use std::cell::Cell;
type Error = Box<dyn std::error::Error>;

/// Independent limits. `usize::MAX` explicitly disables a particular limit.
#[derive(Clone, Copy, Debug)]
pub struct DecodeLimits {
    pub input_bytes: usize,
    pub pixels: usize,
    pub expanded_bytes: usize,
    pub metadata_bytes: usize,
    pub frames: usize,
    pub animation_bytes: usize,
}

impl DecodeLimits {
    pub const fn unlimited() -> Self {
        Self {
            input_bytes: usize::MAX,
            pixels: usize::MAX,
            expanded_bytes: usize::MAX,
            metadata_bytes: usize::MAX,
            frames: usize::MAX,
            animation_bytes: usize::MAX,
        }
    }
}
impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            input_bytes: 512 * 1024 * 1024,
            pixels: 128 * 1024 * 1024,
            expanded_bytes: 512 * 1024 * 1024,
            metadata_bytes: 32 * 1024 * 1024,
            frames: 10_000,
            animation_bytes: 512 * 1024 * 1024,
        }
    }
}

thread_local! {
    static LIMITS: Cell<DecodeLimits> = Cell::new(DecodeLimits::default());
    static METADATA: Cell<usize> = const { Cell::new(0) };
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}
pub(crate) fn current() -> DecodeLimits {
    LIMITS.get()
}
pub(crate) fn scope<T>(limits: DecodeLimits, body: impl FnOnce() -> T) -> T {
    struct Restore(DecodeLimits, usize, bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LIMITS.set(self.0);
            METADATA.set(self.1);
            ACTIVE.set(self.2);
        }
    }
    let _restore = Restore(
        LIMITS.replace(limits),
        METADATA.replace(0),
        ACTIVE.replace(true),
    );
    body()
}
pub(crate) fn check(value: usize, limit: usize, name: &str) -> Result<(), Error> {
    if value > limit {
        return Err(Box::new(crate::error::ImgError::new_const(
            crate::error::ImgErrorKind::OutOfMemory,
            format!("{name} exceeds decode limit"),
        )));
    }
    Ok(())
}

/// Reader lengths and offsets remain u64 even on 32-bit hosts. The documented
/// usize::MAX sentinel disables the input budget without narrowing the length.
pub(crate) fn check_input_length(length: u64, limit: usize) -> Result<(), Error> {
    if limit != usize::MAX && length > u64::try_from(limit)? {
        return Err(Box::new(crate::error::ImgError::new_const(
            crate::error::ImgErrorKind::OutOfMemory,
            "input exceeds decode limit".into(),
        )));
    }
    Ok(())
}
#[cfg(any(feature = "png", test))]
pub(crate) fn charge_metadata(bytes: usize) -> Result<(), Error> {
    let total = metadata_used()
        .checked_add(bytes)
        .ok_or_else(|| std::io::Error::other("metadata size overflow"))?;
    check(total, current().metadata_bytes, "metadata")?;
    if ACTIVE.get() {
        METADATA.set(total);
    }
    Ok(())
}
#[cfg(any(feature = "png", test))]
pub(crate) fn inflate_metadata(data: &[u8]) -> Result<Vec<u8>, Error> {
    let limit = current().metadata_bytes.saturating_sub(metadata_used());
    let decoded = inflate_bounded(data, limit)?;
    charge_metadata(decoded.len())?;
    Ok(decoded)
}

#[cfg(any(feature = "png", test))]
fn metadata_used() -> usize {
    if ACTIVE.get() { METADATA.get() } else { 0 }
}
#[cfg(any(feature = "png", feature = "psd", feature = "tiff"))]
pub(crate) fn inflate_image(data: &[u8], expected: usize) -> Result<Vec<u8>, Error> {
    check(expected, current().expanded_bytes, "expanded image")?;
    inflate_bounded(data, expected)
}

// The 0.7.x core API allows fallible growth; its convenience Vec API uses
// infallible allocations even when an output limit is supplied.
#[cfg(any(feature = "png", feature = "psd", feature = "tiff", test))]
fn inflate_bounded(mut input: &[u8], limit: usize) -> Result<Vec<u8>, Error> {
    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};
    let flags = inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let mut state = DecompressorOxide::default();
    let mut output = Vec::new();
    let initial = input.len().saturating_mul(2).min(limit);
    output.try_reserve_exact(initial)?;
    output.resize(initial, 0);
    let mut position = 0;
    loop {
        let (status, consumed, written) =
            decompress(&mut state, input, &mut output, position, flags);
        position += written;
        match status {
            TINFLStatus::Done => {
                output.truncate(position);
                return Ok(output);
            }
            TINFLStatus::HasMoreOutput if output.len() < limit && consumed <= input.len() => {
                input = &input[consumed..];
                let length = output.len().max(1).saturating_mul(2).min(limit);
                output.try_reserve_exact(length - output.len())?;
                output.resize(length, 0);
            }
            _ => {
                return Err(std::io::Error::other(format!(
                    "inflate failed or exceeded limit: {status:?}"
                ))
                .into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_reader_lengths_keep_finite_budgets_and_unlimited_offsets() {
        let beyond_classic = u64::from(u32::MAX) + 1;
        assert!(check_input_length(beyond_classic, 512 * 1024 * 1024).is_err());
        assert!(check_input_length(beyond_classic, usize::MAX).is_ok());
        assert!(check_input_length(8, 8).is_ok());
        assert!(check_input_length(9, 8).is_err());
    }
    #[test]
    fn inflation_and_aggregate_metadata_stop_at_the_budget() {
        let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&vec![42; 4096], 6);
        assert_eq!(inflate_bounded(&compressed, 4096).unwrap().len(), 4096);
        assert!(inflate_bounded(&compressed, 4095).is_err());
        assert!(inflate_bounded(&compressed, 0).is_err());
        scope(
            DecodeLimits {
                metadata_bytes: 8192,
                ..Default::default()
            },
            || {
                inflate_metadata(&compressed).unwrap();
                inflate_metadata(&compressed).unwrap();
                assert!(inflate_metadata(&compressed).is_err());
            },
        );
    }
}
