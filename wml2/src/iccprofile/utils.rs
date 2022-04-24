use std::fs::File;
use std::io::prelude::*;

pub fn dump (filename:String,buf:&mut [u8]) -> std::io::Result<()>  {
    let mut file = File::create(filename)?;
    file.write_all(buf)?;
    file.flush()?;
    Ok(())
}