#![cfg(feature = "image-buffer-ex")]
use wml2::color::RGBA;
use wml2::draw::*;
use wml2::metadata::DataMap;

#[test]
fn incoming_declaration_is_distinct_from_storage_and_invalidated_on_failure() {
    let mut image = ImageBufferEx::with_storage(PixelFormatEx::Rgba16, PrecisionConversion::Round);
    assert_eq!(image.source_format(), None);
    assert_eq!(image.precision_conversion(), PrecisionConversion::Round);
    image
        .init_ex(
            2,
            1,
            Some(InitOptionsEx {
                source_format: PixelFormatEx::Gray12,
                source_stride: Some(5),
                legacy: None,
            }),
        )
        .unwrap();
    image
        .draw(0, 0, 2, 1, &[0xff, 0x0f, 0, 0, 0xaa], None)
        .unwrap();
    assert_eq!(image.source_format(), Some(PixelFormatEx::Gray12));
    assert_eq!(image.source_stride(), Some(5));
    assert_eq!(image.format(), PixelFormatEx::Rgba16);
    assert_eq!(image.stride(), 16);
    assert_eq!(image.pixel_bytes(0, 0).unwrap(), [255; 8]);
    assert!(image.init_ex(0, 1, None).is_err());
    assert_eq!(image.source_format(), None);
    assert!(image.pixel_bytes(0, 0).is_err());
    image.init(1, 1, None).unwrap();
    assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba8));
    assert_eq!(image.source_stride(), None);
}

#[test]
fn native_pixel_borrow_preserves_low_bits_alpha_and_odd_padding() {
    for format in [
        PixelFormatEx::Gray8,
        PixelFormatEx::Gray12,
        PixelFormatEx::Gray16,
        PixelFormatEx::Rgba8,
        PixelFormatEx::Rgba12,
        PixelFormatEx::Rgba16,
    ] {
        let bp = format.bytes_per_pixel();
        let stride = bp * 2 + 1;
        let max = ((1u32 << format.bits()) - 1) as u16;
        let mut bytes = vec![0xee; stride * 2];
        for y in 0..2 {
            for x in 0..2 {
                for c in 0..format.channels() {
                    let value = if c == 3 {
                        0
                    } else {
                        [1, max, 0, max / 2][y * 2 + x]
                    };
                    let at = y * stride + x * bp + c * format.bytes_per_sample();
                    if format.bits() == 8 {
                        bytes[at] = value as u8;
                    } else {
                        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
                    }
                }
            }
        }
        let image = ImageBufferEx::from_bytes(2, 2, format, Some(stride), bytes).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                let at = y * stride + x * bp;
                let pixel = image.pixel_bytes(x, y).unwrap();
                assert_eq!(pixel, &image.bytes().unwrap()[at..at + bp]);
                assert_eq!(pixel.as_ptr(), image.bytes().unwrap()[at..].as_ptr());
                assert_eq!(pixel.len(), bp);
            }
        }
        assert!(image.pixel_bytes(2, 0).is_err());
        assert!(image.pixel_bytes(0, 2).is_err());
        assert!(image.pixel_bytes(usize::MAX, usize::MAX).is_err());
    }
}

#[test]
fn checked_legacy_ownership_bridge_keeps_pixels_background_and_metadata() {
    let mut old = ImageBuffer::from_buffer(2, 1, vec![1, 2, 3, 0, 255, 128, 129, 255]);
    let allocation = old.buffer.as_ref().unwrap().as_ptr();
    old.background_color = Some(RGBA {
        red: 11,
        green: 22,
        blue: 33,
        alpha: 44,
    });
    old.set_metadata("note", DataMap::Ascii("retained".into()))
        .unwrap();
    let mut image = ImageBufferEx::try_from(old).unwrap();
    assert_eq!(image.bytes().unwrap().as_ptr(), allocation);
    assert_eq!(image.source_format(), Some(PixelFormatEx::Rgba8));
    assert_eq!(image.pixel_bytes(0, 0).unwrap(), &[1, 2, 3, 0]);
    let profile = image.encode_start_ex(None).unwrap().unwrap();
    let bg = profile.legacy.background.unwrap();
    assert_eq!((bg.red, bg.green, bg.blue, bg.alpha), (11, 22, 33, 44));
    assert!(
        matches!(profile.legacy.metadata.unwrap().get("note"), Some(DataMap::Ascii(s)) if s=="retained")
    );
    let old_again = image.to_rgba8(PrecisionConversion::Exact).unwrap();
    assert_eq!(old_again.buffer.unwrap(), &[1, 2, 3, 0, 255, 128, 129, 255]);
}

#[test]
fn legacy_bridge_rejects_mutable_layout_and_animation_state() {
    assert!(ImageBufferEx::try_from(ImageBuffer::new()).is_err());
    for length in [3, 5] {
        assert!(ImageBufferEx::try_from(ImageBuffer::from_buffer(1, 1, vec![0; length])).is_err());
    }
    let mut old = ImageBuffer::from_buffer(1, 1, vec![0; 4]);
    old.width = usize::MAX;
    assert!(ImageBufferEx::try_from(old).is_err());
    for field in 0..4 {
        let mut old = ImageBuffer::from_buffer(1, 1, vec![0; 4]);
        match field {
            0 => old.animation = Some(vec![]),
            1 => old.current = Some(0),
            2 => old.loop_count = Some(1),
            _ => old.first_wait_time = Some(0),
        }
        assert!(ImageBufferEx::try_from(old).is_err());
    }
}

#[test]
fn ordinary_legacy_static_initialization_can_transfer_ownership() {
    let mut old = ImageBuffer::new();
    old.init(1, 1, None).unwrap();
    old.draw(0, 0, 1, 1, &[1, 2, 3, 255], None).unwrap();
    assert_eq!(old.loop_count, Some(0));
    let ex = ImageBufferEx::try_from(old).unwrap();
    assert_eq!(ex.pixel_bytes(0, 0).unwrap(), &[1, 2, 3, 255]);
}

#[test]
fn explicit_legacy_view_converts_each_rectangle_and_retains_storage() {
    let values = [257u16, 514, 771, 65535, 128, 129, 32768, 0];
    let bytes: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut image =
        ImageBufferEx::from_bytes(2, 1, PixelFormatEx::Rgba16, None, bytes.clone()).unwrap();
    image
        .set_metadata("note", DataMap::Ascii("kept".into()))
        .unwrap();
    {
        let mut exact = image.legacy_picker(PrecisionConversion::Exact);
        assert_eq!(exact.encode_start(None).unwrap().unwrap().width, 2);
        assert_eq!(
            exact.encode_pick(0, 0, 1, 1, None).unwrap().unwrap(),
            [1, 2, 3, 255]
        );
        assert!(exact.encode_pick(1, 0, 1, 1, None).is_err());
        assert!(exact.encode_pick(0, 0, 3, 1, None).is_err());
        assert!(exact.encode_pick(0, 0, 0, 1, None).is_err());
        assert!(
            matches!(exact.metadata().unwrap().unwrap().get("note"),Some(DataMap::Ascii(s)) if s=="kept")
        );
        exact.encode_end(None).unwrap();
    }
    let mut rounded = image.legacy_picker(PrecisionConversion::Round);
    assert_eq!(
        rounded.encode_pick(1, 0, 1, 1, None).unwrap().unwrap(),
        [0, 1, 128, 0]
    );
    assert_eq!(image.bytes().unwrap(), bytes);
    assert!(
        ImageBufferEx::new()
            .legacy_picker(PrecisionConversion::Exact)
            .encode_start(None)
            .is_err()
    );
}

#[cfg(feature = "png")]
#[test]
fn png_encoder_accepts_the_explicit_view_without_losing_native_samples() {
    use wml2::util::ImageFormat;
    let values = [1u16, 128, 129, 257, 65535, 32768, 4095, 0];
    let bytes: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let image =
        ImageBufferEx::from_bytes(2, 1, PixelFormatEx::Rgba16, None, bytes.clone()).unwrap();
    let encoded = image_encoder(
        &mut EncodeOptions {
            debug_flag: 0,
            drawer: &mut image.legacy_picker(PrecisionConversion::Round),
            options: None,
        },
        ImageFormat::Png,
    )
    .unwrap();
    let decoded = image_from(&encoded).unwrap();
    assert_eq!(
        decoded.buffer.as_ref().unwrap(),
        &[0, 0, 1, 1, 255, 128, 16, 0]
    );
    let bridged = ImageBufferEx::try_from(decoded).unwrap();
    assert_eq!(bridged.pixel_bytes(1, 0).unwrap(), &[255, 128, 16, 0]);
    assert_eq!(image.pixel_bytes(0, 0).unwrap(), &bytes[..8]);
    assert_eq!(image.bytes().unwrap(), bytes);
    assert!(
        image_encoder(
            &mut EncodeOptions {
                debug_flag: 0,
                drawer: &mut image.legacy_picker(PrecisionConversion::Exact),
                options: None,
            },
            ImageFormat::Png
        )
        .is_err()
    );
}
