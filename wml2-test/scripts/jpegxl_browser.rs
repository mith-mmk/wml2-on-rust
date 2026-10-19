//! Standalone validation harness; no browser/JS dependency in the library.
use wml2::draw::*;
const FIXTURES: &str = "independent libjxl/djxl/LittleCMS fixtures";
macro_rules! orientation {
    ($n:literal) => { (
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-orientation",$n,".jxl")).as_slice(),
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-orientation",$n,".rgba")).as_slice(),
        if $n<=4 {2}else{3}, if $n<=4 {3}else{2}, false,false,
    ) };
}
macro_rules! managed {
    ($name:literal) => { (
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-icc-",$name,".jxl")).as_slice(),
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-icc-",$name,".srgb.rgba")).as_slice(),7,5,true,false,
    ) };
}
macro_rules! official {
    ($name:literal,$width:literal,$height:literal) => { (
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-jbrd-",$name,".jxl")).as_slice(),
        include_bytes!(concat!("../../wml2/tests/fixtures/review/jxl-jbrd-",$name,".srgb.rgba.zlib")).as_slice(),$width,$height,true,true,
    ) };
}
static mut FRAME: [u8;1212000]=[0;1212000];
static mut WIDTH:u32=0;
static mut HEIGHT:u32=0;
#[unsafe(no_mangle)]
pub extern "C" fn case_count()->u32 {13}
#[unsafe(no_mangle)]
pub extern "C" fn width()->u32 {unsafe{WIDTH}}
#[unsafe(no_mangle)]
pub extern "C" fn height()->u32 {unsafe{HEIGHT}}
#[unsafe(no_mangle)]
pub extern "C" fn render_case(index:u32)->*const u8 {
    let cases=[orientation!(1),orientation!(2),orientation!(3),orientation!(4),orientation!(5),orientation!(6),orientation!(7),orientation!(8),managed!("gray16"),managed!("lossless"),managed!("lossy"),official!("bench",606,500),official!("gray",200,200)];
    let (input,expected,width,height,managed,compressed)=cases[index as usize];
    let decoded;let expected=if compressed {decoded=miniz_oxide::inflate::decompress_to_vec_zlib(expected).unwrap();decoded.as_slice()} else {expected};
    let image=if managed {wml2::jpegxl::image_from_srgb(input)}else{image_from(input)}.expect(FIXTURES);
    assert_eq!((image.width,image.height),(width,height));let rgba=image.buffer.unwrap();assert_eq!(rgba.len(),expected.len());
    for (a,b) in rgba.iter().zip(expected) {assert!(a.abs_diff(*b)<=if managed {2}else{0});}
    if !managed {
        let native=image_from_ex(input,PixelFormatEx::Rgba8,PrecisionConversion::Exact).unwrap();assert_eq!(native.bytes().unwrap(),expected);
    } else {
        assert!(matches!(&image.metadata.unwrap()["ICC Profile"],wml2::metadata::DataMap::ICCProfile(v) if v==include_bytes!("../../wml2/src/srgb.icc")));
    }
    // Single-threaded WASM test artifact; no references to mutable statics escape.
    unsafe {WIDTH=width as u32;HEIGHT=height as u32;let ptr=(&raw mut FRAME).cast::<u8>();std::ptr::copy_nonoverlapping(rgba.as_ptr(),ptr,rgba.len());ptr}
}
