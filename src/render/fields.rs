use std::fmt;

/// Fits `Company UID:` and two spaces, so that every command lines up alike;
/// a longer label still keeps two spaces before its value.
const LABEL_WIDTH: usize = 14;
const GAP: usize = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Fields {
    lines: Vec<(&'static str, String)>,
}

impl Fields {
    pub(crate) fn push(&mut self, label: &'static str, value: impl Into<String>) {
        self.lines.push((label, value.into()));
    }

    pub(crate) fn push_some(&mut self, label: &'static str, value: Option<impl Into<String>>) {
        if let Some(value) = value {
            self.push(label, value);
        }
    }
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (label, value) in &self.lines {
            let label = format!("{label}:");
            let width = LABEL_WIDTH.max(label.chars().count() + GAP);
            writeln!(f, "{label:<width$}{value}")?;
        }
        Ok(())
    }
}
