#![cfg(all(feature = "jpeg", feature = "tiff", feature = "psd", feature = "png"))]
#[test]
#[ignore = "local real-image visual and timing validation; needs explicit sample and output paths"]
fn external_cmyk_visual_validation() {
    let paths =
        std::env::var("WML2_CMYK_SAMPLE_FILES").expect("sample paths separated by semicolon");
    let out =
        std::path::PathBuf::from(std::env::var("WML2_CMYK_OUTPUT_DIR").expect("output directory"));
    std::fs::create_dir_all(&out).unwrap();
    for (index, path) in paths.split(';').enumerate() {
        let bytes = std::fs::read(path).unwrap();
        let start = std::time::Instant::now();
        let (mut image, warnings) = wml2::draw::image_load_with_warnings(&bytes).unwrap();
        let mut timings = vec![start.elapsed().as_micros()];
        let repeats: usize = std::env::var("WML2_CMYK_BENCH_REPEATS")
            .unwrap_or_else(|_| "1".into())
            .parse()
            .unwrap();
        for _ in 1..repeats {
            let start = std::time::Instant::now();
            std::hint::black_box(wml2::draw::image_load_with_warnings(&bytes).unwrap());
            timings.push(start.elapsed().as_micros());
        }
        timings.sort_unstable();
        let elapsed = timings[timings.len() / 2];
        assert_eq!(
            image.buffer.as_ref().unwrap().len(),
            image.width * image.height * 4
        );
        eprintln!(
            "{}: {}x{}, {} us, warnings={:?}",
            path, image.width, image.height, elapsed, warnings
        );
        let png = wml2::draw::image_to(&mut image, wml2::util::ImageFormat::Png, None).unwrap();
        std::fs::write(out.join(format!("{index}.png")), png).unwrap();
    }
}

#[cfg(feature = "color-management")]
#[test]
#[ignore = "local ICC stage timing; needs WML2_CMYK_PROFILE_JPEG"]
fn cmyk_profile_stage_timings() {
    use icc_profile::{Profile, Transform, TransformOptions};
    let jpeg = std::fs::read(std::env::var("WML2_CMYK_PROFILE_JPEG").unwrap()).unwrap();
    let header =
        wml2::jpeg::header::JpegHaeder::new(&mut bin_rs::reader::BytesReader::new(&jpeg), 0)
            .unwrap();
    let profile = header.icc_profile.unwrap();
    let source = Profile::parse(&profile).unwrap();
    let destination = Profile::parse(include_bytes!("../src/srgb.icc")).unwrap();
    let transform = Transform::new(&source, &destination, TransformOptions::default()).unwrap();
    eprintln!(
        "profile bytes={}, pixels={}, memory={:?}, routes={:?}/{:?}",
        profile.len(),
        header.width * header.height,
        transform.memory_usage().unwrap(),
        transform.input_route_info(),
        transform.output_route_info()
    );
    fn median(mut run: impl FnMut()) -> u128 {
        let mut times = Vec::new();
        for _ in 0..7 {
            let start = std::time::Instant::now();
            run();
            times.push(start.elapsed().as_micros());
        }
        times.sort_unstable();
        times[times.len() / 2]
    }
    eprintln!(
        "parse median_us={}",
        median(|| {
            std::hint::black_box(Profile::parse(&profile).unwrap());
        })
    );
    eprintln!(
        "compile median_us={}",
        median(|| {
            std::hint::black_box(
                Transform::new(&source, &destination, TransformOptions::default()).unwrap(),
            );
        })
    );
    let pixels = header.width * header.height;
    let input: Vec<u8> = (0..pixels)
        .flat_map(|i| {
            [
                (i % 256) as u8,
                ((i / 3) % 256) as u8,
                ((i / 7) % 256) as u8,
                ((i / 11) % 256) as u8,
            ]
        })
        .collect();
    let float: Vec<f32> = input.iter().map(|v| f32::from(*v) / 255.0).collect();
    let mut output = vec![0; pixels * 3];
    let mut output_float = vec![0.0; pixels * 3];
    // Diagnostic only: retain the same sRGB matrix/white and replace its
    // three parametric TRCs with exact identity curves. This is not sRGB
    // output and must never be used as a rendering optimization.
    let mut linear_bytes = include_bytes!("../src/srgb.icc").to_vec();
    let tags = u32::from_be_bytes(linear_bytes[128..132].try_into().unwrap()) as usize;
    for index in 0..tags {
        let record = 132 + index * 12;
        if matches!(
            &linear_bytes[record..record + 4],
            b"rTRC" | b"gTRC" | b"bTRC"
        ) {
            let offset =
                u32::from_be_bytes(linear_bytes[record + 4..record + 8].try_into().unwrap())
                    as usize;
            linear_bytes[offset..offset + 12].copy_from_slice(b"curv\0\0\0\0\0\0\0\0");
            linear_bytes[record + 8..record + 12].copy_from_slice(&12u32.to_be_bytes());
        }
    }
    let linear = Profile::parse(&linear_bytes).unwrap();
    let linear_transform = Transform::new(&source, &linear, TransformOptions::default()).unwrap();
    eprintln!(
        "f32 linear-destination diagnostic median_us={}",
        median(|| {
            linear_transform
                .transform_f32(&float, &mut output_float)
                .unwrap();
        })
    );
    eprintln!(
        "u8 whole median_us={}",
        median(|| {
            transform.transform_u8(&input, &mut output).unwrap();
        })
    );
    eprintln!(
        "f32 whole median_us={}",
        median(|| {
            transform.transform_f32(&float, &mut output_float).unwrap();
        })
    );
    eprintln!(
        "u8 chunks256 median_us={}",
        median(|| {
            for (i, o) in input.chunks(256 * 4).zip(output.chunks_mut(256 * 3)) {
                transform.transform_u8(i, o).unwrap();
            }
        })
    );
    eprintln!(
        "f32 chunks256 median_us={}",
        median(|| {
            for (i, o) in float.chunks(256 * 4).zip(output_float.chunks_mut(256 * 3)) {
                transform.transform_f32(i, o).unwrap();
            }
        })
    );
}
