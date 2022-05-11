use std::fmt::*;
use crate::warning::ImgWarning;

pub struct TiffWarning {
    message: String,
}

impl ImgWarning for TiffWarning {

}
impl Debug for TiffWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        std::fmt::Display::fmt(&self.message, f)        
    }
}

impl Display for TiffWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", &self.message)
    }
}

impl TiffWarning {
    pub fn new(message:String) -> Self {
        Self {
            message
        }
    }
}