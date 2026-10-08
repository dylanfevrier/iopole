use std::fmt;

const GAP: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fields {
    width: usize,
    lines: Vec<(&'static str, String)>,
}

impl Fields {
    /// Takes every label a record may show, so that records missing some still line up.
    pub(crate) fn aligned_on(labels: &[&str]) -> Self {
        let longest = labels.iter().map(|label| label.chars().count()).max();
        Self {
            width: longest.unwrap_or(0) + ":".len() + GAP,
            lines: Vec::new(),
        }
    }

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
        let width = self.width;
        for (label, value) in &self.lines {
            writeln!(f, "{:<width$}{value}", format!("{label}:"))?;
        }
        Ok(())
    }
}
