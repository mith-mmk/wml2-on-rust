/*
 * WML2 - Web graphic Multi format Library To Rust
 *  (C)Mith@mmk 2022
 * 
 *  use MIT Licnce
 */
//! Sample
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


//pub(crate) mod io; // move bin_rs crate
pub mod draw;
pub mod jpeg;
pub mod tiff;
pub mod bmp;
pub mod gif;
pub mod png;
pub mod error;
pub mod warning;
pub mod util;
pub mod iccprofile;
pub mod color;
pub mod decoder;
pub mod metadata;
