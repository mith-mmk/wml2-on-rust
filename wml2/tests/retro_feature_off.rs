#[cfg(all(
    feature = "tga",
    feature = "pcx",
    feature = "dds",
    feature = "pic2",
    feature = "noretoro"
))]
#[test]
fn noretoro_excludes_new_retro_formats() {
    use wml2::util::{ImageFormat, format_check};

    let mut tga = [0u8; 18];
    tga[2] = 2;
    tga[4] = 0xff;
    tga[12] = 1;
    tga[14] = 1;
    tga[16] = 24;

    let mut pcx = [0u8; 128];
    pcx[0] = 0x0a;
    pcx[2] = 1;
    pcx[3] = 8;
    pcx[65] = 1;

    let samples = [
        b"DDS ".as_slice(),
        tga.as_slice(),
        pcx.as_slice(),
        b"P2DT".as_slice(),
    ];

    for sample in samples {
        assert!(matches!(format_check(sample), ImageFormat::Unknown));
        assert!(!wml2::get_can_decode(sample).unwrap());
    }

    for extension in ["tga", "pcx", "dds", "p2"] {
        assert!(
            !wml2::get_decoder_extentions()
                .iter()
                .any(|advertised| advertised == extension)
        );
    }
}

#[cfg(all(feature = "q4", feature = "noretoro"))]
#[test]
fn noretoro_excludes_q4() {
    use wml2::util::{ImageFormat, format_check};

    let mut header = [0u8; 22];
    header[0..2].copy_from_slice(&0x001au16.to_le_bytes());
    header[2] = 0x12;
    header[3] = 1;
    header[4] = 1;
    header[11..16].copy_from_slice(b"MAJYO");

    assert!(matches!(format_check(&header), ImageFormat::Unknown));
    assert!(!wml2::get_can_decode(&header).unwrap());
    assert!(
        !wml2::get_decoder_extentions()
            .iter()
            .any(|extension| extension == "q4")
    );
}
