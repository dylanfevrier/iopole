use super::*;
use crate::credentials::testing::InMemoryStore;
use crate::iopole::testing::{FakeDirectory, bvs_renovation};

fn search(entities: Vec<BusinessEntity>, identifier_value: &str) -> Result<String> {
    let mut out = Vec::new();
    execute(
        &InMemoryStore::logged_in(),
        |_| FakeDirectory {
            entities,
            ..FakeDirectory::default()
        },
        &EntitySearchArgs {
            identifier_value: identifier_value.to_owned(),
        },
        &mut out,
    )?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[test]
fn shows_company_uid_legal_identifier_and_networks() -> Result<()> {
    let output = search(vec![bvs_renovation()], "980217632")?;

    assert_eq!(
        output,
        "\
980217632: 1 business entity registered with Iopole.

BVS RENOVATION
  Company UID       6071797
  Legal identifier  0002:980217632
  Country           FR
  Networks
    PEPPOL_INTERNATIONAL  0225:980217632  since 2026-07-28
    DOMESTIC_FR           0225:980217632  since 2026-07-29
"
    );
    Ok(())
}

#[test]
fn shows_entities_without_network_registration() -> Result<()> {
    let mut entity = bvs_renovation();
    entity.operator_relation = None;
    entity.identifiers[0].network_registered.clear();

    let output = search(vec![entity], "980217632")?;

    assert!(output.ends_with(
        "\
BVS RENOVATION
  Company UID       -
  Legal identifier  0002:980217632
  Country           FR
  Networks          none
"
    ));
    Ok(())
}

#[test]
fn reports_unregistered_company() -> Result<()> {
    assert_eq!(
        search(Vec::new(), "352862346")?,
        "352862346 is not registered with Iopole.\n"
    );
    Ok(())
}

#[test]
fn refuses_when_not_logged_in() {
    let result = execute(
        &InMemoryStore::default(),
        |_| -> FakeDirectory { panic!("must not connect without credentials") },
        &EntitySearchArgs {
            identifier_value: "352862346".to_owned(),
        },
        &mut Vec::new(),
    );

    assert!(matches!(result, Err(Error::NotLoggedIn)));
}
