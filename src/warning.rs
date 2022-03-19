use core::fmt::*;
use crate::jpeg::warning::JpegWarningKind;
use crate::bmp::warning::BMPWarningKind;

pub trait WarningKind {
    fn as_str(&self) -> &'static str;
}

pub struct ImgWarning {
    repr: Repr,
}

impl Debug for ImgWarning {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Debug::fmt(&self.repr, f)
    }
}

impl ImgWarning {
    pub fn new<E>(kind: ImgWarningKind) -> ImgWarning {
        ImgWarning { repr: Repr::Simple(kind) }
    }

    pub fn add(kind: ImgWarningKind, warning: ImgWarning) -> ImgWarning {
        ImgWarning  { repr: Repr::Custom(Box::new(Custom { kind, warning:Box::new(warning) })) }
    }

    #[inline]
    pub(crate) const fn new_const(kind: ImgWarningKind, message: &'static &'static str) -> ImgWarning {
        Self { repr: Repr::SimpleMessage(kind, message) }
    }

    #[must_use]
    #[inline]
    pub fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Custom(..) => None,
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
        }
    }

    #[must_use]
    #[inline]
    pub fn get_ref(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => None, //todo
        }
    }

    pub fn into_inner(self) -> Option<Box<dyn std::error::Error + Send + Sync>> {
        match self.repr {
            Repr::Simple(..) => None,
            Repr::SimpleMessage(..) => None,
            Repr::Custom(ref c) => None, //todo
        }
    }
}

#[derive(Debug)]
pub(crate) enum Repr {
    Simple(ImgWarningKind),
    // &str is a fat pointer, but &&str is a thin pointer.
    SimpleMessage(ImgWarningKind, &'static &'static str),
    Custom(Box<Custom>),
}

#[derive(Debug)]
pub(crate) struct Custom {
    kind: ImgWarningKind,
    warning: Box<dyn Debug + Send + Sync>,
}



#[derive(Debug)]
pub enum ImgWarningKind {
    Jpeg(JpegWarningKind),
    Bmp(BMPWarningKind),
    Other,
}

impl WarningKind for ImgWarningKind {

    fn as_str(&self) -> &'static str {
        use self::ImgWarningKind::*;
        match &*self {
           Jpeg(waning) => {
               "todo"
           },
           Bmp(waning) => {
               "todo"
           },
           Other => {
               "Unknown warning"
           }
        }
     }
}