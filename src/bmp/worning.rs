
use self::BMPWorning::*;
use self::WorningKind::*;

pub enum BMPWorning {
      Simple(WorningKind),
      SimpleAddMessage(WorningKind,String),
      Custom(String),
  }

impl BMPWorning {
    pub fn fmt(&self) -> String {
        match self {
            Simple(error_kind) => { error_kind.as_str().to_string()},
            SimpleAddMessage(error_kind,s) => {
                error_kind.as_str().to_string() + " " + &s.to_string()
            },
            Custom(s) => {s.to_string()},
        }
    }
}

pub enum WorningKind {
      UnfindEOIMaker,
      DataCorruption,
      BufferOverrun,
      JpegWorning(crate::jpeg::worning::JPEGWorning),
  }

#[allow(unused)]
#[allow(non_snake_case)]
impl WorningKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match &*self {
            OutOfMemory => {"Out of memory"},
            DataCorruption => {"Data Corruption"},
            BufferOverrun => {"Buffer Overrun"},
            JpegWorning(..) => {
                "Jpeg Worning"
            }
        }
    }
}