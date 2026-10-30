//! Checked, separate source/destination channel layouts for codec RGB output.
use icc_profile::{ColorSpace, ParseLimits, Profile, Transform, TransformLimits, TransformOptions};
type Error = Box<dyn std::error::Error>;

// Frozen LittleCMS-generated standard sRGB profile. Its copyright tag reads
// "No copyright, use freely". This is profile data, not another runtime CMS.
pub(crate) const SRGB_ICC: &[u8] = include_bytes!("srgb.icc");

#[cfg(any(feature = "avif", test))]
pub(crate) fn rgba16_to_rgb_profile(
    rgba: &mut [u16],
    source: &[u8],
    destination: &[u8],
    source_is_gray: bool,
    compiled_budget: usize,
) -> Result<(), Error> {
    rgba16_to_rgb_profile_with_cancel(
        rgba,
        source,
        destination,
        source_is_gray,
        compiled_budget,
        &|| Ok(()),
    )
}

pub(crate) fn rgba16_to_rgb_profile_with_cancel(
    rgba: &mut [u16],
    source: &[u8],
    destination: &[u8],
    source_is_gray: bool,
    compiled_budget: usize,
    check: &dyn Fn() -> Result<(), Error>,
) -> Result<(), Error> {
    check()?;
    if !rgba.len().is_multiple_of(4) {
        return Err(std::io::Error::other("ICC RGBA layout mismatch").into());
    }
    crate::limits::check(1, compiled_budget, "ICC compiled workspace")?;
    let limit = crate::limits::current().metadata_bytes.min(64 << 20);
    let budget = compiled_budget.min(64 << 20);
    let parse_limits = ParseLimits {
        max_profile_size: limit,
        max_tag_size: limit,
        ..Default::default()
    };
    let source = Profile::parse_with_limits(source, parse_limits)?;
    let destination = Profile::parse_with_limits(destination, parse_limits)?;
    if !matches!(source.color_space(), ColorSpace::Gray | ColorSpace::Rgb)
        || destination.color_space() != ColorSpace::Rgb
    {
        return Err(std::io::Error::other(
            "ICC display requires Gray/RGB source and RGB destination device models",
        )
        .into());
    }
    let transform = Transform::new_with_limits(
        &source,
        &destination,
        TransformOptions::default(),
        TransformLimits::builder()
            .max_compiled_bytes(budget)
            .max_curve_entries((budget / 4).clamp(2, 1 << 20))
            .max_clut_entries((budget / 4).clamp(1, 1 << 24))
            .build()?,
    )?;
    let channels = transform.input_channels();
    if transform.output_channels() != 3
        || !matches!(channels, 1 | 3)
        || (channels == 1 && !source_is_gray)
    {
        return Err(std::io::Error::other(
            "ICC source channels do not match decoded Gray/RGB samples or RGB destination",
        )
        .into());
    }
    // Alpha never enters the CMS. A Gray profile reads one neutral source
    // component and writes three destination components, with separate spans.
    let mut input = [0.0f32; 256 * 3];
    let mut output = [0.0f32; 256 * 3];
    let mut run_for_pixel = [0usize; 256];
    for chunk in rgba.chunks_mut(256 * 4) {
        check()?;
        let mut runs = 0;
        let mut previous = [0u16; 3];
        for (i, pixel) in chunk.chunks_exact(4).enumerate() {
            // The immutable transform depends only on the participating colour
            // samples. Exact adjacent repeats share one evaluation, even when
            // their alpha differs. No quantization, interpolation or profile
            // substitution is involved. Distinct colours keep their order.
            if i == 0 || pixel[..channels] != previous[..channels] {
                for c in 0..channels {
                    previous[c] = pixel[c];
                    input[runs * channels + c] = f32::from(pixel[c]) / 65535.0;
                }
                runs += 1;
            }
            run_for_pixel[i] = runs - 1;
        }
        // This is private, unpublished scratch output: on error the entire
        // image is dropped before any callback. The f32 API avoids the integer
        // API's extra validation transform while using the same own-library
        // colour operation and final u16 rounding.
        transform.transform_f32(&input[..runs * channels], &mut output[..runs * 3])?;
        for (i, pixel) in chunk.chunks_exact_mut(4).enumerate() {
            for c in 0..3 {
                pixel[c] =
                    (output[run_for_pixel[i] * 3 + c].clamp(0.0, 1.0) * 65535.0).round() as u16;
            }
        }
    }
    check()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_between_icc_batches_leaves_later_scratch_untouched() {
        let calls = std::cell::Cell::new(0);
        let original = [8192, 16384, 24576, 32768];
        let mut rgba = original.repeat(768);
        let error = rgba16_to_rgb_profile_with_cancel(
            &mut rgba,
            include_bytes!("../tests/fixtures/review/avif-icc-linear.icc"),
            SRGB_ICC,
            false,
            4 << 20,
            &|| {
                calls.set(calls.get() + 1);
                if calls.get() == 3 {
                    Err(std::io::Error::other("stop ICC").into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("stop ICC"));
        assert_ne!(&rgba[..4], &original);
        assert_eq!(&rgba[256 * 4..], original.repeat(512));
        assert!(rgba.chunks_exact(4).all(|p| p[3] == original[3]));
    }

    #[test]
    fn exact_colour_runs_match_unbatched_transform_with_distinct_alpha() {
        let rgb = include_bytes!("../tests/fixtures/review/avif-icc-linear.icc");
        let gray = include_bytes!("../tests/fixtures/review/avif-icc-gray-linear.icc");
        for source in [&rgb[..], &gray[..]] {
            let transform = Transform::from_bytes_with_limits(
                source,
                SRGB_ICC,
                TransformOptions::default(),
                ParseLimits::default(),
                TransformLimits::default(),
            )
            .unwrap();
            let channels = transform.input_channels();
            // Runs cross the scratch boundary; other spans have no repeats.
            // Alpha varies independently, including zero and full coverage.
            let original: Vec<u16> = (0..1027)
                .flat_map(|i| {
                    let colour = if i < 517 { i / 257 } else { i } as u16;
                    [
                        colour.wrapping_mul(61),
                        colour.wrapping_mul(37),
                        colour.wrapping_mul(17),
                        [0, 65535, 12345, 54321][i % 4],
                    ]
                })
                .collect();
            let input: Vec<f32> = original
                .chunks_exact(4)
                .flat_map(|p| p[..channels].iter().map(|&s| f32::from(s) / 65535.0))
                .collect();
            let mut expected = vec![0.0; 1027 * 3];
            transform.transform_f32(&input, &mut expected).unwrap();
            let mut actual = original.clone();
            rgba16_to_rgb_profile(&mut actual, source, SRGB_ICC, channels == 1, 4 << 20).unwrap();
            for ((pixel, original), expected) in actual
                .chunks_exact(4)
                .zip(original.chunks_exact(4))
                .zip(expected.chunks_exact(3))
            {
                assert_eq!(pixel[3], original[3]);
                for c in 0..3 {
                    assert_eq!(
                        pixel[c],
                        (expected[c].clamp(0.0, 1.0) * 65535.0).round() as u16
                    );
                }
            }
        }
    }

    #[test]
    fn matching_channel_counts_cannot_substitute_for_device_model_identity() {
        let mut arbitrary = include_bytes!("../tests/fixtures/review/lab-pcs-rgb.icc").to_vec();
        arbitrary[16..20].copy_from_slice(b"3CLR");
        assert_eq!(
            Profile::parse(&arbitrary).unwrap().color_space(),
            ColorSpace::NColor(3)
        );
        let rgb = include_bytes!("../tests/fixtures/review/avif-icc-linear.icc");
        for (source, dest) in [(&arbitrary[..], SRGB_ICC), (&rgb[..], &arbitrary[..])] {
            let mut pixel = [8192, 16384, 24576, 32768];
            let original = pixel;
            let error =
                rgba16_to_rgb_profile(&mut pixel, source, dest, false, 4 << 20).unwrap_err();
            assert!(error.to_string().contains("device models"));
            assert_eq!(pixel, original);
        }
    }
}
