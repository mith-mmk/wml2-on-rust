//! `wml2` draw-side adapter for the standalone `avif-rust` decoder.

type Error = Box<dyn std::error::Error>;

use crate::draw::{
    DecodeOptions, DrawOptions, ImageRect, InitOptions, NextBlend, NextDispose, NextOption,
    NextOptions, TerminateOptions, VerboseOptions,
};
use crate::error::{ImgError, ImgErrorKind};
use crate::metadata::DataMap;
use crate::warning::ImgWarnings;
use bin_rs::reader::BinaryReader;
#[path = "managed.rs"]
mod managed;

#[cfg(feature = "image-buffer-ex")]
#[path = "ex.rs"]
mod ex;
#[cfg(feature = "image-buffer-ex")]
pub(crate) use ex::decode as decode_ex_inner;

fn map_decoder_error(error: &avif_codec::DecoderError) -> ImgErrorKind {
    match error {
        avif_codec::DecoderError::InvalidParam(_) => ImgErrorKind::InvalidParameter,
        avif_codec::DecoderError::NotEnoughData(_) => ImgErrorKind::UnexpectedEof,
        avif_codec::DecoderError::Bitstream(_) => ImgErrorKind::IllegalData,
        avif_codec::DecoderError::Unsupported(_) => ImgErrorKind::UnsupportedFeature,
        avif_codec::DecoderError::Io(_) => ImgErrorKind::IOError,
        _ => ImgErrorKind::DecodeError,
    }
}

fn map_error(error: Error) -> Error {
    match error.downcast::<avif_codec::DecoderError>() {
        Ok(decoder_error) => Box::new(ImgError::new_const(
            map_decoder_error(&decoder_error),
            decoder_error.to_string(),
        )),
        Err(error) => Box::new(ImgError::new_const(
            ImgErrorKind::DecodeError,
            error.to_string(),
        )),
    }
}

fn compat_datamap(value: avif_codec::DataMap) -> DataMap {
    match value {
        avif_codec::DataMap::UInt(value) => DataMap::UInt(value),
        avif_codec::DataMap::UIntAllay(value) => DataMap::UIntAllay(value),
        avif_codec::DataMap::Raw(value) => DataMap::Raw(value),
        avif_codec::DataMap::Ascii(value) => DataMap::Ascii(value),
        avif_codec::DataMap::None => DataMap::None,
    }
}

struct DrawerAdapter<'a> {
    drawer: &'a mut dyn crate::draw::DrawCallback,
}

impl avif_codec::DrawCallback for DrawerAdapter<'_> {
    fn init(
        &mut self,
        width: usize,
        height: usize,
        option: Option<avif_codec::InitOptions>,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        let option = option.map(|option| InitOptions {
            loop_count: option.loop_count,
            background: None,
            animation: option.animation,
        });
        self.drawer.init(width, height, option).map(|response| {
            response.map(|response| {
                if response.response == crate::draw::ResponseCommand::Abort {
                    avif_codec::CallbackResponse::abort()
                } else {
                    avif_codec::CallbackResponse::cont()
                }
            })
        })
    }

    fn draw(
        &mut self,
        start_x: usize,
        start_y: usize,
        width: usize,
        height: usize,
        data: &[u8],
        _option: Option<avif_codec::DrawOptions>,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        self.drawer
            .draw(start_x, start_y, width, height, data, None::<DrawOptions>)
            .map(|response| {
                response.map(|response| {
                    if response.response == crate::draw::ResponseCommand::Abort {
                        avif_codec::CallbackResponse::abort()
                    } else {
                        avif_codec::CallbackResponse::cont()
                    }
                })
            })
    }

    fn next(
        &mut self,
        option: Option<avif_codec::NextOptions>,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        let option = option.map(|option| NextOptions {
            flag: NextOption::Continue,
            await_time: option.await_time,
            image_rect: option.image_rect.map(|rect| ImageRect {
                start_x: rect.start_x,
                start_y: rect.start_y,
                width: rect.width,
                height: rect.height,
            }),
            dispose_option: Some(match option.dispose {
                avif_codec::NextDispose::None => NextDispose::None,
                avif_codec::NextDispose::Background => NextDispose::Background,
                avif_codec::NextDispose::Previous => NextDispose::Previous,
            }),
            blend: Some(match option.blend {
                avif_codec::NextBlend::Source => NextBlend::Source,
                avif_codec::NextBlend::Override => NextBlend::Override,
            }),
        });
        self.drawer.next(option).map(|response| {
            response.map(|response| {
                if response.response == crate::draw::ResponseCommand::Abort {
                    avif_codec::CallbackResponse::abort()
                } else {
                    avif_codec::CallbackResponse::cont()
                }
            })
        })
    }

    fn terminate(
        &mut self,
        _term: Option<avif_codec::TerminateOptions>,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        self.drawer
            .terminate(None::<TerminateOptions>)
            .map(|response| {
                response.map(|response| {
                    if response.response == crate::draw::ResponseCommand::Abort {
                        avif_codec::CallbackResponse::abort()
                    } else {
                        avif_codec::CallbackResponse::cont()
                    }
                })
            })
    }

    fn verbose(
        &mut self,
        verbose: &str,
        _option: Option<avif_codec::VerboseOptions>,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        self.drawer
            .verbose(verbose, None::<VerboseOptions>)
            .map(|response| {
                response.map(|response| {
                    if response.response == crate::draw::ResponseCommand::Abort {
                        avif_codec::CallbackResponse::abort()
                    } else {
                        avif_codec::CallbackResponse::cont()
                    }
                })
            })
    }

    fn set_metadata(
        &mut self,
        key: &str,
        value: avif_codec::DataMap,
    ) -> Result<Option<avif_codec::CallbackResponse>, Error> {
        self.drawer
            .set_metadata(key, compat_datamap(value))
            .map(|response| {
                response.map(|response| {
                    if response.response == crate::draw::ResponseCommand::Abort {
                        avif_codec::CallbackResponse::abort()
                    } else {
                        avif_codec::CallbackResponse::cont()
                    }
                })
            })
    }
}

pub fn decode<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    crate::limits::metadata_scope(|| decode_inner(reader, option))
}

fn native_limits(limits: crate::limits::DecodeLimits) -> avif_codec::NativeDecodeLimits {
    avif_codec::NativeDecodeLimits::new(
        limits.input_bytes,
        u32::MAX as usize,
        u32::MAX as usize,
        limits.pixels,
        limits.expanded_bytes,
        limits.metadata_bytes,
        limits.metadata_bytes,
        128,
        512,
        128,
        16,
        limits.frames.min(1),
    )
}

fn decode_inner<B: BinaryReader>(
    reader: &mut B,
    option: &mut DecodeOptions,
) -> Result<Option<ImgWarnings>, Error> {
    let limits = crate::limits::current();
    let at = reader.offset()?;
    let end = reader.seek(std::io::SeekFrom::End(0))?;
    let length = end.checked_sub(at).ok_or("invalid AVIF input position")?;
    crate::limits::check_input_length(length, limits.input_bytes)?;
    let mut input = crate::draw::zeroed_bytes(usize::try_from(length)?)?;
    reader.seek(std::io::SeekFrom::Start(at))?;
    reader.read_exact(&mut input)?;
    if !crate::avif::signaling::is_sequence(&input)?
        && crate::avif::signaling::has_profile_or_prem(&input)?
    {
        let info = avif_codec::parse_native_info(&input, &native_limits(limits))
            .map_err(|e| map_error(Box::new(e)))?;
        let needs_owned = info.color_information().icc_profile.is_some()
            || crate::avif::signaling::alpha_premultiplied(&input, info.info())?;
        drop(info);
        if needs_owned {
            return managed::decode(&input, option);
        }
    }
    let mut adapter = DrawerAdapter {
        drawer: option.drawer,
    };
    let mut compat_option = avif_codec::DecodeOptions::new(&mut adapter);
    compat_option.debug_flag = option.debug_flag;
    avif_codec::decode(
        &mut bin_rs::reader::BytesReader::new(&input),
        &mut compat_option,
    )
    .map_err(map_error)?;
    Ok(None)
}
