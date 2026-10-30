use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Cursor;
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
use std::io::{self, Read, Seek, SeekFrom};
use wml2::image_info::*;
use wml2::limits::DecodeLimits;
struct Allocations;
thread_local! {static TRACK:Cell<bool>=const{Cell::new(false)};static LARGEST:Cell<usize>=const{Cell::new(0)};}
unsafe impl GlobalAlloc for Allocations {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TRACK.with(|t| {
            if t.get() {
                LARGEST.with(|m| m.set(m.get().max(layout.size())))
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        TRACK.with(|t| {
            if t.get() {
                LARGEST.with(|m| m.set(m.get().max(new_size)))
            }
        });
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static ALLOC: Allocations = Allocations;
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
fn tiny_limits() -> DecodeLimits {
    DecodeLimits {
        pixels: 0,
        expanded_bytes: 8192,
        metadata_bytes: 4096,
        frames: 10,
        animation_bytes: 0,
        ..DecodeLimits::unlimited()
    }
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
struct DenyReads {
    source: Cursor<Vec<u8>>,
    forbidden: Vec<std::ops::Range<u64>>,
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
impl Read for DenyReads {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let p = self.source.position();
        let end = p + out.len() as u64;
        if self.forbidden.iter().any(|r| p < r.end && r.start < end) {
            return Err(io::Error::other("forbidden payload read"));
        }
        self.source.read(out)
    }
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
impl Seek for DenyReads {
    fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
        self.source.seek(p)
    }
}
#[cfg(any(
    feature = "jpeg",
    feature = "png",
    feature = "gif",
    feature = "tiff",
    feature = "psd"
))]
fn no_expansion(source: Vec<u8>, forbidden: Vec<std::ops::Range<u64>>) -> ImageInfo {
    let mut reader = DenyReads {
        source: Cursor::new(source),
        forbidden,
    };
    reader.seek(SeekFrom::Start(3)).unwrap();
    LARGEST.with(|m| m.set(0));
    TRACK.with(|t| t.set(true));
    let result = image_info_from_reader_with_limits(&mut reader, tiny_limits());
    TRACK.with(|t| t.set(false));
    let info = result.unwrap();
    assert_eq!(reader.source.position(), 3);
    assert!(LARGEST.with(|m| m.get()) <= 8192);
    info
}
#[test]
fn unknown_and_disabled_formats_have_explicit_errors_and_restore_reader() {
    let mut reader = Cursor::new(b"not an image".to_vec());
    reader.set_position(5);
    assert!(matches!(
        image_info_from_reader(&mut reader),
        Err(ImageInfoError::UnknownFormat)
    ));
    assert_eq!(reader.position(), 5);
    assert!(matches!(
        image_info(b"BM0123456789"),
        Err(ImageInfoError::UnsupportedFormat)
    ));
    #[cfg(not(feature = "png"))]
    assert!(matches!(
        image_info(b"\x89PNG\r\n\x1a\nmore"),
        Err(ImageInfoError::UnsupportedFormat)
    ));
    #[cfg(not(feature = "psd"))]
    assert!(matches!(
        image_info(b"8BPS\0\x01\0\0"),
        Err(ImageInfoError::UnsupportedFormat)
    ));
    let err = image_info_with_limits(
        b"BM123456",
        DecodeLimits {
            input_bytes: 1,
            ..DecodeLimits::unlimited()
        },
    )
    .unwrap_err();
    assert!(matches!(err, ImageInfoError::LimitExceeded("input bytes")));
}

#[cfg(feature = "png")]
fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend(tag);
    out.extend(data);
    let mut crc = tag.to_vec();
    crc.extend(data);
    out.extend(wml2::png::utils::CRC32::new().crc32(&crc).to_be_bytes());
}
#[cfg(feature = "png")]
fn png(width: u32, height: u32, color: u8, depth: u8, extra: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = width.to_be_bytes().to_vec();
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([depth, color, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    for (tag, data) in extra {
        chunk(&mut out, tag, data);
    }
    chunk(&mut out, b"IDAT", b"not zlib pixels");
    chunk(&mut out, b"IEND", &[]);
    out
}
#[cfg(feature = "png")]
#[test]
fn png_reports_encoded_depth_model_alpha_and_declared_animation_without_inflating_any_payload() {
    for (color, depth, model, channels, alpha) in [
        (0, 1, EncodedColor::Gray, 1, AlphaInfo::Absent),
        (0, 16, EncodedColor::Gray, 1, AlphaInfo::Absent),
        (2, 8, EncodedColor::Rgb, 3, AlphaInfo::Absent),
        (4, 16, EncodedColor::Gray, 2, AlphaInfo::Straight),
        (6, 16, EncodedColor::Rgb, 4, AlphaInfo::Straight),
    ] {
        let bytes = png(
            100_000,
            200_000,
            color,
            depth,
            &[(b"iCCP", vec![42; 100_000])],
        );
        let idat = bytes.windows(4).position(|v| v == b"IDAT").unwrap() as u64;
        let info = no_expansion(bytes.clone(), vec![41..100_041, idat + 4..idat + 4 + 15]);
        assert_eq!(
            info.dimensions.coded,
            ImageSize {
                width: 100_000,
                height: 200_000
            }
        );
        assert_eq!(info.dimensions.output, Some(info.dimensions.coded));
        assert_eq!(info.dimensions.intrinsic_display, None);
        assert_eq!(
            (info.color, info.channels, info.alpha),
            (model, Some(channels), alpha)
        );
        assert_eq!(
            info.bits_per_sample,
            Some(vec![depth as u16; channels as usize])
        );
        assert_eq!(info.structure, ImageStructure::Still);
        assert!(info.bytes_read < 100);
        assert!(wml2::draw::image_load(&bytes).is_err());
    }
    let mut actl = 3u32.to_be_bytes().to_vec();
    actl.extend(0u32.to_be_bytes());
    let bytes = png(
        3,
        2,
        3,
        4,
        &[
            (b"PLTE", vec![0; 6]),
            (b"tRNS", vec![0, 255]),
            (b"acTL", actl),
        ],
    );
    let info = image_info(&bytes).unwrap();
    assert_eq!(info.alpha, AlphaInfo::Transparency);
    assert_eq!(
        info.structure,
        ImageStructure::Animation {
            frames: ImageCount::Declared(3)
        }
    );
    let good = png(3, 2, 6, 8, &[]);
    for n in 0..41 {
        assert!(image_info(&good[..n]).is_err(), "PNG truncated{n}");
    }
    let mut corrupt = good.clone();
    corrupt[29] ^= 1;
    assert!(image_info(&corrupt).is_err());
    assert!(image_info(&png(0, 2, 6, 8, &[])).is_err());
    assert!(image_info(&png(3, 2, 6, 12, &[])).is_err());
    assert!(image_info(&png(3, 2, 6, 8, &[(b"tRNS", vec![0])])).is_err());
    assert!(matches!(
        image_info_with_limits(
            &good,
            DecodeLimits {
                metadata_bytes: 1,
                ..DecodeLimits::unlimited()
            }
        ),
        Err(ImageInfoError::LimitExceeded("metadata read bytes"))
    ));
    assert!(
        image_info_with_limits(
            &good,
            DecodeLimits {
                expanded_bytes: 1,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
}
#[cfg(feature = "png")]
#[test]
fn bundled_png_agrees_with_existing_canvas_without_querying_detailed_metadata() {
    let bytes = include_bytes!("../../samples/WML2Viewer.png");
    let info = image_info(bytes).unwrap();
    let decoded = wml2::draw::image_load(bytes).unwrap();
    assert_eq!(
        (
            info.dimensions.coded.width as usize,
            info.dimensions.coded.height as usize
        ),
        (decoded.width, decoded.height)
    );
}

#[cfg(feature = "jpeg")]
fn jpeg_sof(width: u16, height: u16, components: u8, progressive: bool) -> Vec<u8> {
    let mut v = vec![0xff, 0xd8, 0xff, if progressive { 0xc2 } else { 0xc0 }];
    v.extend((8u16 + 3 * u16::from(components)).to_be_bytes());
    v.push(8);
    v.extend(height.to_be_bytes());
    v.extend(width.to_be_bytes());
    v.push(components);
    for i in 0..components {
        v.extend([i + 1, 0x11, 0]);
    }
    v.extend([0xff, 0xda, 0, 2, 0xde, 0xad, 0xff, 0xd9]);
    v
}
#[cfg(feature = "jpeg")]
#[test]
fn jpeg_reads_sof_not_entropy_tables_app_metadata_or_pixels() {
    for components in [1, 3, 4] {
        for progressive in [false, true] {
            let mut bytes = vec![0xff, 0xd8, 0xff, 0xe1, 0, 102];
            bytes.extend([42; 100]);
            let tail = jpeg_sof(60_000, 60_000, components, progressive);
            let header_end = bytes.len() + tail.len() - 2 - 8;
            bytes.extend_from_slice(&tail[2..]);
            let info = no_expansion(
                bytes,
                vec![6..106, header_end as u64..header_end as u64 + 8],
            );
            assert_eq!(
                info.dimensions.coded,
                ImageSize {
                    width: 60_000,
                    height: 60_000
                }
            );
            assert_eq!(info.channels, Some(components as u16));
            assert_eq!(
                info.color,
                if components == 1 {
                    EncodedColor::Gray
                } else {
                    EncodedColor::Components
                }
            );
            assert_eq!(info.alpha, AlphaInfo::Absent);
            assert_eq!(info.orientation, None);
        }
    }
    let good = jpeg_sof(3, 2, 3, false);
    for n in 0..21 {
        assert!(image_info(&good[..n]).is_err(), "JPEG truncated{n}");
    }
    let mut bad = good;
    bad[11] = 0;
    assert!(image_info(&bad).is_err());
    let bytes = include_bytes!("fixtures/review/restart-p420-i1.jpg");
    let info = image_info(bytes).unwrap();
    let decoded = wml2::draw::image_load(bytes).unwrap();
    assert_eq!(
        (
            info.dimensions.coded.width as usize,
            info.dimensions.coded.height as usize
        ),
        (decoded.width, decoded.height)
    );
}

#[cfg(feature = "gif")]
fn gif(frames: usize) -> (Vec<u8>, Vec<std::ops::Range<u64>>) {
    let mut out = b"GIF89a\x02\0\x01\0\x80\0\0\0\0\0\xff\xff\xff".to_vec();
    let mut forbidden = Vec::new();
    for i in 0..frames {
        out.extend([0x21, 0xf9, 4, if i % 2 == 0 { 1 } else { 0 }, 0, 0, 0, 0]);
        out.extend([0x2c, 0, 0, 0, 0, 2, 0, 1, 0, 0, 2, 3]);
        let start = out.len() as u64;
        out.extend([0xff; 3]);
        forbidden.push(start..start + 3);
        out.push(0);
    }
    out.push(0x3b);
    (out, forbidden)
}
#[cfg(feature = "gif")]
#[test]
fn gif_scans_subblocks_without_lzw_decode_and_counts_actual_images() {
    let (bytes, forbidden) = gif(3);
    let info = no_expansion(bytes.clone(), forbidden);
    assert_eq!(
        info.structure,
        ImageStructure::Animation {
            frames: ImageCount::Known(3)
        }
    );
    assert_eq!(info.alpha, AlphaInfo::Transparency);
    assert_eq!(info.scan, InfoScan::ContainerScan);
    assert_eq!(info.bits_per_sample, None);
    assert!(wml2::draw::image_load(&bytes).is_err());
    for n in 0..bytes.len() {
        assert!(image_info(&bytes[..n]).is_err(), "GIF truncated{n}");
    }
    assert!(
        image_info_with_limits(
            &bytes,
            DecodeLimits {
                frames: 2,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
    let (single, _) = gif(1);
    assert_eq!(
        image_info(&single).unwrap().structure,
        ImageStructure::Still
    );
}
#[cfg(feature = "gif")]
#[test]
fn gif_plain_text_graphics_leave_presentation_count_and_alpha_unknown() {
    let (bytes, _) = gif(1);
    let mut with_text = bytes[..27].to_vec();
    with_text.extend([0x21, 1, 12, 0, 0, 0, 0, 2, 0, 1, 0, 1, 1, 0, 1, 1, b'X', 0]);
    with_text.extend_from_slice(&bytes[27..]);
    let info = image_info(&with_text).unwrap();
    assert_eq!(
        info.structure,
        ImageStructure::Animation {
            frames: ImageCount::Unknown
        }
    );
    assert_eq!(info.alpha, AlphaInfo::Unknown);
}

#[cfg(feature = "gif")]
#[test]
fn bundled_gif_canvas_matches_ordinary_decoder() {
    let bytes = include_bytes!("../../samples/WML2Viewer.gif");
    let info = image_info(bytes).unwrap();
    let decoded = wml2::draw::image_load(bytes).unwrap();
    assert_eq!(
        (
            info.dimensions.coded.width as usize,
            info.dimensions.coded.height as usize
        ),
        (decoded.width, decoded.height)
    );
}

#[cfg(feature = "psd")]
fn psd() -> Vec<u8> {
    let mut v = b"8BPS\0\x01\0\0\0\0\0\0".to_vec();
    v.extend(4u16.to_be_bytes());
    v.extend(30_000u32.to_be_bytes());
    v.extend(30_000u32.to_be_bytes());
    v.extend(16u16.to_be_bytes());
    v.extend(3u16.to_be_bytes());
    v.extend([0xff; 32]);
    v
}
#[cfg(feature = "psd")]
#[test]
fn psd_uses_existing_header_and_keeps_layers_and_alpha_unknown() {
    let bytes = psd();
    let info = no_expansion(bytes.clone(), std::iter::once(26..58).collect());
    assert_eq!(
        info.dimensions.coded,
        ImageSize {
            width: 30_000,
            height: 30_000
        }
    );
    assert_eq!(info.channels, Some(4));
    assert_eq!(info.alpha, AlphaInfo::Unknown);
    assert_eq!(
        info.structure,
        ImageStructure::Layers {
            layers: ImageCount::Unknown
        }
    );
    assert_eq!(info.bytes_read, 34);
    for n in 0..26 {
        assert!(image_info(&bytes[..n]).is_err());
    }
    let mut bad = bytes.clone();
    bad[6] = 1;
    assert!(image_info(&bad).is_err());
    bad = bytes;
    bad[23] = 32;
    assert!(image_info(&bad).is_err());
}

#[cfg(feature = "tiff")]
fn tiff(
    big: bool,
    be: bool,
    next: bool,
    inkset: Option<u16>,
) -> (Vec<u8>, Vec<std::ops::Range<u64>>) {
    fn number(v: u64, size: usize, be: bool) -> Vec<u8> {
        if be {
            v.to_be_bytes()[8 - size..].to_vec()
        } else {
            v.to_le_bytes()[..size].to_vec()
        }
    }
    let channels = 4;
    let photo = if inkset.is_some() { 5 } else { 2 };
    let mut entries = vec![
        (256, 4, 1, number(100_000, 4, be)),
        (257, 4, 1, number(200_000, 4, be)),
        (
            258,
            3,
            channels,
            vec![number(16, 2, be); channels as usize].concat(),
        ),
        (262, 3, 1, number(photo, 2, be)),
        (274, 3, 1, number(6, 2, be)),
        (277, 3, 1, number(channels, 2, be)),
        (
            339,
            3,
            channels,
            vec![number(1, 2, be); channels as usize].concat(),
        ),
        (34675, 7, 100_000, vec![42; 100_000]),
    ];
    if let Some(v) = inkset {
        entries.push((332, 3, 1, number(u64::from(v), 2, be)));
    } else {
        entries.push((338, 3, 1, number(2, 2, be)));
    }
    entries.sort_by_key(|e| e.0);
    let (base, prefix, record, slot) = if big { (16, 8, 20, 8) } else { (8, 2, 12, 4) };
    let payload_start = base + prefix + entries.len() * record + slot;
    let mut out = if be { b"MM".to_vec() } else { b"II".to_vec() };
    out.extend(number(if big { 43 } else { 42 }, 2, be));
    if big {
        out.extend(number(8, 2, be));
        out.extend([0, 0]);
    }
    out.extend(number(base as u64, slot, be));
    out.extend(number(entries.len() as u64, prefix, be));
    let mut append = Vec::new();
    let mut forbidden = Vec::new();
    for (tag, ty, count, payload) in entries {
        out.extend(number(tag, 2, be));
        out.extend(number(ty, 2, be));
        out.extend(number(count, if big { 8 } else { 4 }, be));
        if payload.len() <= slot {
            out.extend(&payload);
            out.extend(vec![0; slot - payload.len()]);
        } else {
            let offset = (payload_start + append.len()) as u64;
            out.extend(number(offset, slot, be));
            if tag == 34675 {
                forbidden.push(offset..offset + payload.len() as u64);
            }
            append.extend(payload);
        }
    }
    let second = (payload_start + append.len()) as u64;
    out.extend(number(if next { second } else { 0 }, slot, be));
    out.extend(append);
    // This could be an invalid next IFD or compressed raster. Neither is parsed.
    forbidden.push(second..second + 32);
    out.extend([0xff; 32]);
    (out, forbidden)
}
#[cfg(feature = "tiff")]
#[test]
fn tiff_reads_only_first_projected_ifd_with_bounded_allocations_and_unknown_page_output() {
    for big in [false, true] {
        for be in [false, true] {
            for next in [false, true] {
                let (bytes, forbidden) = tiff(big, be, next, None);
                let info = no_expansion(bytes, forbidden);
                assert_eq!(
                    info.dimensions.coded,
                    ImageSize {
                        width: 100_000,
                        height: 200_000
                    }
                );
                assert_eq!(info.dimensions.output, None);
                assert_eq!(info.dimensions.intrinsic_display, None);
                assert_eq!(info.bits_per_sample, Some(vec![16; 4]));
                assert_eq!(info.color, EncodedColor::Rgb);
                assert_eq!(info.alpha, AlphaInfo::Straight);
                assert_eq!(info.orientation, Some(6));
                assert_eq!(info.sample_kind, SampleKind::Unsigned);
                assert_eq!(
                    info.structure,
                    ImageStructure::Pages {
                        pages: if next {
                            ImageCount::Unknown
                        } else {
                            ImageCount::Known(1)
                        }
                    }
                );
                assert!(info.bytes_read < 400);
            }
        }
    }
    for (inkset, model) in [(1, EncodedColor::Cmyk), (2, EncodedColor::Components)] {
        let (bytes, _) = tiff(false, false, false, Some(inkset));
        assert_eq!(image_info(&bytes).unwrap().color, model);
    }
    let (good, _) = tiff(false, false, false, None);
    for n in 0..126 {
        assert!(image_info(&good[..n]).is_err(), "TIFF truncated{n}");
    }
    let mut bad = good.clone();
    bad[10..12].copy_from_slice(&257u16.to_le_bytes());
    assert!(image_info(&bad).is_err());
    bad = good.clone();
    bad[12..14].copy_from_slice(&1u16.to_le_bytes());
    assert!(image_info(&bad).is_err());
    let mut oversized = good.clone();
    for record in oversized[10..118].chunks_exact_mut(12) {
        if u16::from_le_bytes(record[..2].try_into().unwrap()) == 258 {
            record[4..8].copy_from_slice(&65u32.to_le_bytes());
        }
    }
    let error = image_info_with_limits(
        &oversized,
        DecodeLimits {
            metadata_bytes: usize::MAX,
            ..tiny_limits()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("information scope"));
    let mut compact = good.clone();
    for record in compact[10..118].chunks_exact_mut(12) {
        if matches!(
            u16::from_le_bytes(record[..2].try_into().unwrap()),
            258 | 339
        ) {
            let tag = u16::from_le_bytes(record[..2].try_into().unwrap());
            record[4..8].copy_from_slice(&1u32.to_le_bytes());
            record[8..12].copy_from_slice(&(if tag == 258 { 16u32 } else { 1u32 }).to_le_bytes());
        }
    }
    let compact_info = image_info(&compact).unwrap();
    assert_eq!(compact_info.bits_per_sample, Some(vec![16; 4]));
    assert_eq!(compact_info.sample_kind, SampleKind::Unsigned);
    assert!(
        image_info_with_limits(
            &good,
            DecodeLimits {
                metadata_bytes: 1,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
    assert!(
        image_info_with_limits(
            &good,
            DecodeLimits {
                expanded_bytes: 4096,
                ..DecodeLimits::unlimited()
            }
        )
        .is_err()
    );
}
#[cfg(feature = "tiff")]
#[test]
fn existing_tiff_fixture_reports_header_precision_independently_of_decode_profile() {
    let bytes = include_bytes!("fixtures/ex-gray16-lzw.tif");
    let info = image_info(bytes).unwrap();
    assert_eq!(info.bits_per_sample, Some(vec![16]));
    assert_eq!(info.color, EncodedColor::Gray);
    let decoded = wml2::draw::image_load(bytes).unwrap();
    assert_eq!(
        (
            info.dimensions.coded.width as usize,
            info.dimensions.coded.height as usize
        ),
        (decoded.width, decoded.height)
    );
    let signed = include_bytes!("fixtures/ex-review-sampleformat-signed16.tif");
    assert_eq!(image_info(signed).unwrap().sample_kind, SampleKind::Signed);
}

#[cfg(all(feature = "png", feature = "jpeg", feature = "tiff"))]
#[test]
#[ignore = "manual metadata query benchmark"]
fn benchmark_header_queries() {
    let cases: [(&str, &[u8]); 3] = [
        ("PNG", include_bytes!("../../samples/WML2Viewer.png")),
        (
            "JPEG",
            include_bytes!("fixtures/review/restart-p420-i1.jpg"),
        ),
        ("TIFF", include_bytes!("fixtures/ex-gray16-lzw.tif")),
    ];
    for (name, bytes) in cases {
        let mut times = Vec::new();
        let mut read = 0;
        for i in 0..1003 {
            let start = std::time::Instant::now();
            let info = image_info(bytes).unwrap();
            if i >= 3 {
                times.push(start.elapsed().as_secs_f64() * 1e6);
            }
            read = info.bytes_read;
            std::hint::black_box(info);
        }
        times.sort_by(f64::total_cmp);
        println!(
            "{name}: {:.3} us, {read} metadata bytes read of {} input bytes (1000 timed +3 warmup)",
            times[times.len() / 2],
            bytes.len()
        );
    }
}

#[cfg(feature = "psd")]
#[test]
fn valid_psd_composite_matches_coded_canvas_while_layer_count_remains_unqueried() {
    let mut bytes = b"8BPS\0\x01\0\0\0\0\0\0".to_vec();
    bytes.extend(3u16.to_be_bytes());
    bytes.extend(1u32.to_be_bytes());
    bytes.extend(2u32.to_be_bytes());
    bytes.extend(8u16.to_be_bytes());
    bytes.extend(3u16.to_be_bytes());
    bytes.extend([0; 12]);
    bytes.extend([0, 0, 1, 2, 3, 4, 5, 6]);
    let info = image_info(&bytes).unwrap();
    let decoded = wml2::draw::image_load(&bytes).unwrap();
    assert_eq!((decoded.width, decoded.height), (2, 1));
    assert_eq!(
        info.dimensions.coded,
        ImageSize {
            width: 2,
            height: 1
        }
    );
    assert_eq!(
        info.structure,
        ImageStructure::Layers {
            layers: ImageCount::Unknown
        }
    );
    assert_eq!(decoded.buffer.unwrap(), vec![1, 3, 5, 255, 2, 4, 6, 255]);
}

#[cfg(any(feature = "png", feature = "gif", feature = "tiff"))]
#[test]
fn valid_animated_and_multipage_streams_distinguish_declared_known_and_unknown_counts() {
    use wml2::draw::*;
    use wml2::util::ImageFormat;
    let mut image = ImageBuffer::from_buffer(2, 1, vec![0; 8]);
    image.animation = Some(
        vec![[255, 0, 0, 255], [0, 0, 255, 128]]
            .into_iter()
            .map(|color| AnimationLayer {
                width: 2,
                height: 1,
                start_x: 0,
                start_y: 0,
                buffer: color.repeat(2),
                control: NextOptions {
                    flag: NextOption::Continue,
                    await_time: 80,
                    image_rect: None,
                    dispose_option: None,
                    blend: Some(NextBlend::Override),
                },
            })
            .collect(),
    );
    for (format, expected) in [
        #[cfg(feature = "png")]
        (
            ImageFormat::Png,
            ImageStructure::Animation {
                frames: ImageCount::Declared(2),
            },
        ),
        #[cfg(feature = "gif")]
        (
            ImageFormat::Gif,
            ImageStructure::Animation {
                frames: ImageCount::Known(2),
            },
        ),
        #[cfg(feature = "tiff")]
        (
            ImageFormat::Tiff,
            ImageStructure::Pages {
                pages: ImageCount::Unknown,
            },
        ),
    ] {
        let bytes = image_encoder(
            &mut EncodeOptions {
                debug_flag: 0,
                drawer: &mut image,
                options: None,
            },
            format,
        )
        .unwrap();
        let info = image_info(&bytes).unwrap();
        assert_eq!(info.structure, expected);
        let decoded = image_load(&bytes).unwrap();
        assert!(decoded.animation.is_some());
        assert_eq!((decoded.width, decoded.height), (2, 1));
    }
}
