use std::io::Write;

use super::StateSummary;
use crate::Result;
use crate::render::{MISSING, Table};

const NO_STATUS: &str = "(no status)";

/// Each state with its total, then its last statuses.
pub(super) fn states(out: &mut impl Write, states: &[StateSummary]) -> Result<()> {
    let mut table = Table::new(&["STATE", "STATUS", "COUNT"]).align_right("COUNT");
    for state in states {
        table.push([
            state.state.clone(),
            MISSING.to_owned(),
            state.count.to_string(),
        ]);
        for status in &state.statuses {
            table.push([
                MISSING.to_owned(),
                status
                    .status
                    .clone()
                    .unwrap_or_else(|| NO_STATUS.to_owned()),
                status.count.to_string(),
            ]);
        }
    }
    write!(out, "{table}")?;
    let total: usize = states.iter().map(|state| state.count).sum();
    writeln!(out, "\n{total} invoices.")?;
    Ok(())
}

pub(super) fn no_invoice(out: &mut impl Write) -> Result<()> {
    writeln!(out, "No invoice synchronized yet.")?;
    writeln!(
        out,
        "Run `iopole document sync --from <date>` to retrieve invoices from Iopole."
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::document::stat::StatusCount;

    #[test]
    fn shows_each_state_then_its_statuses() -> Result<()> {
        let status = |status: Option<&str>, count| StatusCount {
            status: status.map(str::to_owned),
            count,
        };
        let mut out = Vec::new();

        states(
            &mut out,
            &[
                StateSummary {
                    state: "DELIVERED".to_owned(),
                    count: 1000,
                    statuses: vec![status(Some("RECEIVED"), 580), status(Some("ISSUED"), 420)],
                },
                StateSummary {
                    state: "WRONG_ROUTING".to_owned(),
                    count: 3000,
                    statuses: vec![status(None, 3000)],
                },
            ],
        )?;

        assert_eq!(
            String::from_utf8_lossy(&out),
            "\
STATE          STATUS       COUNT
DELIVERED      -             1000
-              RECEIVED       580
-              ISSUED         420
WRONG_ROUTING  -             3000
-              (no status)   3000

4000 invoices.
"
        );
        Ok(())
    }
}
