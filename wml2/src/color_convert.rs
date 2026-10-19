//! Checked, separate source/destination channel layouts for codec RGB output.
use icc_profile::{ParseLimits, Transform, TransformLimits, TransformOptions};
type Error = Box<dyn std::error::Error>;

// Frozen LittleCMS-generated standard sRGB profile. Its copyright tag reads
// "No copyright, use freely". This is profile data, not another runtime CMS.
pub(crate) const SRGB_ICC: &[u8] = include_bytes!("srgb.icc");

pub(crate) fn rgba16_to_rgb_profile(
    rgba: &mut [u16],
    source: &[u8],
    destination: &[u8],
    source_is_gray: bool,
    compiled_budget: usize,
) -> Result<(), Error> {
    if !rgba.len().is_multiple_of(4) {
        return Err(std::io::Error::other("ICC RGBA layout mismatch").into());
    }
    crate::limits::check(1, compiled_budget, "ICC compiled workspace")?;
    let limit = crate::limits::current().metadata_bytes.min(64 << 20);
    let budget = compiled_budget.min(64 << 20);
    let transform = Transform::from_bytes_with_limits(
        source,
        destination,
        TransformOptions::default(),
        ParseLimits {
            max_profile_size: limit,
            max_tag_size: limit,
            ..ParseLimits::default()
        },
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
    for chunk in rgba.chunks_mut(256 * 4) {
        let count = chunk.len() / 4;
        for (i, pixel) in chunk.chunks_exact(4).enumerate() {
            if channels == 1 {
                input[i] = f32::from(pixel[0]) / 65535.0;
            } else {
                for c in 0..3 {
                    input[i * 3 + c] = f32::from(pixel[c]) / 65535.0;
                }
            }
        }
        // This is private, unpublished scratch output: on error the entire
        // image is dropped before any callback. The f32 API avoids the integer
        // API's extra validation transform while using the same own-library
        // colour operation and final u16 rounding.
        transform.transform_f32(&input[..count * channels], &mut output[..count * 3])?;
        for (i, pixel) in chunk.chunks_exact_mut(4).enumerate() {
            for c in 0..3 {
                pixel[c] = (output[i * 3 + c].clamp(0.0, 1.0) * 65535.0).round() as u16;
            }
        }
    }
    Ok(())
}
