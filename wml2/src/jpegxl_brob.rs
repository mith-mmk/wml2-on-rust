//! Strict RFC 7932 streaming decode with bounded output and allocation requests.
//! The dependency's convenience Vec/Read APIs do not provide this contract.
use super::{Error, error, invalid};
use crate::error::ImgErrorKind;
use crate::limits;
use brotli_decompressor::{
    Allocator, BrotliDecompressStream, BrotliResult, BrotliState, SliceWrapper, SliceWrapperMut,
};
use std::cell::Cell;

const CHUNK: usize = 16 * 1024;
const MAX_CODEC_HEAP: usize = 32 * 1024 * 1024;

struct Budget {
    limit: usize,
    live: Cell<usize>,
    codec_live: Cell<usize>,
    denied: Cell<bool>,
    allocation_failed: Cell<bool>,
}
impl Budget {
    fn claim(&self, bytes: usize, codec: bool) -> bool {
        let Some(live) = self.live.get().checked_add(bytes) else {
            self.denied.set(true);
            return false;
        };
        let Some(codec_live) = self
            .codec_live
            .get()
            .checked_add(if codec { bytes } else { 0 })
        else {
            self.denied.set(true);
            return false;
        };
        if live > self.limit || codec_live > MAX_CODEC_HEAP {
            self.denied.set(true);
            return false;
        }
        self.live.set(live);
        self.codec_live.set(codec_live);
        true
    }
    fn release(&self, bytes: usize, codec: bool) {
        self.live.set(self.live.get() - bytes);
        if codec {
            self.codec_live.set(self.codec_live.get() - bytes);
        }
    }
    fn check(&self) -> Result<(), Error> {
        if self.denied.get() {
            return Err(error(
                ImgErrorKind::OutOfMemory,
                "JPEG XL brob workspace exceeds decode limit",
            ));
        }
        if self.allocation_failed.get() {
            return Err(error(
                ImgErrorKind::OutOfMemory,
                "JPEG XL brob allocation failed",
            ));
        }
        Ok(())
    }
}
struct Memory<'a, T> {
    data: Vec<T>,
    budget: Option<&'a Budget>,
}
impl<T> Default for Memory<'_, T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            budget: None,
        }
    }
}
impl<T> Drop for Memory<'_, T> {
    fn drop(&mut self) {
        if let Some(budget) = self.budget {
            budget.release(self.data.len() * std::mem::size_of::<T>(), true);
        }
    }
}
impl<T> SliceWrapper<T> for Memory<'_, T> {
    fn slice(&self) -> &[T] {
        &self.data
    }
}
impl<T> SliceWrapperMut<T> for Memory<'_, T> {
    fn slice_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}
struct BoundedAllocator<'a>(&'a Budget);
impl<'a, T: Default + Clone> Allocator<T> for BoundedAllocator<'a> {
    type AllocatedMemory = Memory<'a, T>;
    fn alloc_cell(&mut self, count: usize) -> Self::AllocatedMemory {
        let Some(bytes) = count.checked_mul(std::mem::size_of::<T>()) else {
            self.0.denied.set(true);
            return Memory::default();
        };
        if !self.0.claim(bytes, true) {
            return Memory::default();
        }
        let mut data = Vec::new();
        if data.try_reserve_exact(count).is_err() {
            self.0.release(bytes, true);
            self.0.allocation_failed.set(true);
            return Memory::default();
        }
        data.resize(count, T::default());
        Memory {
            data,
            budget: Some(self.0),
        }
    }
    fn free_cell(&mut self, data: Self::AllocatedMemory) {
        drop(data);
    }
}
type State<'a> = BrotliState<BoundedAllocator<'a>, BoundedAllocator<'a>, BoundedAllocator<'a>>;

pub(super) fn decompress(
    input: &[u8],
    maximum: usize,
    retained: usize,
    workspace: usize,
) -> Result<Vec<u8>, Error> {
    // Conservative accounting for state construction/moves, the bounded stack
    // output chunk and allocator/control records; not a process-RSS promise.
    let overhead = 2 * std::mem::size_of::<State<'_>>() + CHUNK + 4096;
    let base = super::add(retained, overhead)?;
    limits::check(base, workspace, "JPEG XL brob workspace")?;
    let budget = Budget {
        limit: workspace - base,
        live: Cell::new(0),
        codec_live: Cell::new(0),
        denied: Cell::new(false),
        allocation_failed: Cell::new(false),
    };
    // new() admits a nonstandard large-window extension; brob uses standard
    // Brotli. new_strict() also leaves external/shared dictionaries disabled.
    let mut state = State::new_strict(
        BoundedAllocator(&budget),
        BoundedAllocator(&budget),
        BoundedAllocator(&budget),
    );
    budget.check()?; // Includes constructor allocation failure before indexing.
    let mut output = Vec::new();
    let mut scratch = [0u8; CHUNK];
    let mut available_in = input.len();
    let mut input_offset = 0;
    let mut total_out = 0;
    loop {
        let previous_offset = input_offset;
        // One excess byte detects a quota violation even exactly at the limit;
        // an empty completed stream remains valid when maximum is zero.
        let mut available_out = maximum
            .saturating_sub(output.len())
            .saturating_add(1)
            .min(CHUNK);
        let mut offset = 0;
        let result = BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            input,
            &mut available_out,
            &mut offset,
            &mut scratch,
            &mut total_out,
            &mut state,
        );
        budget.check()?;
        let next = super::add(output.len(), offset)?;
        limits::check(next, maximum, "JPEG XL brob expanded metadata")?;
        if offset != 0 {
            // Charge the full new output request while retaining the old one;
            // realloc is permitted to keep both allocations live transiently.
            if !budget.claim(next, false) {
                budget.check()?;
            }
            output.try_reserve_exact(offset)?;
            budget.release(output.len(), false);
            output.extend_from_slice(&scratch[..offset]);
        }
        match result {
            BrotliResult::ResultSuccess => {
                if available_in != 0 || input_offset != input.len() {
                    return Err(invalid("trailing JPEG XL brob compressed data"));
                }
                return Ok(output);
            }
            BrotliResult::NeedsMoreInput => {
                return Err(error(
                    ImgErrorKind::UnexpectedEof,
                    "truncated JPEG XL brob stream",
                ));
            }
            BrotliResult::ResultFailure => return Err(invalid("invalid JPEG XL brob stream")),
            BrotliResult::NeedsMoreOutput => {
                if offset == 0 && input_offset == previous_offset {
                    return Err(invalid("JPEG XL brob made no progress"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incompressible_stream_and_mutated_headers_remain_bounded() {
        let stream = include_bytes!("../tests/fixtures/review/jxl-brob-random.br");
        let mut seed = 0x12345678u32;
        let expected: Vec<u8> = (0..16384 + 31)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        assert_eq!(
            decompress(stream, expected.len(), 0, 4 << 20).unwrap(),
            expected
        );
        let xml = include_bytes!("../tests/fixtures/review/jxl-brob-xml.br");
        for index in 0..xml.len().min(32) {
            for bit in 0..8 {
                let mut changed = xml.to_vec();
                changed[index] ^= 1 << bit;
                if let Ok(data) = decompress(&changed, 1024, 0, 2 << 20) {
                    assert!(data.len() <= 1024);
                }
            }
        }
        let large = include_bytes!("../tests/fixtures/review/jxl-brob-large-window-invalid.br");
        assert!(
            decompress(large, 1024, 0, 2 << 20)
                .unwrap_err()
                .to_string()
                .contains("invalid JPEG XL brob stream")
        );
    }

    #[test]
    fn strict_output_limit_accepts_empty_and_exact_and_rejects_one_byte_less() {
        let empty = include_bytes!("../tests/fixtures/review/jxl-brob-empty.br");
        assert!(decompress(empty, 0, 0, 4 << 20).unwrap().is_empty());
        let repeat = include_bytes!("../tests/fixtures/review/jxl-brob-repeat.br");
        let size = 65536 + 123;
        assert_eq!(
            decompress(repeat, size, 0, 4 << 20).unwrap(),
            vec![b'A'; size]
        );
        assert!(
            decompress(repeat, size - 1, 0, 4 << 20)
                .unwrap_err()
                .to_string()
                .contains("brob expanded metadata")
        );
    }
    #[test]
    fn workspace_denials_include_constructor_decoder_and_output_growth_without_panics() {
        for (stream, size) in [
            (
                include_bytes!("../tests/fixtures/review/jxl-brob-xml.br").as_slice(),
                1024,
            ),
            (
                include_bytes!("../tests/fixtures/review/jxl-brob-random.br").as_slice(),
                16384 + 31,
            ),
        ] {
            let mut low = 0;
            let mut high = 4 << 20;
            assert!(decompress(stream, size, 0, high).is_ok());
            while low < high {
                let mid = low + (high - low) / 2;
                match decompress(stream, size, 0, mid) {
                    Ok(_) => high = mid,
                    Err(error) => {
                        assert!(error.to_string().contains("workspace"), "{error}");
                        low = mid + 1;
                    }
                }
            }
            assert!(decompress(stream, size, 0, high).is_ok());
            assert!(decompress(stream, size, 0, high - 1).is_err());
        }
    }
}
