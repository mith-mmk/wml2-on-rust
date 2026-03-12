//! Format detection helpers and shared image format identifiers.

use bin_rs::io::read_string;

/// Image formats recognized by [`format_check`].
pub enum ImageFormat {
    Gif,  // GIF87a , GIF89a
    Jpeg, // 0xfffe
    Bmp,  // BM
    Tiff, // II/MM
    Png,  // [0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]
    Webp, // RIFF . . . . WEBP
    //
    // Japanse old format
    Mag,
    #[cfg(not(feature = "noretoro"))]
    Maki,
    #[cfg(not(feature = "noretoro"))]
    Pi,
    #[cfg(not(feature = "noretoro"))]
    Pic,
    Pic2,
    #[cfg(not(feature = "noretoro"))]
    Vsp,
    #[cfg(not(feature = "noretoro"))]
    Pcd,
    RiffFormat(String),
    Unknown,
}

/// Detects an image format from the leading bytes of `buffer`.
pub fn format_check(buffer: &[u8]) -> ImageFormat {
    if buffer.len() < 8 {
        return ImageFormat::Unknown;
    }
    match buffer[0] {
        b'G' => {
            if buffer[1] == b'I'
                && buffer[2] == b'F'
                && buffer[3] == b'8'
                && (buffer[4] == b'7' || buffer[4] == b'9')
                && buffer[5] == b'a'
            {
                return ImageFormat::Gif;
            }
        }
        b'B' => {
            if buffer[1] == b'M' {
                return ImageFormat::Bmp;
            }
        }
        b'I' => {
            if buffer[1] == b'I' {
                let ver = bin_rs::io::read_u16_le(buffer, 2);
                if ver == 42 {
                    return ImageFormat::Tiff;
                }
            }
        }
        b'M' => {
            if buffer[1] == b'M' {
                let ver = bin_rs::io::read_u16_be(buffer, 2);
                if ver == 42 {
                    return ImageFormat::Tiff;
                }
                return ImageFormat::Tiff;
            }
            if buffer[1] == b'A' && buffer[2] == b'K' && buffer[3] == b'I' && buffer[4] == b'0' && buffer[5] == b'2' {
                return ImageFormat::Mag;
            }
            #[cfg(not(feature = "noretoro"))]
            if buffer.len() >= 7
                && buffer[1] == b'A'
                && buffer[2] == b'K'
                && buffer[3] == b'I'
                && buffer[4] == b'0'
                && buffer[5] == b'1'
            {
                return ImageFormat::Maki;
            }
        }
        b'P' => {
            #[cfg(not(feature = "noretoro"))]
            if buffer[1] == b'i' {
                return ImageFormat::Pi;
            }
            #[cfg(not(feature = "noretoro"))]
            if buffer[1] == b'I' && buffer[2] == b'C' {
                return ImageFormat::Pic;
            }
        }
        b'R' => {
            if buffer[1] == b'I' && buffer[2] == b'F' && buffer[3] == b'F' {
                // RIFF
                if buffer[8] == b'W'
                    && buffer[9] == b'E'
                    && buffer[10] == b'B'
                    && buffer[11] == b'P'
                {
                    return ImageFormat::Webp;
                }
                let s = read_string(buffer, 8, 4);
                return ImageFormat::RiffFormat(s);
            }
        }
        //[0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]
        0x89 => {
            if buffer[1] == 0x50
                && buffer[2] == 0x4e
                && buffer[3] == 0x47
                && buffer[4] == 0x0d
                && buffer[5] == 0x0a
                && buffer[6] == 0x1a
                && buffer[7] == 0x0a
            {
                return ImageFormat::Png;
            }
        }
        0xff => {
            if buffer[1] == 0xd8 {
                return ImageFormat::Jpeg;
            }
        }

        _ => {}
    }

    #[cfg(not(feature = "noretoro"))]
    if buffer.len() >= 10 {
        let pixel = buffer[8];
        let start_x = bin_rs::io::read_u16_le(buffer, 0);
        let end_x = bin_rs::io::read_u16_le(buffer, 4);
        if (pixel == 0 || pixel == 1 || pixel == 8) && start_x <= 80 && (end_x <= 80 || pixel == 1)
        {
            return ImageFormat::Vsp;
        }
        let page_count = bin_rs::io::read_u16_le(buffer, 0);
        if page_count > 0 && page_count <= 0x10 {
            return ImageFormat::Vsp;
        }
    }
    ImageFormat::Unknown
}

