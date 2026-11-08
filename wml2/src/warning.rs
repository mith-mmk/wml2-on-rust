//! Warning aggregation traits and container types.

use std::fmt::*;

/// Trait implemented by warning category enums.
pub trait WarningKind {
    fn as_str(&self) -> &'static str;
}

/// Marker trait for displayable warnings.
///
/// There is no common typed category or severity contract. Implementations from
/// callers remain supported; display text should not be parsed as an error code.
pub trait ImgWarning: Display + Debug {}

/// A collection of warnings produced during decode or encode.
///
/// Warnings describe recoverable conditions reported by a particular codec,
/// not a complete validation report. Some codecs intentionally ignore optional
/// metadata. Legacy `Display` concatenates messages without inserting separators;
/// use [`Self::iter`] to display individual warnings in a UI or log.
pub struct ImgWarnings {
    pub(crate) warnings: Vec<Box<dyn ImgWarning>>,
}

impl Debug for ImgWarnings {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for warning in &self.warnings {
            std::fmt::Display::fmt(&warning, f)?;
        }
        Ok(())
    }
}

impl Display for ImgWarnings {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for warning in &self.warnings {
            write!(f, "{}", &warning)?;
        }
        Ok(())
    }
}

impl ImgWarnings {
    /// Number of reported warnings.
    pub fn len(&self) -> usize {
        self.warnings.len()
    }

    /// Whether the collection contains no warnings.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Borrows each warning in its original order, without merging messages.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &dyn ImgWarning> + DoubleEndedIterator {
        self.warnings.iter().map(|warning| warning.as_ref())
    }

    /// Adds one warning to an optional warning collection.
    pub fn add(warnings: Option<ImgWarnings>, warning: Box<dyn ImgWarning>) -> Option<Self> {
        match warnings {
            Some(mut w) => {
                w.warnings.push(warning);
                Some(w)
            }
            None => {
                let mut result: Vec<Box<dyn ImgWarning>> = Vec::new();
                result.push(warning);
                Some(ImgWarnings { warnings: result })
            }
        }
    }

    /// Appends one optional warning collection to another.
    pub fn append(
        mut warnings: Option<ImgWarnings>,
        warnings2: Option<ImgWarnings>,
    ) -> Option<Self> {
        if let Some(ws) = warnings2 {
            for w in ws.warnings {
                warnings = ImgWarnings::add(warnings, w);
            }
        }
        warnings
    }
}
