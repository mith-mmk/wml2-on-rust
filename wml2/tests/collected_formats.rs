use std::env;
use std::fs;
use std::path::Path;

use wml2::draw::image_load;

fn decode_tree(root: &Path, extension: &str) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(root).expect("sample directory should be readable") {
        let entry = entry.expect("sample directory entry should be readable");
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            let bytes = fs::read(&path).expect("sample should be readable");
            let image = match image_load(&bytes) {
                Ok(image) => image,
                Err(error) if path.file_name().is_some_and(|name| name == "GMARBLES.PCX") => {
                    // The archived sample declares 1001 rows but ends after
                    // 997 rows; keep it as a negative safety case.
                    assert!(error.to_string().contains("truncated"));
                    continue;
                }
                Err(error) => panic!("{} should decode: {error}", path.display()),
            };
            assert!(image.width > 0, "{} has no width", path.display());
            assert!(image.height > 0, "{} has no height", path.display());
            assert!(
                image
                    .buffer
                    .as_ref()
                    .is_some_and(|buffer| !buffer.is_empty())
            );
            count += 1;
        }
    }
    count
}

#[test]
fn decode_collected_samples() {
    let root = match env::var_os("WML2_SAMPLE_ROOT") {
        Some(root) => root,
        None => return,
    };
    let root = Path::new(&root);
    assert!(decode_tree(&root.join("tga"), "tga") >= 1);
    assert!(decode_tree(&root.join("pcx"), "pcx") >= 1);
    assert!(decode_tree(&root.join("dds"), "dds") >= 1);
    assert!(decode_tree(&root.join("pic2").join("samples"), "p2") >= 1);
}

#[test]
fn pic2_reference_pixels_are_preserved() {
    let Some(root) = env::var_os("WML2_SAMPLE_ROOT") else {
        return;
    };
    let path = Path::new(&root)
        .join("pic2")
        .join("samples")
        .join("isis.p2");
    let data = fs::read(&path).expect("PIC2 reference sample should be readable");
    let image = image_load(&data).expect("PIC2 reference sample should decode");
    let buffer = image.buffer.expect("PIC2 image should have pixels");
    let expected = [
        [186, 192, 234, 255],
        [189, 193, 239, 255],
        [187, 195, 240, 255],
        [189, 193, 239, 255],
    ];
    for (index, pixel) in expected.iter().enumerate() {
        assert_eq!(&buffer[index * 4..index * 4 + 4], pixel, "pixel {index}");
    }
}
