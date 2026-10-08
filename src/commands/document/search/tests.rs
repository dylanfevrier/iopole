use super::*;
use crate::cli::DirectionFilter;
use crate::database::testing::{InMemoryKeyStore, InvoiceRecord};

const IDENTIFIER: &str = "352862346";

fn args() -> DocumentSearchArgs {
    DocumentSearchArgs {
        identifier_value: IDENTIFIER.to_owned(),
        role: None,
        direction: DirectionFilter::Both,
        states: Vec::new(),
        statuses: Vec::new(),
        from: None,
        to: None,
        limit: 50,
    }
}

struct Fixture {
    tmp: tempfile::TempDir,
    keys: InMemoryKeyStore,
}

impl Fixture {
    fn new(records: &[InvoiceRecord<'_>]) -> Result<Self> {
        let fixture = Self {
            tmp: tempfile::tempdir()?,
            keys: InMemoryKeyStore::default(),
        };
        let connection = database::open(&fixture.path(), &fixture.keys)?;
        for record in records {
            record.insert(&connection)?;
        }
        Ok(fixture)
    }

    fn path(&self) -> std::path::PathBuf {
        self.tmp.path().join("iopole.db")
    }

    fn search(&self, args: &DocumentSearchArgs) -> Result<String> {
        let mut out = Vec::new();
        execute(&self.path(), &self.keys, args, &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }
}

fn delivered() -> InvoiceRecord<'static> {
    InvoiceRecord {
        id: "01a2",
        created_at: "2026-09-02T23:30:00.000+00:00",
        number: Some("F-2026-0042"),
        amount: Some("1250.00"),
        seller_name: Some("CCLS"),
        seller_identifier: Some(IDENTIFIER),
        buyer_name: Some("LES RAVALEURS MANTOIS ET ASSOCIES DE LA VALLEE"),
        state: "DELIVERED",
        last_status_code: Some("APPROVED"),
        ..InvoiceRecord::default()
    }
}

fn pending() -> InvoiceRecord<'static> {
    InvoiceRecord {
        id: "01a1",
        direction: "OUTBOUND",
        amount: Some("60"),
        buyer_name: Some("CCLS"),
        buyer_identifier: Some(IDENTIFIER),
        state: "PENDING",
        ..InvoiceRecord::default()
    }
}

#[test]
fn prints_matching_invoices_as_a_table() -> Result<()> {
    let fixture = Fixture::new(&[pending(), delivered()])?;

    let output = fixture.search(&args())?;

    assert_eq!(
        output,
        "\
DATE        DIRECTION  NUMBER       SELLER  BUYER                            AMOUNT  STATE      STATUS
2026-09-02  inbound    F-2026-0042  CCLS    LES RAVALEURS MANTOIS ET ASSO…  1250.00  DELIVERED  APPROVED
2026-09-01  outbound   -            -       CCLS                                 60  PENDING    -

2 invoices found.
"
    );
    Ok(())
}

#[test]
fn applies_filters() -> Result<()> {
    let fixture = Fixture::new(&[pending(), delivered()])?;
    let pending_only = DocumentSearchArgs {
        states: vec!["PENDING".to_owned()],
        ..args()
    };

    let output = fixture.search(&pending_only)?;

    assert!(output.contains("PENDING"));
    assert!(!output.contains("DELIVERED"));
    Ok(())
}

#[test]
fn tells_when_results_are_truncated() -> Result<()> {
    let fixture = Fixture::new(&[pending(), delivered()])?;
    let one = DocumentSearchArgs { limit: 1, ..args() };

    let output = fixture.search(&one)?;

    assert!(output.ends_with("\nShowing 1 of 2 invoices; use --limit to see more.\n"));
    Ok(())
}

#[test]
fn invites_to_sync_when_nothing_matches() -> Result<()> {
    let fixture = Fixture::new(&[delivered()])?;
    let unknown = DocumentSearchArgs {
        identifier_value: "000000000".to_owned(),
        ..args()
    };

    assert_eq!(
        fixture.search(&unknown)?,
        "No invoice found.\nRun `iopole document sync --from <date>` to retrieve invoices from Iopole, then search again.\n"
    );
    Ok(())
}

#[test]
fn sync_invitation_reuses_the_searched_period_and_direction() -> Result<()> {
    let fixture = Fixture::new(&[])?;
    let september = DocumentSearchArgs {
        from: Some("2026-09-01T00:00:00Z".parse()?),
        to: Some("2026-10-01T00:00:00Z".parse()?),
        direction: DirectionFilter::Inbound,
        ..args()
    };

    assert!(fixture.search(&september)?.contains(
        "`iopole document sync --from 2026-09-01T00:00:00Z --to 2026-10-01T00:00:00Z --direction inbound`"
    ));
    Ok(())
}

#[test]
fn missing_database_is_not_created() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let path = tmp.path().join("iopole.db");
    let keys = InMemoryKeyStore::default();

    execute(&path, &keys, &args(), &mut Vec::new())?;

    assert!(!path.exists());
    assert!(keys.load_key()?.is_none());
    Ok(())
}
