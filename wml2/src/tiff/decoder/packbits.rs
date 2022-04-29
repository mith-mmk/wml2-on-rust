type Error = Box<dyn std::error::Error>;

pub fn decode(data:&[u8]) -> Result<Vec<u8>,Error> {
    let mut buf = vec![];
    let mut i = 0;
    while i < data.len() {
        let run = data[i] as usize;
        i += 1;
        if run > 128 {
            let len = 256 - run;
            let byte = data[i];
            for _ in 0..len + 1 {
                buf.push(byte)
            }
            i += 1;            
        } else if run < 128 {
            for _ in 0..run + 1 {
                buf.push(data[i]);
                i += 1;
            }
        }
    }
    Ok(buf)
}