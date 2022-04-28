//
use crate::tiff::header::TiffHeaders;

#[derive(Debug,Clone,PartialEq)]
pub enum DataMap{
    UInt(u64),
    SInt(i64),
    Float(f64),
    UIntAllay(Vec<u64>),
    SIntAllay(Vec<i64>),
    FloatAllay(Vec<f64>),
    Raw(Vec<u8>),
    Ascii(String),
    I18NString(String),
    Exif(TiffHeaders),
    ICCProfile(Vec<u8>),
    None,
}