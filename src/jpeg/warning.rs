/*
 * jpeg/Warning.rs  Mith@mmk (C) 2022
 * use MIT License
 */
use crate::warning::WarningKind;

#[derive(Debug)]
pub enum JpegWarningKind {
      IlligalRSTMaker,
      UnfindEOIMaker,
      DataCorruption,
      BufferOverrun,
      UnexpectMarker,
      UnknowFormat,
  }

#[allow(unused)]
#[allow(non_snake_case)]
impl WarningKind for JpegWarningKind {
    fn as_str(&self) -> &'static str {
        use JpegWarningKind::*;
        match &*self {
            IlligalRSTMaker => {"Illigal RST Maker"},
            OutOfMemory => {"Out of memory"},
            DataCorruption => {"Data Corruption"},
            BufferOverrun => {"Buffer Overrun"},
            UnexpectMarker => {"Unexpect Marker"},
            UnknowFormat => {"Unexpect Maker"},
        }
    }
}