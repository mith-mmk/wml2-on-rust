use crate::bmp::worning::BMPWorning;
use crate::jpeg::worning::JPEGWorning;

pub enum ImgWornings {
    Jpeg(JPEGWorning),
    Bmp(BMPWorning),
}