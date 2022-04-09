use crate::draw::CallbackResponse;
use crate::draw::image_reader;
use crate::draw::DecodeOptions;
use crate::draw::ImageBuffer;
use std::io::BufReader;
use std::fs::File;
use std::error::Error;
use std::fs;
use dotenv;

fn write_log(str: &str) -> Result<Option<CallbackResponse>,Box<dyn Error>> {
    println!("{}", str);
    Ok(None)
}

#[test]
fn wml_test() -> Result<(),Box<dyn Error>>{
    let path = dotenv::var("IMAGEPATH")?;
    println!("read");
    let dir = fs::read_dir(path)?;
    for file in dir {
        let filename = file?.path();
        println!("decode {:?}", filename);
        let f = File::open(filename)?;
        let reader = BufReader::new(f);
        let mut image = ImageBuffer::new();
        image.set_verbose(write_log);
        let mut option = DecodeOptions {
            debug_flag: 0x00,
            drawer: &mut image,
        };
        image_reader(reader, &mut option)?;

    }
    Ok(())

}