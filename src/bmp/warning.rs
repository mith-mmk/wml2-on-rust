use crate::warning::WarningKind;


#[derive(Debug)]
pub enum BMPWarningKind {
    OutOfMemory,
    DataCorruption,
    BufferOverrun,
}

#[allow(unused)]
#[allow(non_snake_case)]
impl WarningKind for BMPWarningKind {
    fn as_str(&self) -> &'static str {
        match &*self {
            OutOfMemory => {"Out of memory"},
            DataCorruption => {"Data Corruption"},
            BufferOverrun => {"Buffer Overrun"},
        }
    }
}