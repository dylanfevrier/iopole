mod args;
mod render;

use std::io::Write;
use std::path::Path;

pub(crate) use args::{Args, RoleFilter, parse_code};

use crate::Result;
use crate::catalog::{self, InvoiceFilter, Page};
use crate::database;

pub(crate) fn execute(database_path: &Path, args: &Args, out: &mut impl Write) -> Result<()> {
    let page = find(database_path, &args.filter()?)?;
    if args.json {
        return render::json(out, &page);
    }
    if page.total == 0 {
        return render::no_invoice(out, &args.sync_command());
    }
    render::invoices(out, &page, args.identifiers)
}

/// Nothing without local database, which is not created.
pub(crate) fn find(database_path: &Path, filter: &InvoiceFilter) -> Result<Page> {
    if !database_path.exists() {
        return Ok(Page::default());
    }
    catalog::find(&database::open(database_path)?, filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::document::DirectionFilter;
    use crate::database::testing::InvoiceRecord;

    const IDENTIFIER: &str = "352862346";

    fn args() -> Args {
        Args {
            identifier_value: Some(IDENTIFIER.to_owned()),
            role: None,
            direction: DirectionFilter::Both,
            states: Vec::new(),
            statuses: Vec::new(),
            from: None,
            to: None,
            limit: 50,
            json: false,
            identifiers: false,
        }
    }

    struct Fixture {
        tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new(records: &[InvoiceRecord<'_>]) -> Result<Self> {
            let fixture = Self {
                tmp: tempfile::tempdir()?,
            };
            let connection = database::open(&fixture.path())?;
            for record in records {
                record.insert(&connection)?;
            }
            Ok(fixture)
        }

        fn path(&self) -> std::path::PathBuf {
            self.tmp.path().join("iopole.db")
        }

        fn search(&self, args: &Args) -> Result<String> {
            let mut out = Vec::new();
            execute(&self.path(), args, &mut out)?;
            Ok(String::from_utf8_lossy(&out).into_owned())
        }
    }

    fn delivered() -> InvoiceRecord<'static> {
        InvoiceRecord {
            id: "01a2",
            seller_identifier: Some(IDENTIFIER),
            state: "DELIVERED",
            ..InvoiceRecord::default()
        }
    }

    fn pending() -> InvoiceRecord<'static> {
        InvoiceRecord {
            id: "01a1",
            direction: "OUTBOUND",
            buyer_identifier: Some(IDENTIFIER),
            state: "PENDING",
            ..InvoiceRecord::default()
        }
    }

    #[test]
    fn finds_the_invoices_of_the_company_in_the_database() -> Result<()> {
        let fixture = Fixture::new(&[pending(), delivered()])?;

        let output = fixture.search(&args())?;

        assert!(output.contains("DELIVERED"));
        assert!(output.contains("PENDING"));
        assert!(output.ends_with("\n2 invoices found.\n"));
        Ok(())
    }

    #[test]
    fn applies_filters() -> Result<()> {
        let fixture = Fixture::new(&[pending(), delivered()])?;
        let pending_only = Args {
            states: vec!["PENDING".to_owned()],
            ..args()
        };

        let output = fixture.search(&pending_only)?;

        assert!(output.contains("PENDING"));
        assert!(!output.contains("DELIVERED"));
        Ok(())
    }

    #[test]
    fn finds_the_invoices_of_every_company_without_identifier() -> Result<()> {
        let fixture = Fixture::new(&[
            pending(),
            delivered(),
            InvoiceRecord {
                id: "01a3",
                seller_identifier: Some("517657334"),
                ..InvoiceRecord::default()
            },
        ])?;
        let everyone = Args {
            identifier_value: None,
            ..args()
        };

        assert!(
            fixture
                .search(&everyone)?
                .ends_with("\n3 invoices found.\n")
        );
        Ok(())
    }

    #[test]
    fn invites_to_sync_when_nothing_matches() -> Result<()> {
        let fixture = Fixture::new(&[delivered()])?;
        let unknown = Args {
            identifier_value: Some("000000000".to_owned()),
            ..args()
        };

        assert!(fixture.search(&unknown)?.starts_with("No invoice found.\n"));
        Ok(())
    }

    #[test]
    fn prints_the_invoices_as_iopole_returned_them_with_json() -> Result<()> {
        let fixture = Fixture::new(&[
            InvoiceRecord {
                raw: r#"{"invoiceId":"01a2","state":"DELIVERED"}"#,
                ..delivered()
            },
            InvoiceRecord {
                raw: r#"{"invoiceId":"01a1"}"#,
                ..pending()
            },
        ])?;
        let json = Args {
            json: true,
            ..args()
        };

        assert_eq!(
            fixture.search(&json)?,
            "\
[
  {
    \"invoiceId\": \"01a2\",
    \"state\": \"DELIVERED\"
  },
  {
    \"invoiceId\": \"01a1\"
  }
]
"
        );
        Ok(())
    }

    #[test]
    fn prints_an_empty_list_with_json_when_nothing_matches() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let json = Args {
            json: true,
            ..args()
        };
        let mut out = Vec::new();

        execute(&tmp.path().join("iopole.db"), &json, &mut out)?;

        assert_eq!(String::from_utf8_lossy(&out), "[]\n");
        Ok(())
    }

    #[test]
    fn missing_database_is_not_created() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        execute(&path, &args(), &mut Vec::new())?;

        assert!(!path.exists());
        Ok(())
    }
}
