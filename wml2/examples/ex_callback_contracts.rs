//! Input declaration, independent storage and explicit encoder output.
//! `cargo run -p wml2 --no-default-features --features image-buffer-ex
//! --example ex_callback_contracts` (add `png,tiff` for optional file decoding).

#[cfg(feature = "image-buffer-ex")]
pub mod contracts {
    use wml2::draw::*;
    use wml2::metadata::{DataMap, Metadata};
    use wml2::warning::ImgWarnings;

    pub type Error = Box<dyn std::error::Error>;

    pub fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    pub fn source(format: PixelFormatEx) -> Option<InitOptionsEx> {
        Some(InitOptionsEx {
            source_format: format,
            ..Default::default()
        })
    }

    /// One fresh wrapper per static image. Metadata may arrive before init or
    /// after draw, so retain it separately and do no color work inside draw.
    /// This callback never returns Abort. No ICC parser/CMM is implemented here.
    pub struct BufferedIccCallback {
        image: ImageBufferEx,
        tags: Metadata,
        terminated: bool,
    }

    impl BufferedIccCallback {
        pub fn new(storage: PixelFormatEx, conversion: PrecisionConversion) -> Self {
            Self {
                image: ImageBufferEx::with_storage(storage, conversion),
                tags: Metadata::new(),
                terminated: false,
            }
        }

        /// Supply the actual decoder result, not merely terminate's response.
        /// This example rejects every warning because PNG can terminate early
        /// with a warning before final metadata delivery. Errors/warnings do
        /// not run the external stage or publish partial pixels.
        pub fn after_decode<T>(
            self,
            status: Result<Option<ImgWarnings>, Error>,
            external_stage: impl FnOnce(ImageBufferEx, Metadata) -> Result<T, Error>,
        ) -> Result<T, Error> {
            if status?.is_some() {
                return Err(
                    "example requires a warning-free decode before color processing".into(),
                );
            }
            if !self.terminated || self.image.bytes().is_none() {
                return Err("example requires a terminated, initialized static image".into());
            }
            external_stage(self.image, self.tags)
        }
    }

    impl DrawCallback for BufferedIccCallback {
        fn init(&mut self, w: usize, h: usize, option: Option<InitOptions>) -> Response {
            self.terminated = false;
            self.image.init(w, h, option)
        }
        fn init_ex(&mut self, w: usize, h: usize, option: Option<InitOptionsEx>) -> Response {
            self.terminated = false;
            self.image.init_ex(w, h, option)
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            data: &[u8],
            option: Option<DrawOptions>,
        ) -> Response {
            self.image.draw(x, y, w, h, data, option)
        }
        fn terminate(&mut self, option: Option<TerminateOptions>) -> Response {
            let result = self.image.terminate(option)?;
            self.terminated = true;
            Ok(result)
        }
        fn next(&mut self, option: Option<NextOptions>) -> Response {
            self.image.next(option)
        }
        fn verbose(&mut self, text: &str, option: Option<VerboseOptions>) -> Response {
            self.image.verbose(text, option)
        }
        fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
            self.tags.insert(key.to_owned(), value);
            Ok(None)
        }
    }

    /// No init_ex override: the trait's default rejects Gray/high-depth or
    /// explicit-stride declarations before calling this legacy initializer.
    pub struct LegacyOnly(pub ImageBuffer);
    impl Default for LegacyOnly {
        fn default() -> Self {
            Self(ImageBuffer::new())
        }
    }
    impl DrawCallback for LegacyOnly {
        fn init(&mut self, w: usize, h: usize, option: Option<InitOptions>) -> Response {
            self.0.init(w, h, option)
        }
        fn draw(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            data: &[u8],
            option: Option<DrawOptions>,
        ) -> Response {
            self.0.draw(x, y, w, h, data, option)
        }
        fn terminate(&mut self, option: Option<TerminateOptions>) -> Response {
            self.0.terminate(option)
        }
        fn next(&mut self, option: Option<NextOptions>) -> Response {
            self.0.next(option)
        }
        fn verbose(&mut self, text: &str, option: Option<VerboseOptions>) -> Response {
            self.0.verbose(text, option)
        }
        fn set_metadata(&mut self, key: &str, value: DataMap) -> Response {
            self.0.set_metadata(key, value)
        }
    }

    pub fn run() -> Result<(), Error> {
        // 1. Legacy init declares packed RGBA8, although storage is RGBA16.
        let mut image =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
        image.init(1, 1, None)?;
        image.draw(0, 0, 1, 1, &[1, 128, 255, 0], None)?;
        assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba8));
        assert_eq!(image.bytes().unwrap(), words(&[257, 32896, 65535, 0]));

        // 2. Decoder declares RGBA16. Callback owns 16->12 precision policy.
        // Exact is normalized scaling, not a four-bit shift: 4369 -> 273.
        let representable = words(&[4369, 21845, 65535, 34952]);
        let mut exact =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Exact);
        exact.init_ex(2, 1, source(PixelFormatEx::Rgba16))?;
        exact.draw(0, 0, 1, 1, &representable, None)?;
        assert_eq!(exact.pixel_bytes(0, 0)?, words(&[273, 1365, 4095, 2184]));
        let before = exact.bytes().unwrap().to_vec();
        let mut late_error = words(&[65535; 4]); // would change the first pixel
        late_error.extend(words(&[65535, 65535, 65535, 1])); // final alpha fails
        assert!(exact.draw(0, 0, 2, 1, &late_error, None).is_err());
        assert_eq!(exact.bytes().unwrap(), before); // entire failed draw atomic

        let mut rounded =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Round);
        rounded.init_ex(1, 1, source(PixelFormatEx::Rgba16))?;
        rounded.draw(0, 0, 1, 1, &words(&[1, 32768, 65535, 32768]), None)?;
        assert_eq!(rounded.bytes().unwrap(), words(&[0, 2048, 4095, 2048]));

        // 3. RGBA12->RGBA12 preserves every addressed sample, including hidden RGB.
        let native = words(&[1, 1234, 4095, 0]);
        let mut twelve =
            ImageBufferEx::with_storage(PixelFormatEx::Rgba12, PrecisionConversion::Exact);
        twelve.init_ex(1, 1, source(PixelFormatEx::Rgba12))?;
        twelve.draw(0, 0, 1, 1, &native, None)?;
        let profile = twelve.encode_start_ex(None)?.ok_or("missing profile")?;
        assert_eq!(profile.format, PixelFormatEx::Rgba12); // storage, not codec negotiation
        let same = twelve
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
                None,
            )?
            .ok_or("missing pixels")?;
        assert_eq!(same.bytes(), native);
        // A consumer whose documented input is packed RGBA8 must ask for it.
        let output = twelve
            .encode_pick_ex(
                0,
                0,
                1,
                1,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Round),
                None,
            )?
            .ok_or("missing pixels")?;
        assert_eq!(output.bytes(), &[0, 77, 255, 0]);
        assert_eq!(twelve.bytes().unwrap(), native);
        twelve.encode_end_ex(None)?;
        #[cfg(feature = "png")]
        {
            // Current image_encoder codecs use legacy PickCallback RGBA8.
            // PickCallbackEx is an explicit consumer interface, not a native
            // high-depth PNG encoder. Use the existing explicit adapter.
            let png = image_encoder(
                &mut EncodeOptions {
                    debug_flag: 0,
                    drawer: &mut twelve.legacy_picker(PrecisionConversion::Round),
                    options: None,
                },
                wml2::util::ImageFormat::Png,
            )?;
            assert_eq!(image_from(&png)?.buffer.unwrap(), output.bytes());
        }

        // 4. Scripted stream: ICC intentionally arrives after draw.
        // Bytes are only a marker, not a valid ICC profile or a CMM transform.
        let mut buffered =
            BufferedIccCallback::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
        buffered.init_ex(1, 1, source(PixelFormatEx::Rgba16))?;
        buffered.draw(0, 0, 1, 1, &representable, None)?;
        buffered.set_metadata(
            "Source ICC Profile",
            DataMap::ICCProfile(b"opaque example marker".to_vec()),
        )?;
        buffered.terminate(None)?;
        buffered.after_decode(Ok(None), |pixels, tags| {
            assert!(matches!(
                tags.get("Source ICC Profile"),
                Some(DataMap::ICCProfile(_))
            ));
            assert_eq!(pixels.bytes().unwrap(), representable);
            // INSERT EXTERNAL CMM HERE: validate profile/layout, choose target
            // profile/intent, transform color channels, preserve straight alpha.
            // This example deliberately keeps source pixels and metadata apart;
            // it neither transforms colors nor labels source ICC as output ICC.
            Ok(())
        })?;

        // 5. Old callback rejects RGBA16 before its legacy init can run.
        let mut old = LegacyOnly::default();
        let error = old
            .init_ex(1, 1, source(PixelFormatEx::Rgba16))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("legacy callback requires packed RGBA8")
        );
        assert!(old.0.buffer.is_none());
        old.init_ex(1, 1, None)?; // default declaration is packed RGBA8
        old.draw(0, 0, 1, 1, &[1, 2, 3, 255], None)?;
        assert_eq!(old.0.buffer.as_deref().unwrap(), &[1, 2, 3, 255]);

        println!("5 callback scenarios passed; ICC collection only, no color transform");
        Ok(())
    }

    #[cfg(any(feature = "png", feature = "tiff"))]
    pub fn decode_file(path: &str) -> Result<(), Error> {
        use bin_rs::reader::BytesReader;
        let input = std::fs::read(path)?;
        let mut callback =
            BufferedIccCallback::new(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
        let status = image_decoder_ex(
            &mut BytesReader::new(&input),
            &mut DecodeOptions {
                debug_flag: 0,
                drawer: &mut callback,
            },
        );
        callback.after_decode(status, |pixels, tags| {
            let icc_bytes = match tags.get("Source ICC Profile") {
                Some(DataMap::ICCProfile(profile)) => profile.len(),
                _ => 0,
            };
            println!("{}x{} {:?} input -> {:?} storage; {icc_bytes} source ICC bytes collected; no transform",
                pixels.width(), pixels.height(), pixels.source_format(), pixels.format());
            // External color processing belongs here, before output picking.
            // Gray ICC cannot simply be applied as an RGB profile after Gray
            // replication. Validate compatibility or retain native Gray storage.
            Ok(())
        })
    }
}

#[cfg(feature = "image-buffer-ex")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    contracts::run()?;
    #[cfg(any(feature = "png", feature = "tiff"))]
    if let Some(path) = std::env::args().nth(1) {
        contracts::decode_file(&path)?;
    }
    Ok(())
}

#[cfg(not(feature = "image-buffer-ex"))]
fn main() {
    eprintln!("ex_callback_contracts requires --features image-buffer-ex");
}
