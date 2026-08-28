//! Native AVIF ownership mapping. This module intentionally exposes no
//! standalone codec types through the highres public namespace.

pub(super) mod allocation;
mod mapping;
#[cfg(test)]
mod mapping_tests;
mod metadata;

pub(crate) use mapping::consume_native_frame;
