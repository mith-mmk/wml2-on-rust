//use crate::error::Repr as OtherRepr;
use core::fmt::*;

pub struct ImgError {
    repr: Repr,
}

impl Debug for ImgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Debug::fmt(&self.repr, f)
    }
}

impl ImgError {
    pub fn new<E>(kind: ImgErrorKind, error: E) -> ImgError
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ImgErrorKind, error: Box<dyn std::error::Error + Send + Sync>) -> ImgError {
        ImgError { repr: Repr::Custom(Box::new(Custom { kind, error })) }
    }

    #[inline]
    pub(crate) const fn new_const(kind: ImgErrorKind, message: &'static &'static str) -> ImgError {
        Self { repr: Repr::SimpleMessage(kind, message) }
    }

    #[must_use]
    #[inline]
    pub fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Os(i) => Some(i),
            Repr::Custom(..) => None,
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
        }
    }

    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Os(..) => None,
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => Some(&*c.error),
        }
    }

    pub fn into_inner(self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        match self.repr {
            Repr::Os(..) => None,
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
            Repr::Custom(c) => Some(c.error),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Repr {
    Os(i32),
    Simple(ImgErrorKind),
    // &str is a fat pointer, but &&str is a thin pointer.
    SimpleMessage(ImgErrorKind, &'static &'static str),
    Custom(Box<Custom>),
}

#[derive(Debug)]
pub(crate) struct Custom {
    kind: ImgErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
}

#[allow(unused)]
#[derive(Debug)]
pub enum ImgErrorKind {
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
    OSError,
    UnknownError,
}

#[allow(unused)]
impl ImgErrorKind {
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
            NotInitializedImageBuffer => {"OS error"},
            UnknownError => {"Unkonw error"}            
        }
    }
}