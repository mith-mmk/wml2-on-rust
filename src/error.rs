use self::ImgError::{Custom, Simple, SimpleAddMessage};
use self::ErrorKind::*;

#[allow(unused)]
pub enum ImgError {
    Simple(ErrorKind),
    SimpleAddMessage(ErrorKind,String),
    Custom(String),
}

#[allow(unused)]
impl ImgError {
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

#[allow(unused)]
pub enum ErrorKind {
    UnknownFormat,
    OutOfMemory,
    CannotDecode,
    CannotEncode,
    MemoryOfShortage,
    SizeZero,
    NoSupportFormat,
    UnimprimentFormat,
    IlligalData,
    DecodeError,
    WriteError,
    IOError,
    OutboundIndex,
    Reset,
    IlligalCallback,
    NotInitializedImageBuffer,
    UnknownError,
}

#[allow(unused)]
impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match &*self {
            UnknownFormat => {"Unknown format"},
            OutOfMemory => {"Out of memory"},
            CannotDecode => {"Cannot decode this decoder"},
            CannotEncode => {"Cannot encode this encoder"},
            MemoryOfShortage => {"Memroy shortage"},
            SizeZero => {"size is zero"},
            NoSupportFormat => {"No Support format"},
            UnimprimentFormat => {"Unimplement format"},
            IlligalData => {"illigal data"},
            DecodeError => {"decode error"},
            WriteError => {"write error"},
            IOError => {"IO error"},
            Reset => {"Decoder Reset command"},
            OutboundIndex => {"Outbound index"},
            IlligalCallback => {"Illigal Callback"},
            NotInitializedImageBuffer => {"Not initialized Image Buffer"},
            UnknownError => {"Unkonw error"}            
        }
    }
}

