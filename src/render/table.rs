use std::fmt;

const COLUMN_GAP: &str = "  ";

/// Plain-text table, columns aligned with spaces so that it does not depend on
/// the tab width of the terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Table {
    header: Vec<&'static str>,
    right_aligned: Vec<bool>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn new(header: &[&'static str]) -> Self {
        Self {
            header: header.to_vec(),
            right_aligned: vec![false; header.len()],
            rows: Vec::new(),
        }
    }

    pub(crate) fn align_right(mut self, header: &str) -> Self {
        for (column, aligned) in self.header.iter().zip(&mut self.right_aligned) {
            *aligned |= *column == header;
        }
        self
    }

    pub(crate) fn push(&mut self, cells: impl IntoIterator<Item = String>) {
        let mut row: Vec<String> = cells.into_iter().take(self.header.len()).collect();
        row.resize(self.header.len(), String::new());
        self.rows.push(row);
    }

    fn shown_columns(&self) -> Vec<usize> {
        (0..self.header.len())
            .filter(|&column| self.rows.iter().any(|row| !row[column].is_empty()))
            .collect()
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let columns = self.shown_columns();
        let header: Vec<&str> = columns.iter().map(|&column| self.header[column]).collect();
        let lines = std::iter::once(header).chain(
            self.rows
                .iter()
                .map(|row| columns.iter().map(|&column| row[column].as_str()).collect()),
        );
        let lines: Vec<Vec<&str>> = lines.collect();

        let widths: Vec<usize> = (0..columns.len())
            .map(|index| {
                lines
                    .iter()
                    .map(|line| line[index].chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for line in lines {
            let cells: Vec<String> = line
                .iter()
                .zip(&widths)
                .zip(&columns)
                .map(|((cell, &width), &column)| {
                    if self.right_aligned[column] {
                        format!("{cell:>width$}")
                    } else {
                        format!("{cell:<width$}")
                    }
                })
                .collect();
            writeln!(f, "{}", cells.join(COLUMN_GAP).trim_end())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_can_be_aligned_on_the_right() {
        let mut table = Table::new(&["BUYER", "AMOUNT"]).align_right("AMOUNT");
        table.push(["CCLS".to_owned(), "1250.00".to_owned()]);
        table.push(["OBAT".to_owned(), "60".to_owned()]);

        assert_eq!(
            table.to_string(),
            "BUYER   AMOUNT\nCCLS   1250.00\nOBAT        60\n"
        );
    }

    #[test]
    fn accented_values_stay_aligned() {
        let mut table = Table::new(&["CITY", "POSTAL CODE"]);
        table.push(["SAINT-ÉTIENNE".to_owned(), "42000".to_owned()]);
        table.push(["LYON".to_owned(), "69003".to_owned()]);

        assert_eq!(
            table.to_string(),
            "CITY           POSTAL CODE\nSAINT-ÉTIENNE  42000\nLYON           69003\n"
        );
    }
}
