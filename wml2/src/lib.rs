/*
 * WML2 - Web graphic Multi format Library To Rust
 *  (C)Mith@mmk 2022
 *
 *  use MIT Licnce
 */
//! Multi-format image decoding and encoding for RGBA buffers.
//!
//! `wml2` exposes a callback-based decoding API in [`draw`] and built-in
//! buffer-backed helpers via [`draw::ImageBuffer`]. The crate can decode still
//! images and animations into RGBA buffers, preserve metadata, and encode BMP,
//! GIF, JPEG, PNG/APNG, TIFF, and WebP output.
//!
//! # Example
//! ```rust
//! use wml2::draw::*;
//! use wml2::metadata::DataMap;
//! use std::error::Error;
//! use std::env;
//!
//! pub fn main()-> Result<(),Box<dyn Error>> {
//!     let args: Vec<String> = env::args().collect();
//!     if args.len() < 2 {
//!         println!("usage: metadata <inputfilename>");
//!         return Ok(())
//!     }
//!
//!     let filename = &args[1];
//!     let mut image = image_from_file(filename.to_string())?;
//!     let metadata = image.metadata()?;
//!     if let Some(metadata) = metadata {
//!         for (key,value) in metadata {
//!             match value {
//!                 DataMap::None => {
//!                     println!("{}",key);
//!                 },
//!                 DataMap::Raw(value) => {
//!                     println!("{}: {}bytes",key,value.len());
//!                 },
//!                 DataMap::Ascii(string) => {
//!                     println!("{}: {}",key,string);
//!                 },
//!                 DataMap::Exif(value) => {
//!                     println!("=============== EXIF START ==============");
//!                     let string = value.to_string();
//!                     println!("{}", string);
//!                     println!("================ EXIF END ===============");
//!                 },
//!                 DataMap::ICCProfile(data) => {
//!                     println!("{}: {}bytes",key,data.len());
//!                 },
//!                 _ => {
//!                     println!("{}: {:?}",key,value);
//!                 }
//!             }
//!         }        
//!     }
//!     Ok(())
//! }
//! ```


use crate::util::ImageFormat;

// 0.0.19 new!
/// get_version get WML2 crate version
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// 0.0.19 new!
/// get_decoder_extentions get extentions of WML2's decoders
pub fn get_decoder_extentions() -> Vec<String> {
    vec![
        "bmp".to_string(),
        "gif".to_string(),
        "jpg".to_string(),
        "jpe".to_string(),
        "jpeg".to_string(),
        "png".to_string(),
        "tif".to_string(),
        "tiff".to_string(),
        "webp".to_string(),
#[cfg(not(feature = "noretoro"))]
        "mag".to_string(),
#[cfg(not(feature = "noretoro"))]
        "mki".to_string(),
#[cfg(not(feature = "noretoro"))]
        "pi".to_string(),
#[cfg(not(feature = "noretoro"))]
        "pic".to_string(),
    ]
}

// 0.0.19 new!
/// get_can_decode get WML2 crate decoder header check
pub fn get_can_decode(buffer: &[u8]) ->Result<bool, Box<dyn std::error::Error>> {
    let result = crate::util::format_check(buffer);
    match result {
        ImageFormat::Unknown => Ok(false),
        ImageFormat::RiffFormat(_) => Ok(false),
        _ => Ok(true)
    } 
}

// 0.0.19 new!
/// get_encode_extentions get extentions of WML2's encoders
pub fn get_encode_extentions() -> Vec<String> {
    vec![
        "bmp".to_string(),
        "gif".to_string(),
        "jpg".to_string(),
        "jpe".to_string(),
        "jpeg".to_string(),
        "png".to_string(),
        "tif".to_string(),
        "tiff".to_string(),
        "webp".to_string(),
    ]
}


//pub(crate) mod io; // move bin_rs crate
pub mod bmp;
pub mod draw;
pub mod encoder;
pub mod error;
pub mod gif;
pub mod jpeg;
pub mod mag;
#[cfg(not(feature = "noretoro"))]
pub mod maki;
#[cfg(not(feature = "noretoro"))]
pub mod pcd;
#[cfg(not(feature = "noretoro"))]
pub mod pi;
#[cfg(not(feature = "noretoro"))]
pub mod pic;
pub mod png;
#[cfg(not(feature = "noretoro"))]
mod retro;
pub mod tiff;
pub mod util;
#[cfg(not(feature = "noretoro"))]
pub mod vsp;
pub mod warning;
//pub mod iccprofile;
pub mod color;
pub mod decoder;
pub mod metadata;
pub mod webp;
