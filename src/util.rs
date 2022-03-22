
use crate::io::read_string;

pub enum ImageFormat{
    Gif,    // GIF87a , GIF89a
    Jpeg,   // 0xfffe
    Bmp,    // BM
    Tiff,   // II/MM
    Png,    // [0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]
    Webp,   // RIFF . . . . WEBP
//
// Japanse old format
    Mag,
    Maki,
    Pi,
    Pic,
    Pic2,
    RiffFormat(String),    
    Unknown,
}

pub fn format_check(buffer: &[u8]) -> ImageFormat {
    match buffer[0] {
        b'G' => {
            if buffer[1] == b'I' && buffer[2] == b'F' && buffer[3] == b'8'
             && (buffer[4] == b'7' || buffer[4] == b'9') && buffer[5] == b'a' {
                 return ImageFormat::Gif
             }
        },
        b'B' => {
            if buffer[1] == b'M' {
                return ImageFormat::Bmp
            }
        },
        b'I' => {
            if buffer[1] == b'I' {
                return ImageFormat::Tiff
            }
        },
        b'M' => {
            if buffer[1] == b'M' {
                return ImageFormat::Tiff
            }
        },
        b'R' => {
            if buffer[1] == b'I' && buffer[2] == b'F' && buffer[3] == b'F' {
                // RIFF
                if buffer[8] == b'W' && buffer[9] == b'E' && buffer[10] == b'B' && buffer[11] == b'P' {
                    return ImageFormat::Webp
                }
                let s = read_string(buffer,8,4);
                return ImageFormat::RiffFormat(s)
            }
        },
        //[0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A]
        0x89 => {
            if buffer[1] == 0x50 && buffer[2] == 0x4e && buffer[3] == 0x47
                && buffer[4] == 0x0d && buffer[5] == 0x0a && buffer[6] == 0x1a
                && buffer[7] == 0x0a { 
                    return ImageFormat::Png
                }
        },
        0xff => {
            if buffer[1] == 0xd8 {
                return ImageFormat::Jpeg
            }

        }

        _ => {
            return ImageFormat::Unknown
        }
    }    ImageFormat::Unknown

}
