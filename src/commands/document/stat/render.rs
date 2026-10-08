use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::io::Write;

use crate::Result;
use crate::catalog::StateCount;
use crate::render::{MISSING, Table};

const NO_STATUS: &str = "(no status)";

/// Each state with its total, then its last statuses, most frequent first.
pub(super) fn counts(out: &mut impl Write, counts: Vec<StateCount>) -> Result<()> {
    let mut states: BTreeMap<String, Vec<(Option<String>, usize)>> = BTreeMap::new();
    for count in counts {
        states
            .entry(count.state)
            .or_default()
            .push((count.last_status_code, count.count));
    }

    let mut table = Table::new(&["STATE", "STATUS", "COUNT"]).align_right("COUNT");
    let mut total = 0;
    for (state, mut statuses) in states {
        let state_total: usize = statuses.iter().map(|(_, count)| count).sum();
        total += state_total;
        table.push([state, MISSING.to_owned(), state_total.to_string()]);
        statuses.sort_by(|(a_status, a_count), (b_status, b_count)| {
            (Reverse(a_count), a_status).cmp(&(Reverse(b_count), b_status))
        });
        for (status, count) in statuses {
            table.push([
                MISSING.to_owned(),
                status.unwrap_or_else(|| NO_STATUS.to_owned()),
                count.to_string(),
            ]);
        }
    }
    write!(out, "{table}")?;
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

    fn count(state: &str, status: Option<&str>, count: usize) -> StateCount {
        StateCount {
            state: state.to_owned(),
            last_status_code: status.map(str::to_owned),
            count,
        }
    }

    #[test]
    fn details_each_state_by_status_most_frequent_first() -> Result<()> {
        let mut out = Vec::new();

        counts(
            &mut out,
            vec![
                count("WRONG_ROUTING", None, 3000),
                count("DELIVERED", Some("ISSUED"), 420),
                count("DELIVERED", Some("RECEIVED"), 580),
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
