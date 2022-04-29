use std::fmt::*;

pub trait WarningKind {
    fn as_str(&self) -> &'static str;
}

pub trait ImgWarning: Display + Debug {

}

pub struct ImgWarnings {
    pub(crate) warnings: Vec<Box<dyn ImgWarning>>,
}



impl Debug for ImgWarnings {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Ok(for warning in &self.warnings {
            std::fmt::Display::fmt(&warning, f)?;
        })
    }
}

impl Display for ImgWarnings {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Ok(for warning in &self.warnings {
            write!(f,"{}",&warning)?;
        })
    }
}

impl ImgWarnings {
    pub fn add(warnings: Option<ImgWarnings>,warning: Box<dyn ImgWarning>) -> Option<Self> {
        match warnings {
            Some(mut w) => {
                w.warnings.push(warning);
                Some(w)
            },
            None => {
                let mut result: Vec<Box<dyn ImgWarning>> = Vec::new();
                result.push(warning);
                Some(ImgWarnings{warnings:result})
            }
        }
    }
}
