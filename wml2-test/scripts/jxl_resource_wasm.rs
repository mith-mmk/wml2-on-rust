//! Typed-result supplement: unlike the existing display harness, this never
//! unwraps a decode error. Single-threaded, trusted fixed internal-layer input.
use wml2::limits::DecodeLimits;
static INPUT: &[u8] = include_bytes!("../../wml2/tests/fixtures/review/jxl-sunset-logo.jxl");
static mut ERROR: [u8; 512] = [0; 512];
static mut ERROR_LEN: u32 = 0;
static mut WIDTH: u32 = 0;
static mut HEIGHT: u32 = 0;
#[unsafe(no_mangle)]
pub extern "C" fn error_ptr() -> *const u8 {
    (&raw const ERROR).cast::<u8>()
}
#[unsafe(no_mangle)]
pub extern "C" fn error_len() -> u32 {
    unsafe { ERROR_LEN }
}
#[unsafe(no_mangle)]
pub extern "C" fn width() -> u32 {
    unsafe { WIDTH }
}
#[unsafe(no_mangle)]
pub extern "C" fn height() -> u32 {
    unsafe { HEIGHT }
}
#[unsafe(no_mangle)]
pub extern "C" fn evaluate_layer(expanded_mib: u32) -> u32 {
    // SAFETY: this development harness has one thread; exported pointers are
    // read only after the synchronous call. No Rust references escape.
    unsafe {
        WIDTH = 0;
        HEIGHT = 0;
        ERROR_LEN = 0;
    }
    let limits = DecodeLimits {
        expanded_bytes: expanded_mib as usize * 1024 * 1024,
        ..Default::default()
    };
    match wml2::jpegxl::image_from_srgb_with_limits(INPUT, limits) {
        Ok(image) => {
            unsafe {
                WIDTH = image.width as u32;
                HEIGHT = image.height as u32;
            }
            0
        }
        Err(error) => {
            let text = error.to_string();
            let bytes = text.as_bytes();
            let n = bytes.len().min(512);
            // SAFETY: n is bounded by both source and static destination sizes.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), (&raw mut ERROR).cast::<u8>(), n);
                ERROR_LEN = n as u32;
            }
            1
        }
    }
}
