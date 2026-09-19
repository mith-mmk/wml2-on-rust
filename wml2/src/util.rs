//! Format detection helpers and shared image format identifiers.

use bin_rs::io::read_string;

/// Image formats recognized by [`format_check`].
pub enum ImageFormat {
    Gif,  // GIF87a , GIF89a
    Jpeg, // 0xfffe
    Bmp,  // BM
    Ico,  // 00 00 01 00
    Tiff, // II/MM
    Png,  // [0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]
    #[cfg(feature = "psd")]
    Psd, // 8BPS, version checked by the decoder
    Webp, // RIFF . . . . WEBP
    #[cfg(feature = "avif")]
    Avif, // ISO BMFF ftyp avif/avis
    #[cfg(all(feature = "tga", not(feature = "noretoro")))]
    Tga,
    #[cfg(all(feature = "pcx", not(feature = "noretoro")))]
    Pcx,
    #[cfg(all(feature = "dds", not(feature = "noretoro")))]
    Dds,
    //
    // Japanse old format
    #[cfg(not(feature = "noretoro"))]
    Mag,
    #[cfg(not(feature = "noretoro"))]
    Maki,
    #[cfg(not(feature = "noretoro"))]
    Pi,
    #[cfg(not(feature = "noretoro"))]
    Pic,
    #[cfg(all(feature = "pic2", not(feature = "noretoro")))]
    Pic2,
    #[cfg(all(feature = "q4", not(feature = "noretoro")))]
    Q4,
    #[cfg(not(feature = "noretoro"))]
    Vsp,
    #[cfg(not(feature = "noretoro"))]
    Pcd,
    RiffFormat(String),
    Unknown,
}

/// Detects an image format from the leading bytes of `buffer`.
pub fn format_check(buffer: &[u8]) -> ImageFormat {
    #[cfg(feature = "psd")]
    if buffer.len() >= 4 && buffer.starts_with(b"8BPS") {
        return ImageFormat::Psd;
    }
    if buffer.len() >= 4 && buffer.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
        return ImageFormat::Ico;
    }
    if buffer.len() >= 6
        && buffer[0] == b'G'
        && buffer[1] == b'I'
        && buffer[2] == b'F'
        && buffer[3] == b'8'
        && (buffer[4] == b'7' || buffer[4] == b'9')
        && buffer[5] == b'a'
    {
        return ImageFormat::Gif;
    }
    if buffer.len() >= 2 && buffer[0] == b'B' && buffer[1] == b'M' {
        return ImageFormat::Bmp;
    }
    if buffer.len() >= 4 && buffer[0] == b'I' && buffer[1] == b'I' {
        let ver = bin_rs::io::read_u16_le(buffer, 2);
        if matches!(ver, 42 | 43) {
            return ImageFormat::Tiff;
        }
    }
    if buffer.len() >= 4 && buffer[0] == b'M' && buffer[1] == b'M' {
        let ver = bin_rs::io::read_u16_be(buffer, 2);
        if matches!(ver, 42 | 43) {
            return ImageFormat::Tiff;
        }
        return ImageFormat::Tiff;
    }
    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 6 && buffer.starts_with(b"MAKI02") {
        return ImageFormat::Mag;
    }
    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 6 && buffer.starts_with(b"MAKI01") {
        return ImageFormat::Maki;
    }
    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 2 && buffer[0] == b'P' && buffer[1] == b'i' {
        return ImageFormat::Pi;
    }
    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 3 && buffer[0] == b'P' && buffer[1] == b'I' && buffer[2] == b'C' {
        return ImageFormat::Pic;
    }
    if buffer.len() >= 12 && buffer.starts_with(b"RIFF") {
        if &buffer[8..12] == b"WEBP" {
            return ImageFormat::Webp;
        }
        let s = read_string(buffer, 8, 4);
        return ImageFormat::RiffFormat(s);
    }
    #[cfg(feature = "avif")]
    if buffer.len() >= 16 && &buffer[4..8] == b"ftyp" {
        let major_brand = &buffer[8..12];
        if major_brand == b"avif" || major_brand == b"avis" {
            return ImageFormat::Avif;
        }
        let mut offset = 16;
        while offset + 4 <= buffer.len() {
            let brand = &buffer[offset..offset + 4];
            if brand == b"avif" || brand == b"avis" {
                return ImageFormat::Avif;
            }
            offset += 4;
        }
    }
    if buffer.len() >= 8
        && buffer[0] == 0x89
        && buffer[1] == 0x50
        && buffer[2] == 0x4e
        && buffer[3] == 0x47
        && buffer[4] == 0x0d
        && buffer[5] == 0x0a
        && buffer[6] == 0x1a
        && buffer[7] == 0x0a
    {
        return ImageFormat::Png;
    }
    if buffer.len() >= 2 && buffer[0] == 0xff && buffer[1] == 0xd8 {
        return ImageFormat::Jpeg;
    }

    #[cfg(all(feature = "tga", not(feature = "noretoro")))]
    if buffer.len() >= 18
        && buffer[1] <= 1
        && matches!(buffer[2], 1 | 2 | 3 | 9 | 10 | 11)
        && bin_rs::io::read_u16_le(buffer, 12) > 0
        && bin_rs::io::read_u16_le(buffer, 14) > 0
        && matches!(buffer[16], 8 | 15 | 16 | 24 | 32)
    {
        return ImageFormat::Tga;
    }

    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 58 {
        let pixel = buffer[8];
        let start_x = bin_rs::io::read_u16_le(buffer, 0);
        let start_y = bin_rs::io::read_u16_le(buffer, 2);
        let end_x = bin_rs::io::read_u16_le(buffer, 4);
        let end_y = bin_rs::io::read_u16_le(buffer, 6);
        let valid_dimensions = if pixel == 1 {
            start_x <= end_x && start_y <= end_y
        } else {
            start_x < end_x && start_y < end_y
        };
        if (pixel == 0 || pixel == 1 || pixel == 8)
            && valid_dimensions
            && start_x <= 80
            && (end_x <= 80 || pixel == 1)
        {
            return ImageFormat::Vsp;
        }
        let page_count = bin_rs::io::read_u16_le(buffer, 0);
        if page_count > 0 && page_count <= 0x10 {
            return ImageFormat::Vsp;
        }
    }

    #[cfg(all(feature = "dds", not(feature = "noretoro")))]
    if buffer.len() >= 4 && buffer.starts_with(b"DDS ") {
        return ImageFormat::Dds;
    }
    #[cfg(all(feature = "pcx", not(feature = "noretoro")))]
    if buffer.len() >= 128
        && buffer[0] == 0x0a
        && buffer[2] == 1
        && matches!(buffer[3], 1 | 2 | 4 | 8)
        && buffer[65] > 0
        && buffer[65] <= 4
        && bin_rs::io::read_u16_le(buffer, 8) >= bin_rs::io::read_u16_le(buffer, 4)
        && bin_rs::io::read_u16_le(buffer, 10) >= bin_rs::io::read_u16_le(buffer, 6)
    {
        return ImageFormat::Pcx;
    }
    #[cfg(all(feature = "pic2", not(feature = "noretoro")))]
    if buffer.starts_with(b"P2DT") || (buffer.len() >= 132 && buffer[128..].starts_with(b"P2DT")) {
        return ImageFormat::Pic2;
    }
    #[cfg(all(feature = "q4", not(feature = "noretoro")))]
    if buffer.len() >= 16
        && buffer[11..16] == *b"MAJYO"
        && (buffer[2] == 2 || (buffer[1] <= 1 && buffer[3] <= 1))
    {
        return ImageFormat::Q4;
    }
    ImageFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::{ImageFormat, format_check};

    #[test]
    fn short_buffers_return_unknown_instead_of_panicking() {
        for len in 0..12 {
            let buffer = vec![0; len];
            let _ = format_check(&buffer);
        }
    }

    #[test]
    fn short_riff_header_does_not_panic() {
        let buffer = b"RIFF\x00\x00\x00\x00".to_vec();
        assert!(matches!(format_check(&buffer), ImageFormat::Unknown));
    }
}

/// Returns whether a decoder for `format` is enabled in the current build.
pub fn decoder_supports_format(format: &ImageFormat) -> bool {
    match format {
        #[cfg(feature = "gif")]
        ImageFormat::Gif => true,
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg => true,
        #[cfg(feature = "bmp")]
        ImageFormat::Bmp => true,
        #[cfg(feature = "ico")]
        ImageFormat::Ico => true,
        #[cfg(feature = "tiff")]
        ImageFormat::Tiff => true,
        #[cfg(feature = "png")]
        ImageFormat::Png => true,
        #[cfg(feature = "psd")]
        ImageFormat::Psd => true,
        #[cfg(feature = "webp")]
        ImageFormat::Webp => true,
        #[cfg(feature = "avif")]
        ImageFormat::Avif => true,
        #[cfg(all(feature = "tga", not(feature = "noretoro")))]
        ImageFormat::Tga => true,
        #[cfg(all(feature = "pcx", not(feature = "noretoro")))]
        ImageFormat::Pcx => true,
        #[cfg(all(feature = "dds", not(feature = "noretoro")))]
        ImageFormat::Dds => true,
        #[cfg(all(feature = "pic2", not(feature = "noretoro")))]
        ImageFormat::Pic2 => true,
        #[cfg(all(feature = "q4", not(feature = "noretoro")))]
        ImageFormat::Q4 => true,
        #[cfg(all(feature = "mag", not(feature = "noretoro")))]
        ImageFormat::Mag => true,
        #[cfg(all(feature = "maki", not(feature = "noretoro")))]
        ImageFormat::Maki => true,
        #[cfg(all(feature = "pi", not(feature = "noretoro")))]
        ImageFormat::Pi => true,
        #[cfg(all(feature = "pic", not(feature = "noretoro")))]
        ImageFormat::Pic => true,
        #[cfg(all(feature = "vsp", not(feature = "noretoro")))]
        ImageFormat::Vsp => true,
        #[cfg(all(feature = "pcd", not(feature = "noretoro")))]
        ImageFormat::Pcd => true,
        _ => false,
    }
}

/// Returns whether an encoder for `format` is enabled in the current build.
pub fn encoder_supports_format(format: &ImageFormat) -> bool {
    match format {
        #[cfg(feature = "gif")]
        ImageFormat::Gif => true,
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg => true,
        #[cfg(feature = "bmp")]
        ImageFormat::Bmp => true,
        #[cfg(feature = "png")]
        ImageFormat::Png => true,
        #[cfg(feature = "tiff")]
        ImageFormat::Tiff => true,
        #[cfg(feature = "webp")]
        ImageFormat::Webp => true,
        _ => false,
    }
}
