//! Errors specific to new highres processing boundaries.

use super::HighresError;
use std::fmt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingError {
    Invalid(HighresError),
    Malformed(String),
    Truncated(String),
    ResourceLimit(String),
    Allocation(String),
    Unsupported(String),
    Conversion(String),
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(e) => e.fmt(f),
            Self::Malformed(s) => write!(f, "malformed input: {s}"),
            Self::Truncated(s) => write!(f, "truncated input: {s}"),
            Self::ResourceLimit(s) => write!(f, "resource limit: {s}"),
            Self::Allocation(s) => write!(f, "allocation failed: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported: {s}"),
            Self::Conversion(s) => write!(f, "conversion failed: {s}"),
        }
    }
}
impl std::error::Error for ProcessingError {}
impl From<HighresError> for ProcessingError {
    fn from(value: HighresError) -> Self {
        Self::Invalid(value)
    }
}
