#![cfg(all(feature = "image-buffer-ex", feature = "avif"))]
use bin_rs::reader::BytesReader;
use wml2::draw::*;
use wml2::limits::DecodeLimits;
use wml2::metadata::DataMap;
use wml2::util::ImageFormat;

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

struct Probe {
    image: ImageBufferEx,
    events: Vec<&'static str>,
    abort: Option<&'static str>,
}
impl Probe {
    fn new(abort: Option<&'static str>) -> Self {
        Self {
            image: ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact),
            events: Vec::new(),
            abort,
        }
    }
    fn event(&mut self, event: &'static str) -> Response {
        self.events.push(event);
        Ok((self.abort == Some(event)).then(CallbackResponse::abort))
    }
}
impl DrawCallback for Probe {
    fn init(&mut self, _: usize, _: usize, _: Option<InitOptions>) -> Response {
        panic!("EX must declare its source")
    }
    fn init_ex(&mut self, w: usize, h: usize, o: Option<InitOptionsEx>) -> Response {
        assert_eq!(o.as_ref().unwrap().source_format, PixelFormatEx::Rgba16);
        assert_eq!(o.as_ref().unwrap().source_stride, None);
        self.image.init_ex(w, h, o)?;
        self.event("init")
    }
    fn draw(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        p: &[u8],
        o: Option<DrawOptions>,
    ) -> Response {
        self.image.draw(x, y, w, h, p, o)?;
        self.event("draw")
    }
    fn set_metadata(&mut self, k: &str, v: DataMap) -> Response {
        self.image.set_metadata(k, v)?;
        self.event("metadata")
    }
    fn terminate(&mut self, o: Option<TerminateOptions>) -> Response {
        self.image.terminate(o)?;
        self.event("end")
    }
    fn next(&mut self, _: Option<NextOptions>) -> Response {
        panic!("static only")
    }
    fn verbose(&mut self, _: &str, _: Option<VerboseOptions>) -> Response {
        Ok(None)
    }
}

fn decode_probe(
    data: &[u8],
    probe: &mut Probe,
    limits: DecodeLimits,
) -> Result<Option<wml2::warning::ImgWarnings>, Box<dyn std::error::Error>> {
    image_decoder_ex_with_limits(
        &mut BytesReader::new(data),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: probe,
        },
        limits,
    )
}

#[test]
fn decoder_only_uses_declared_rgba16_and_preserves_source_color_metadata() {
    let input = include_bytes!("fixtures/review/av1-full-omitted.avif");
    let native = avif_codec::decode_frame_bytes(input).unwrap();
    let expected =
        avif_codec::av1::frame_buffers_to_rgba_16(&native.buffers, &native.color_config).unwrap();
    let mut probe = Probe::new(None);
    decode_probe(input, &mut probe, DecodeLimits::default()).unwrap();
    assert_eq!(probe.image.bytes().unwrap(), words(&expected.rgba));
    assert_eq!(probe.events[0..2], ["init", "draw"]);
    assert_eq!(probe.events.last(), Some(&"end"));
    assert_eq!(
        probe.image.metadata.as_ref().unwrap()["AVIF Source Bit Depth"],
        DataMap::UInt(8)
    );
    let mut legacy = ImageBuffer::new();
    let error = image_decoder_ex(
        &mut BytesReader::new(input),
        &mut DecodeOptions {
            debug_flag: 0,
            drawer: &mut legacy,
        },
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("legacy callback requires packed RGBA8")
    );
    assert!(legacy.buffer.is_none());
}

#[test]
fn abort_stops_draw_and_metadata_forwarding_and_terminates_once() {
    for abort in ["init", "draw", "metadata"] {
        let mut probe = Probe::new(Some(abort));
        decode_probe(
            include_bytes!("fixtures/review/av1-full-omitted.avif"),
            &mut probe,
            DecodeLimits::default(),
        )
        .unwrap();
        let expected = match abort {
            "init" => vec!["init", "end"],
            "draw" => vec!["init", "draw", "end"],
            _ => vec!["init", "draw", "metadata", "end"],
        };
        assert_eq!(probe.events, expected);
    }
}

#[test]
fn malformed_and_short_limits_fail_before_callbacks_and_restore_scope() {
    let input = include_bytes!("fixtures/review/av1-full-omitted.avif");
    for limits in [
        DecodeLimits {
            input_bytes: input.len() - 1,
            ..Default::default()
        },
        DecodeLimits {
            pixels: 0,
            ..Default::default()
        },
        DecodeLimits {
            frames: 0,
            ..Default::default()
        },
        DecodeLimits {
            expanded_bytes: 1,
            ..Default::default()
        },
        DecodeLimits {
            metadata_bytes: 1,
            ..Default::default()
        },
    ] {
        let mut probe = Probe::new(None);
        assert!(decode_probe(input, &mut probe, limits).is_err());
        assert!(probe.events.is_empty());
    }
    for end in (0..input.len()).step_by(7) {
        let mut probe = Probe::new(None);
        assert!(decode_probe(&input[..end], &mut probe, DecodeLimits::default()).is_err());
        assert!(probe.events.is_empty());
    }
    assert!(image_from_ex(input, PixelFormatEx::Rgba16, PrecisionConversion::Exact).is_ok());
}

#[cfg(not(feature = "avifenc"))]
#[test]
fn avif_decode_feature_does_not_implicitly_enable_ex_encoding() {
    let mut image =
        ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba8, None, vec![0; 4]).unwrap();
    assert!(
        image_encoder_ex(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: &mut image,
                output: PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
                options: None
            },
            ImageFormat::Avif
        )
        .is_err()
    );
}

#[cfg(feature = "avifenc")]
mod encode {
    use super::*;
    use std::collections::HashMap;
    fn lossless() -> Option<HashMap<String, DataMap>> {
        Some(HashMap::from([("lossless".into(), DataMap::UInt(1))]))
    }
    fn encode(
        image: &mut dyn PickCallbackEx,
        output: PixelOutputEx,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        image_encoder_ex(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: image,
                output,
                options: lossless(),
            },
            ImageFormat::Avif,
        )
    }
    fn native(bits: u8, icc: Option<Vec<u8>>, hdr: bool) -> (Vec<u8>, Vec<u16>) {
        use avifenc_codec::*;
        let max = (1u16 << bits) - 1;
        let values = vec![
            1,
            2,
            3,
            0,
            max,
            max / 2,
            0,
            max,
            17,
            19,
            23,
            max / 3,
            max / 2,
            max / 3,
            max / 5,
            max,
        ];
        let order: &[usize] = if hdr { &[0] } else { &[1, 2, 0, 3] };
        let planes = order
            .iter()
            .enumerate()
            .map(|(i, &c)| PlaneBuffer {
                layout: PlaneLayout {
                    plane: i as u8,
                    width: 2,
                    height: 2,
                    subsampling_x: 0,
                    subsampling_y: 0,
                    sample_count: 4,
                },
                samples: values.chunks_exact(4).map(|p| p[c]).collect(),
            })
            .collect();
        let mut colors = NativeColorInformation::from_nclx(NclxColorInformation {
            color_primaries: 1,
            transfer_characteristics: if hdr { 16 } else { 13 },
            matrix_coefficients: if hdr { 1 } else { 0 },
            full_range_flag: true,
        });
        if let Some(profile) = icc {
            colors.icc_profile = Some(profile);
            colors.icc_color_type = Some(*b"prof");
        }
        let options = NativeEncodeOptions {
            bit_depth: bits,
            subsampling: if hdr {
                NativeSubsampling::Cs400
            } else {
                NativeSubsampling::Cs444
            },
            encoder: EncoderOptions {
                lossless: true,
                color_information: None,
                ..Default::default()
            },
            color_information: colors,
        };
        (
            encode_native_bytes(
                &FrameBuffers {
                    width: 2,
                    height: 2,
                    planes,
                },
                &options,
            )
            .unwrap(),
            values,
        )
    }

    #[test]
    fn independent_native_8_10_12bit_inputs_retain_normalized_samples_and_straight_alpha() {
        for bits in [8, 10, 12] {
            let (bytes, source) = native(bits, None, false);
            let image =
                image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            let max = (1u32 << bits) - 1;
            let expected: Vec<_> = source
                .iter()
                .map(|&s| ((u32::from(s) * 65535 + max / 2) / max) as u16)
                .collect();
            assert_eq!(image.bytes().unwrap(), words(&expected));
            assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba16));
            assert_eq!(
                image.metadata.as_ref().unwrap()["AVIF Source Bit Depth"],
                DataMap::UInt(bits.into())
            );
            if bits == 12 {
                assert!(
                    image_from_ex(&bytes, PixelFormatEx::Rgba12, PrecisionConversion::Exact)
                        .is_err()
                );
                assert_eq!(
                    image_from_ex(&bytes, PixelFormatEx::Rgba12, PrecisionConversion::Round)
                        .unwrap()
                        .bytes()
                        .unwrap(),
                    words(&source)
                );
            }
        }
    }

    #[test]
    fn legacy_avif_decoder_still_initializes_packed_rgba8_even_with_ex_enabled() {
        for bits in [8, 10, 12] {
            let (data, _) = native(bits, None, false);
            let expected = image_from(&data).unwrap().buffer.unwrap();
            let mut image =
                ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Exact);
            image_decoder(
                &mut BytesReader::new(&data),
                &mut DecodeOptions {
                    debug_flag: 0,
                    drawer: &mut image,
                },
            )
            .unwrap();
            assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba8));
            assert_eq!(image.bytes().unwrap().len(), expected.len() * 2);
            for (stored, byte) in image.bytes().unwrap().chunks_exact(2).zip(expected) {
                assert_eq!(
                    u16::from_le_bytes([stored[0], stored[1]]),
                    u16::from(byte) * 257
                );
            }
        }
    }

    #[test]
    fn opaque_icc_is_metadata_after_draw_and_is_not_applied_or_relabelled() {
        let profile = b"opaque profile; not a CMM transform".to_vec();
        let (bytes, _) = native(12, Some(profile.clone()), false);
        let (plain, _) = native(12, None, false);
        let mut probe = Probe::new(None);
        decode_probe(&bytes, &mut probe, DecodeLimits::default()).unwrap();
        assert_eq!(
            probe.image.bytes().unwrap(),
            image_from_ex(&plain, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                .unwrap()
                .bytes()
                .unwrap()
        );
        let metadata = probe.image.metadata.as_ref().unwrap();
        assert_eq!(metadata["Source ICC Profile"], DataMap::ICCProfile(profile));
        assert!(!metadata.contains_key("ICC Profile"));
        assert_eq!(probe.events[..2], ["init", "draw"]);
    }

    #[test]
    fn transfer_requiring_display_processing_is_explicitly_unsupported() {
        let (data, _) = native(8, None, true);
        let mut probe = Probe::new(None);
        assert!(
            decode_probe(&data, &mut probe, DecodeLimits::default())
                .unwrap_err()
                .to_string()
                .contains("external color processing")
        );
        assert!(probe.events.is_empty());
    }

    #[test]
    fn rgba8_and_twelve_lossless_encode_roundtrip_without_storage_mutation() {
        for format in [PixelFormatEx::Rgba8, PixelFormatEx::Rgba12] {
            let raw = if format.bits() == 8 {
                vec![1, 2, 3, 0, 255, 128, 0, 255]
            } else {
                words(&[1, 2, 3, 0, 4095, 2048, 0, 4095])
            };
            let mut image = ImageBufferEx::from_bytes(2, 1, format, None, raw.clone()).unwrap();
            let encoded = encode(
                &mut image,
                PixelOutputEx::new(format, PrecisionConversion::Exact),
            )
            .unwrap();
            assert!(
                encoded
                    .windows(14)
                    .any(|bytes| bytes == b"\0\0\0\x0eauxl\0\x02\0\x01\0\x01")
            );
            assert_eq!(image.bytes().unwrap(), raw);
            let decoded = image_from_ex(&encoded, format, PrecisionConversion::Round).unwrap();
            assert_eq!(decoded.bytes().unwrap(), raw);
            let native = avif_codec::decode_frame_bytes(&encoded).unwrap();
            assert_eq!(native.bit_depth, format.bits());
        }
    }

    // Optional export feeds verify_avif_registry_oracles.py. Assertions execute
    // in ordinary CI too; external readers are required only by that script.
    fn oracle(label: &str, bits: u8, bytes: &[u8], rgba: &[u16]) {
        let decoded = avif_codec::decode_frame_bytes(bytes).unwrap();
        assert_eq!(decoded.bit_depth, bits);
        for (role, channel) in [1, 2, 0, 3].into_iter().enumerate() {
            let plane = decoded
                .buffers
                .planes
                .iter()
                .find(|p| p.layout.plane as usize == role)
                .unwrap();
            assert_eq!(
                plane.samples,
                rgba.chunks_exact(4).map(|p| p[channel]).collect::<Vec<_>>()
            );
        }
        assert!(
            bytes
                .windows(14)
                .any(|b| b == b"\0\0\0\x0eauxl\0\x02\0\x01\0\x01")
        );
        if let Some(dir) = std::env::var_os("WML2_AVIF_REGISTRY_ORACLES") {
            let path = std::path::Path::new(&dir);
            assert!(path.is_dir(), "oracle output directory must already exist");
            std::fs::write(path.join(format!("{label}-{bits}.avif")), bytes).unwrap();
            std::fs::write(path.join(format!("{label}-{bits}.rgba16le")), words(rgba)).unwrap();
        }
    }

    #[test]
    fn published_writer_alpha_and_shared_padded_ex_match_exact_native_samples() {
        for (bits, format) in [(8, PixelFormatEx::Rgba8), (12, PixelFormatEx::Rgba12)] {
            let (upstream, source) = native(bits, None, false);
            oracle("upstream-native", bits, &upstream, &source);
            let raw = if bits == 8 {
                source.iter().map(|&v| v as u8).collect()
            } else {
                words(&source)
            };
            let mut image = ImageBufferEx::from_bytes(2, 2, format, None, raw.clone()).unwrap();
            let mut output = PixelOutputEx::new(format, PrecisionConversion::Exact);
            output.stride = Some(2 * format.bytes_per_pixel() + 1);
            let encoded = encode(&mut image, output).unwrap();
            oracle("wml2-ex", bits, &encoded, &source);
            assert_eq!(image.bytes().unwrap(), raw);
            let opts = avifenc_codec::EncodeOptionsEx {
                native: avifenc_codec::NativeEncodeOptions {
                    bit_depth: bits,
                    subsampling: avifenc_codec::NativeSubsampling::Cs444,
                    encoder: avifenc_codec::EncoderOptions {
                        lossless: true,
                        color_information: None,
                        ..Default::default()
                    },
                    color_information: avifenc_codec::NativeColorInformation::from_nclx(
                        avifenc_codec::NclxColorInformation {
                            color_primaries: 1,
                            transfer_characteristics: 13,
                            matrix_coefficients: 0,
                            full_range_flag: true,
                        },
                    ),
                },
                precision: avifenc_codec::buffer::PrecisionPolicy::Exact,
                color: avifenc_codec::ColorPolicyEx::Preserve,
            };
            // Construct via decoder's re-export and pass to encoder: both must
            // resolve one registry avif-image-buffer source and type identity.
            use avif_codec::buffer::{
                AlphaMode, Cicp, ColorMetadataRef, ImageViewEx, PackedChannels, PackedImageViewEx,
                PlaneLayoutEx, PlaneViewEx, SamplesRef,
            };
            let view = ImageViewEx::Packed(PackedImageViewEx {
                plane: PlaneViewEx {
                    samples: if bits == 8 {
                        SamplesRef::U8(&raw)
                    } else {
                        SamplesRef::U16LeBytes(&raw)
                    },
                    layout: PlaneLayoutEx {
                        width: 2,
                        height: 2,
                        valid_bits: bits,
                        offset_bytes: 0,
                        row_stride_bytes: 2 * format.bytes_per_pixel(),
                    },
                },
                channels: PackedChannels::Rgba,
                alpha: AlphaMode::Straight,
                color: ColorMetadataRef {
                    cicp: Some(Cicp {
                        primaries: 1,
                        transfer: 13,
                        matrix: 0,
                    }),
                    icc: None,
                },
            });
            let preview = avifenc_codec::preflight_ex(view, &opts).unwrap();
            assert!(!preview.encoded);
            let shared = avifenc_codec::encode_ex(view, &opts).unwrap();
            assert!(shared.report.encoded);
            oracle("upstream-shared", bits, &shared.bytes, &source);
            if bits == 8 {
                let bytes = avifenc_codec::encode_bytes(
                    &avifenc_codec::ImageBuffer {
                        width: 2,
                        height: 2,
                        rgba: raw.clone(),
                    },
                    &avifenc_codec::EncoderOptions {
                        lossless: true,
                        ..Default::default()
                    },
                )
                .unwrap();
                oracle("upstream-legacy", bits, &bytes, &source);
                let mut legacy = ImageBuffer::from_buffer(2, 2, raw);
                let bytes = image_to(&mut legacy, ImageFormat::Avif, lossless()).unwrap();
                oracle("wml2-legacy", bits, &bytes, &source);
            }
        }
    }

    #[test]
    fn sixteen_storage_requires_explicit_exact_or_round_output_policy() {
        let mut exact = ImageBufferEx::from_bytes(
            1,
            1,
            PixelFormatEx::Rgba16,
            None,
            words(&[4369, 21845, 65535, 34952]),
        )
        .unwrap();
        let data = encode(
            &mut exact,
            PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
        )
        .unwrap();
        assert_eq!(
            image_from_ex(&data, PixelFormatEx::Rgba12, PrecisionConversion::Round)
                .unwrap()
                .bytes()
                .unwrap(),
            words(&[273, 1365, 4095, 2184])
        );
        let mut image = ImageBufferEx::from_bytes(
            1,
            1,
            PixelFormatEx::Rgba16,
            None,
            words(&[1, 32768, 65535, 32768]),
        )
        .unwrap();
        let before = image.bytes().unwrap().to_vec();
        assert!(
            encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact)
            )
            .is_err()
        );
        assert!(
            encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba16, PrecisionConversion::Round)
            )
            .is_err()
        );
        let bytes = encode(
            &mut image,
            PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Round),
        )
        .unwrap();
        assert_eq!(
            image_from_ex(&bytes, PixelFormatEx::Rgba12, PrecisionConversion::Round)
                .unwrap()
                .bytes()
                .unwrap(),
            words(&[0, 2048, 4095, 2048])
        );
        assert_eq!(image.bytes().unwrap(), before);
    }

    #[test]
    fn gray8_twelve_and_odd_stride_pickers_are_explicit() {
        for format in [PixelFormatEx::Gray8, PixelFormatEx::Gray12] {
            let raw = if format.bits() == 8 {
                vec![0, 1, 128, 255]
            } else {
                words(&[0, 1, 2048, 4095])
            };
            let mut image = ImageBufferEx::from_bytes(2, 2, format, None, raw).unwrap();
            let mut output = PixelOutputEx::new(format, PrecisionConversion::Exact);
            output.stride = Some(format.bytes_per_pixel() * 2 + 1);
            let data = encode(&mut image, output).unwrap();
            let decoded =
                image_from_ex(&data, PixelFormatEx::Rgba16, PrecisionConversion::Exact).unwrap();
            assert_eq!(decoded.width(), 2);
            assert_eq!(decoded.height(), 2);
            let values = if format.bits() == 8 {
                [0u32, 1, 128, 255]
            } else {
                [0u32, 1, 2048, 4095]
            };
            let maximum = (1u32 << format.bits()) - 1;
            let expected: Vec<u16> = values
                .into_iter()
                .flat_map(|value| {
                    let scaled = ((value * 65535 + maximum / 2) / maximum) as u16;
                    [scaled, scaled, scaled, 65535]
                })
                .collect();
            assert_eq!(decoded.bytes().unwrap(), words(&expected));
        }
    }

    struct Picker {
        image: ImageBufferEx,
        ends: usize,
        wrong: bool,
    }
    impl PickCallbackEx for Picker {
        fn encode_start_ex(
            &mut self,
            o: Option<EncoderOptions>,
        ) -> Result<Option<ImageProfilesEx>, Box<dyn std::error::Error>> {
            self.image.encode_start_ex(o)
        }
        fn encode_pick_ex(
            &mut self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            mut out: PixelOutputEx,
            o: Option<PickOptions>,
        ) -> Result<Option<PixelBytesEx>, Box<dyn std::error::Error>> {
            if self.wrong {
                out.format = PixelFormatEx::Rgba16;
            }
            self.image.encode_pick_ex(x, y, w, h, out, o)
        }
        fn encode_end_ex(
            &mut self,
            _: Option<EndOptions>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.ends += 1;
            Ok(())
        }
        fn metadata_ex(
            &mut self,
        ) -> Result<Option<HashMap<String, DataMap>>, Box<dyn std::error::Error>> {
            self.image.metadata_ex()
        }
    }
    #[test]
    fn encoder_rejects_wrong_layout_and_finalizes_once_after_pick_errors() {
        for wrong in [false, true] {
            let mut picker = Picker {
                image: ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba16, None, words(&[1; 4]))
                    .unwrap(),
                ends: 0,
                wrong,
            };
            let before = picker.image.bytes().unwrap().to_vec();
            assert!(
                encode(
                    &mut picker,
                    PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact)
                )
                .is_err()
            );
            assert_eq!(picker.ends, 1);
            assert_eq!(picker.image.bytes().unwrap(), before);
        }
    }

    #[test]
    fn backend_preflight_failure_finalizes_once_without_changing_storage() {
        let raw = words(&[1, 2, 3, 0]);
        let mut picker = Picker {
            image: ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba12, None, raw.clone())
                .unwrap(),
            ends: 0,
            wrong: false,
        };
        let error = image_encoder_ex(
            &mut EncodeOptionsEx {
                debug_flag: 0,
                drawer: &mut picker,
                output: PixelOutputEx::new(PixelFormatEx::Rgba12, PrecisionConversion::Exact),
                options: Some(HashMap::from([("speed".into(), DataMap::UInt(11))])),
            },
            ImageFormat::Avif,
        )
        .unwrap_err();
        assert!(error.to_string().contains("speed"));
        assert!(error.downcast_ref::<wml2::error::ImgError>().is_some());
        assert_eq!(picker.ends, 1);
        assert_eq!(picker.image.bytes().unwrap(), raw);
    }

    #[test]
    fn output_icc_is_explicit_and_encoded_limits_restore_after_error() {
        let mut image =
            ImageBufferEx::from_bytes(1, 1, PixelFormatEx::Rgba8, None, vec![1, 2, 3, 255])
                .unwrap();
        image
            .set_metadata("Source ICC Profile", DataMap::ICCProfile(vec![5, 6, 7]))
            .unwrap();
        let bytes = encode(
            &mut image,
            PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
        )
        .unwrap();
        assert!(
            avif_codec::parse_native_info(
                &bytes,
                &avif_codec::NativeDecodeLimits::new(
                    4096, 16, 16, 256, 4096, 4096, 4096, 128, 512, 128, 16, 1
                )
            )
            .unwrap()
            .color_information()
            .icc_profile
            .is_none()
        );
        image
            .set_metadata("ICC Profile", DataMap::ICCProfile(vec![1, 2, 3]))
            .unwrap();
        let bytes = encode(
            &mut image,
            PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
        )
        .unwrap();
        assert_eq!(
            image_from_ex(&bytes, PixelFormatEx::Rgba16, PrecisionConversion::Exact)
                .unwrap()
                .metadata
                .unwrap()["Source ICC Profile"],
            DataMap::ICCProfile(vec![1, 2, 3])
        );
        assert!(
            image_encoder_ex_with_limits(
                &mut EncodeOptionsEx {
                    debug_flag: 0,
                    drawer: &mut image,
                    output: PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact),
                    options: lossless()
                },
                ImageFormat::Avif,
                DecodeLimits {
                    pixels: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            encode(
                &mut image,
                PixelOutputEx::new(PixelFormatEx::Rgba8, PrecisionConversion::Exact)
            )
            .is_ok()
        );
    }
}
