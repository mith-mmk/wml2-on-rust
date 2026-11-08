use std::error::Error;
use std::fmt;
use wml2::draw::*;
use wml2::error::{ImgError, ImgErrorKind};
use wml2::limits::{DecodeLimitExceeded, DecodeLimits};
use wml2::metadata::DataMap;
use wml2::warning::{ImgWarning, ImgWarnings};

#[test]
fn kinds_and_sources_preserve_existing_display_and_accessors() {
    let simple = ImgError::new_const(ImgErrorKind::IllegalData, "bad input".into());
    assert_eq!(simple.kind(), ImgErrorKind::IllegalData);
    assert_eq!(simple.to_string(), "Illegal data: bad input");
    assert!(simple.source().is_none());
    assert!(simple.get_ref().is_none());
    assert!(simple.into_inner().is_none());

    let wrapped = ImgError::new(
        ImgErrorKind::IOError,
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    );
    assert_eq!(wrapped.kind(), ImgErrorKind::IOError);
    assert_eq!(wrapped.to_string(), "IO error: denied");
    assert_eq!(
        wrapped.source().unwrap().downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(wrapped.get_ref().unwrap().is::<std::io::Error>());
    assert!(wrapped.into_inner().unwrap().is::<std::io::Error>());

    let nested = ImgError::new(ImgErrorKind::DecodeError, simple_error());
    assert_eq!(
        nested.source().unwrap().downcast_ref::<ImgError>().unwrap().kind(),
        ImgErrorKind::UnexpectedEof
    );
}

fn simple_error() -> ImgError {
    ImgError::new_const(ImgErrorKind::UnexpectedEof, "short input".into())
}

#[test]
fn shared_input_limit_has_a_typed_source_without_changing_legacy_kind_or_text() {
    let limits = DecodeLimits { input_bytes: 2, ..DecodeLimits::unlimited() };
    let error = image_from_with_limits_and_warnings(&[0; 3], limits).err().unwrap();
    let image_error = error.downcast_ref::<ImgError>().unwrap();
    assert_eq!(image_error.kind(), ImgErrorKind::OutOfMemory);
    assert_eq!(error.to_string(), "Out of memory: input exceeds decode limit");
    let cause = image_error.source().unwrap().downcast_ref::<DecodeLimitExceeded>().unwrap();
    assert_eq!(cause.resource(), "input");
    assert_eq!(cause.requested(), 3);
    assert_eq!(cause.limit(), 2);

    // A fallible allocator's capacity rejection is not a decode policy limit.
    let reserve = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
    let allocation = ImgError::new(ImgErrorKind::OutOfMemory, reserve);
    assert!(allocation.source().unwrap().is::<std::collections::TryReserveError>());
    assert!(!allocation.source().unwrap().is::<DecodeLimitExceeded>());
}

#[test]
fn unknown_input_keeps_the_existing_unsupported_category() {
    let error = image_load(b"not an image").err().unwrap();
    assert_eq!(error.downcast_ref::<ImgError>().unwrap().kind(), ImgErrorKind::NoSupportFormat);
}

// The callback object is Send + Sync, but its returned error need not be.
#[derive(Debug)]
struct LocalCallbackError(std::rc::Rc<u8>);
impl fmt::Display for LocalCallbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "local callback {}", self.0)
    }
}
impl Error for LocalCallbackError {}

#[derive(Default)]
struct CallbackProbe { abort: bool, terminations: usize }
impl DrawCallback for CallbackProbe {
    fn begin_image(&mut self) -> Response {
        if self.abort { Ok(Some(CallbackResponse::abort())) }
        else { Err(Box::new(LocalCallbackError(std::rc::Rc::new(7)))) }
    }
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response { panic!("unexpected init") }
    fn draw(&mut self, _: usize, _: usize, _: usize, _: usize, _: &[u8], _: Option<DrawOptions>) -> Response { panic!("unexpected draw") }
    fn terminate(&mut self, _: Option<TerminateOptions>) -> Response { self.terminations += 1; Ok(None) }
    fn next(&mut self, _: Option<NextOptions>) -> Response { panic!("unexpected next") }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response { panic!("unexpected verbose") }
    fn set_metadata(&mut self, _: &str, _: DataMap) -> Response { panic!("unexpected metadata") }
}

#[test]
fn callback_errors_remain_downcastable_and_abort_remains_success() {
    let mut callback = CallbackProbe::default();
    let error = image_loader(&[], &mut DecodeOptions { debug_flag: 0, drawer: &mut callback }).err().unwrap();
    assert_eq!(*error.downcast_ref::<LocalCallbackError>().unwrap().0, 7);
    assert!(!error.is::<ImgError>());
    assert_eq!(callback.terminations, 0);

    callback.abort = true;
    let warnings = image_loader(&[], &mut DecodeOptions { debug_flag: 0, drawer: &mut callback }).unwrap();
    assert!(warnings.is_none());
    assert_eq!(callback.terminations, 1);
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn filesystem_errors_keep_the_original_io_type() {
    let error = image_from_file("\0".to_owned()).err().unwrap();
    assert!(error.is::<std::io::Error>());
    assert!(!error.is::<ImgError>());
}

#[derive(Debug)]
struct CallerWarning(&'static str);
impl fmt::Display for CallerWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.0) }
}
// Existing external marker-trait implementations need no new methods.
impl ImgWarning for CallerWarning {}

#[test]
fn warning_iteration_preserves_individual_messages_and_order() {
    let first = ImgWarnings::add(None, Box::new(CallerWarning("first")));
    let second = ImgWarnings::add(None, Box::new(CallerWarning("second")));
    let warnings = ImgWarnings::append(first, second).unwrap();
    assert_eq!(warnings.len(), 2);
    assert!(!warnings.is_empty());
    assert_eq!(warnings.iter().len(), 2);
    assert_eq!(warnings.iter().map(ToString::to_string).collect::<Vec<_>>(), ["first", "second"]);
    assert_eq!(warnings.iter().rev().map(ToString::to_string).collect::<Vec<_>>(), ["second", "first"]);
    assert_eq!(warnings.to_string(), "firstsecond");
    assert_eq!(format!("{warnings:?}"), "firstsecond");
    assert!(ImgWarnings::append(None, None).is_none());
}

#[cfg(feature = "jpeg")]
#[test]
fn warning_helpers_preserve_pixels_and_expose_recoverable_exif_warning() {
    use wml2::util::ImageFormat;
    let mut source = ImageBuffer::from_buffer(8, 8, [17, 34, 51, 255].repeat(64));
    let plain = image_to(&mut source, ImageFormat::Jpeg, None).unwrap();
    let (reference, warnings) = image_load_with_warnings(&plain).unwrap();
    assert!(warnings.is_none());

    // Empty EXIF TIFF payload is malformed optional metadata, not corrupt pixels.
    let mut damaged_metadata = plain[..2].to_vec();
    damaged_metadata.extend_from_slice(b"\xff\xe1\x00\x08Exif\0\0");
    damaged_metadata.extend_from_slice(&plain[2..]);
    let (image, warnings) = image_load_with_warnings(&damaged_metadata).unwrap();
    let warnings = warnings.unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings.iter().next().unwrap().to_string().contains("EXIF"));
    assert_eq!((image.width, image.height), (reference.width, reference.height));
    assert_eq!(image.buffer, reference.buffer);
    assert_eq!(image_load(&damaged_metadata).unwrap().buffer, reference.buffer);
    let (limited, warnings) = image_from_with_limits_and_warnings(&damaged_metadata, DecodeLimits::unlimited()).unwrap();
    assert_eq!(limited.buffer, reference.buffer);
    assert_eq!(warnings.unwrap().len(), 1);

    let error = image_from_with_limits_and_warnings(&plain, DecodeLimits { pixels: 63, ..DecodeLimits::unlimited() }).err().unwrap();
    let cause = error.source().unwrap().downcast_ref::<DecodeLimitExceeded>().unwrap();
    assert_eq!((cause.requested(), cause.limit()), (64, 63));
}
