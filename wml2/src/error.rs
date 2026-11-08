//! Error types returned by decoders, encoders, and helpers.

use std::error::Error;
use std::fmt::*;

/// Library error type with a high-level kind.
///
/// Decode/encode APIs also propagate other boxed errors (including I/O,
/// allocation and caller callback errors). Downcasting to this type is optional,
/// not a guarantee that every failure has an [`ImgErrorKind`].
pub struct ImgError {
    repr: Repr,
}

impl Error for ImgError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.get_ref().map(|error| error as &(dyn Error + 'static))
    }
}

impl Display for ImgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(&self.repr, f)
    }
}

impl Debug for ImgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Debug::fmt(&self.repr, f)
    }
}

impl ImgError {
    /// Returns the stored category without parsing the display message.
    ///
    /// This is the category assigned by the producing code, not a reclassification
    /// of its source. In particular, legacy `OutOfMemory` includes some resource
    /// limits; inspect [`Error::source`] for [`crate::limits::DecodeLimitExceeded`].
    #[must_use]
    pub fn kind(&self) -> ImgErrorKind {
        match &self.repr {
            Repr::SimpleMessage(kind, _) => *kind,
            Repr::Custom(custom) => custom.kind,
        }
    }

    #[cfg(any(feature = "jpeg", all(feature = "jpeg-xl", feature = "exif")))]
    pub(crate) fn is_optional_metadata_error(&self) -> bool {
        matches!(
            self.kind(),
            ImgErrorKind::IllegalData
                | ImgErrorKind::NoSupportFormat
                | ImgErrorKind::UnsupportedFeature
                | ImgErrorKind::UnexpectedEof
        )
    }
    #[cfg(feature = "jpeg2000")]
    pub(crate) fn is_unsupported(&self) -> bool {
        matches!(self.kind(), ImgErrorKind::UnsupportedFeature)
    }
    /// Creates an error from another error value and an [`ImgErrorKind`].
    pub fn new<E>(kind: ImgErrorKind, error: E) -> ImgError
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ImgErrorKind, error: Box<dyn std::error::Error + Send + Sync>) -> ImgError {
        ImgError {
            repr: Repr::Custom(Box::new(Custom { kind, error })),
        }
    }

    #[inline]
    /// Creates a message-only error.
    pub const fn new_const(kind: ImgErrorKind, message: String) -> ImgError {
        Self {
            repr: Repr::SimpleMessage(kind, message),
        }
    }

    #[must_use]
    #[inline]
    /// Returns the OS error code when one is available.
    pub fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Custom(..) => None,
            Repr::SimpleMessage(..) => None,
        }
    }

    #[must_use]
    #[inline]
    /// Returns a shared reference to the wrapped source error.
    pub fn get_ref(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => Some(&*c.error),
        }
    }

    /// Consumes the error and returns the wrapped source error, if any.
    pub fn into_inner(self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        match self.repr {
            Repr::SimpleMessage(..) => None,
            Repr::Custom(c) => Some(c.error),
        }
    }
}

pub(crate) enum Repr {
    SimpleMessage(ImgErrorKind, String),
    Custom(Box<Custom>),
}

impl Display for Repr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::SimpleMessage(kind, message) => write!(f, "{}: {}", kind.as_str(), message),
            Self::Custom(custom) => write!(f, "{}: {}", custom.kind.as_str(), custom.error),
        }
    }
}

impl Debug for Repr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::SimpleMessage(kind, message) => f
                .debug_struct("SimpleMessage")
                .field("kind", kind)
                .field("message", message)
                .finish(),
            Self::Custom(custom) => f
                .debug_struct("Custom")
                .field("kind", &custom.kind)
                .field("error", &custom.error)
                .finish(),
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub(crate) struct Custom {
    kind: ImgErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// High-level error categories used by `wml2`.
///
/// Existing names and variants are retained for compatibility. These categories
/// do not cover every boxed error returned by the library, and are not a reliable
/// whole-file validation result (for example, unsupported and malformed inputs
/// may be rejected at different stages by different codecs).
pub enum ImgErrorKind {
    UnknownFormat,
    OutOfMemory,
    CannotDecode,
    CannotEncode,
    MemoryOfShortage,
    SizeZero,
    NoSupportFormat,
    UnimprimentFormat,
    IllegalData,
    DecodeError,
    EncodeError,
    WriteError,
    IOError,
    OutboundIndex,
    Reset,
    IllegalCallback,
    NotInitializedImageBuffer,
    InvalidParameter,
    UnexpectedEof,
    UnsupportedFeature,
    OSError,
    UnknownError,
}

#[allow(unused)]
impl ImgErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ImgErrorKind::*;
        match self {
            UnknownFormat => "Unknown format",
            OutOfMemory => "Out of memory",
            CannotDecode => "Cannot decode this decoder",
            CannotEncode => "Cannot encode this encoder",
            MemoryOfShortage => "Memroy shortage",
            SizeZero => "size is zero",
            NoSupportFormat => "No Support format",
            UnimprimentFormat => "Unimplement format",
            IllegalData => "Illegal data",
            DecodeError => "decode error",
            EncodeError => "encode error",
            WriteError => "write error",
            IOError => "IO error",
            Reset => "Decoder Reset command",
            OutboundIndex => "Outbound index",
            IllegalCallback => "Illegal Callback",
            NotInitializedImageBuffer => "Not initialized Image Buffer",
            InvalidParameter => "Invalid parameter",
            UnexpectedEof => "Unexpected EOF",
            UnsupportedFeature => "Unsupported feature",
            OSError => "OS error",
            UnknownError => "Unkonw error",
        }
    }
}
