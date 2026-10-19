mod render;

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use clap::Args as ClapArgs;
use schemars::JsonSchema;
use serde::Serialize;

use super::DirectionFilter;
use crate::Result;
use crate::catalog::{self, StateCount};
use crate::database;
use crate::invoice::Direction;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Invoice flow
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub(crate) direction: DirectionFilter,
}

pub(crate) fn execute(database_path: &Path, args: &Args, out: &mut impl Write) -> Result<()> {
    let states = summarize(database_path, args.direction.directions())?;
    if states.is_empty() {
        return render::no_invoice(out);
    }
    render::states(out, &states)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct StateSummary {
    /// Iopole processing state, e.g. `DELIVERED` or `WRONG_ROUTING`.
    pub(crate) state: String,
    /// Invoices in this state.
    pub(crate) count: usize,
    /// The same invoices by last lifecycle status, most frequent first.
    pub(crate) statuses: Vec<StatusCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct StatusCount {
    /// Last lifecycle status exchanged on the network, e.g. `RECEIVED`; null
    /// when none was exchanged yet.
    pub(crate) status: Option<String>,
    pub(crate) count: usize,
}

/// The synchronized invoices of these directions by state, in alphabetical
/// order; nothing without local database, which is not created.
pub(crate) fn summarize(
    database_path: &Path,
    directions: &[Direction],
) -> Result<Vec<StateSummary>> {
    if !database_path.exists() {
        return Ok(Vec::new());
    }
    let counts = catalog::count_by_state(&database::open(database_path)?, directions)?;
    Ok(group(counts))
}

fn group(counts: Vec<StateCount>) -> Vec<StateSummary> {
    let mut states: BTreeMap<String, Vec<StatusCount>> = BTreeMap::new();
    for count in counts {
        states.entry(count.state).or_default().push(StatusCount {
            status: count.last_status_code,
            count: count.count,
        });
    }
    states
        .into_iter()
        .map(|(state, mut statuses)| {
            statuses
                .sort_by(|a, b| (Reverse(a.count), &a.status).cmp(&(Reverse(b.count), &b.status)));
            StateSummary {
                state,
                count: statuses.iter().map(|status| status.count).sum(),
                statuses,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::InvoiceRecord;

    fn stat(database_path: &Path) -> Result<String> {
        stat_with(database_path, DirectionFilter::Both)
    }

    fn stat_with(database_path: &Path, direction: DirectionFilter) -> Result<String> {
        let mut out = Vec::new();
        execute(database_path, &Args { direction }, &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn groups_states_alphabetically_and_statuses_most_frequent_first() {
        let count = |state: &str, status: Option<&str>, count| StateCount {
            state: state.to_owned(),
            last_status_code: status.map(str::to_owned),
            count,
        };
        let status = |status: Option<&str>, count| StatusCount {
            status: status.map(str::to_owned),
            count,
        };

        assert_eq!(
            group(vec![
                count("WRONG_ROUTING", None, 3000),
                count("DELIVERED", Some("ISSUED"), 420),
                count("DELIVERED", Some("RECEIVED"), 580),
            ]),
            [
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
            ]
        );
    }

    #[test]
    fn counts_the_synchronized_invoices() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let connection = database::open(&path)?;
        for (id, status) in [
            ("01a1", Some("RECEIVED")),
            ("01a2", Some("RECEIVED")),
            ("01a3", None),
        ] {
            InvoiceRecord {
                id,
                state: "DELIVERED",
                last_status_code: status,
                ..InvoiceRecord::default()
            }
            .insert(&connection)?;
        }

        assert_eq!(
            stat(&path)?,
            "\
STATE      STATUS       COUNT
DELIVERED  -                3
-          RECEIVED         2
-          (no status)      1

3 invoices.
"
        );
        Ok(())
    }

    #[test]
    fn counts_only_the_invoices_of_the_direction() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let connection = database::open(&path)?;
        for (id, direction) in [
            ("01a1", "INBOUND"),
            ("01a2", "OUTBOUND"),
            ("01a3", "OUTBOUND"),
        ] {
            InvoiceRecord {
                id,
                direction,
                ..InvoiceRecord::default()
            }
            .insert(&connection)?;
        }

        assert!(stat_with(&path, DirectionFilter::Outbound)?.ends_with("\n2 invoices.\n"));
        assert!(stat_with(&path, DirectionFilter::Inbound)?.ends_with("\n1 invoices.\n"));
        assert!(stat(&path)?.ends_with("\n3 invoices.\n"));
        Ok(())
    }

    #[test]
    fn invites_to_sync_without_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        assert!(stat(&path)?.starts_with("No invoice synchronized yet.\n"));
        assert!(!path.exists());
        Ok(())
    }
}
