//! Metadata value types shared by image decoders and encoders.

use crate::tiff::header::TiffHeaders;
use std::collections::HashMap;

/// A map of metadata keys to typed values.
pub type Metadata = HashMap<String, DataMap>;

/// A typed metadata value.
#[derive(Debug, Clone, PartialEq)]
pub enum DataMap {
    UInt(u64),
    SInt(i64),
    Float(f64),
    UIntAllay(Vec<u64>),
    SIntAllay(Vec<i64>),
    FloatAllay(Vec<f64>),
    Raw(Vec<u8>),
    Ascii(String),
    I18NString(String),
    SJISString(Vec<u8>),
    Exif(TiffHeaders),
    ICCProfile(Vec<u8>),
    None,
}

impl DataMap {
    /// Converts the metadata value to a display-oriented string.
    pub fn to_string(&self) -> String {
        match self {
            DataMap::UInt(d) => d.to_string(),
            DataMap::SInt(d) => d.to_string(),
            DataMap::Float(d) => d.to_string(),
            DataMap::UIntAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::SIntAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::FloatAllay(d) => {
                format!("{:?}", d)
            }
            DataMap::Raw(d) => {
                format!("{:?}", d)
            }
            DataMap::Ascii(d) => d.to_string(),
            DataMap::I18NString(d) => d.to_string(),
            DataMap::SJISString(d) => {
                format!("{:?}", d)
            }
            DataMap::Exif(header) => header.to_string(),
            DataMap::ICCProfile(iccprofile) => {
                format!("{:?}", iccprofile)
            }
            DataMap::None => "none".to_owned(),
        }
    }
}
