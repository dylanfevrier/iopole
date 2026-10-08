mod fields;
mod table;

use std::fmt::Display;
use std::io::{self, Write};

pub(crate) use fields::Fields;
pub(crate) use table::Table;

pub(crate) const MISSING: &str = "-";
const RULE_WIDTH: usize = 64;

pub(crate) fn records(
    out: &mut impl Write,
    records: impl IntoIterator<Item = impl Display>,
) -> io::Result<()> {
    for (index, record) in records.into_iter().enumerate() {
        if index > 0 {
            writeln!(out, "\n{}\n", "=".repeat(RULE_WIDTH))?;
        }
        write!(out, "{record}")?;
    }
    Ok(())
}
