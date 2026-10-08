use super::*;

const OBSERVED_PAGE: &str = r#"{
  "data": [
    {
      "businessEntityId": "0199b69f-fe32-701a-9604-617d13785726",
      "country": "FR",
      "countryIdentifier": { "siren": "980217632" },
      "disclosureStatus": "Public",
      "identifierScheme": "0002",
      "identifierValue": "980217632",
      "identifiers": [
        {
          "businessEntityIdentifierId": "0199b69f-fe32-701a-9604-6669329b91fe",
          "networkRegistered": [
            {
              "directoryAddress": "0225:980217632",
              "directoryId": "019fa93e-2169-75be-bbba-e5ad570ec5dc",
              "isSelfBilling": false,
              "networkId": "c11d52d5-40b8-4d71-83f1-1709cf47e834",
              "networkIdentifier": "PEPPOL_INTERNATIONAL",
              "validFrom": "2026-07-28"
            },
            {
              "directoryAddress": "0225:980217632",
              "directoryId": "019fa93e-1b8b-77a9-8e2b-c29a88f44712",
              "isSelfBilling": false,
              "networkId": "b00d52d5-40b8-4d71-83f1-1709cf47e812",
              "networkIdentifier": "DOMESTIC_FR",
              "validFrom": "2026-07-29"
            }
          ],
          "scheme": "0002",
          "type": "LEGAL_IDENTIFIER",
          "value": "980217632"
        }
      ],
      "name": "BVS RENOVATION",
      "operatorRelation": {
        "data": { "header": [ { "key": "X-Company-UID", "value": "6071797" } ] }
      },
      "scope": "PRIVATE_TAX_PAYER",
      "type": "LEGAL_UNIT",
      "vatRegime": "REAL_MONTHLY_TAX_REGIME"
    }
  ],
  "meta": { "count": 1, "limit": 200, "offset": 0 }
}"#;

fn entity() -> Result<BusinessEntity> {
    let mut entities = parse_entities(OBSERVED_PAGE)?;
    assert_eq!(entities.len(), 1);
    Ok(entities.remove(0))
}

#[test]
fn reads_name_country_and_company_uid() -> Result<()> {
    let entity = entity()?;

    assert_eq!(entity.name.as_deref(), Some("BVS RENOVATION"));
    assert_eq!(entity.country.as_deref(), Some("FR"));
    assert_eq!(entity.company_uid(), Some("6071797"));
    Ok(())
}

#[test]
fn reads_legal_identifier() -> Result<()> {
    let entity = entity()?;

    let legal = entity.legal_identifier();

    assert_eq!(legal.and_then(|id| id.scheme.as_deref()), Some("0002"));
    assert_eq!(legal.and_then(|id| id.value.as_deref()), Some("980217632"));
    Ok(())
}

#[test]
fn reads_network_registrations() -> Result<()> {
    let entity = entity()?;

    let networks: Vec<_> = entity
        .network_registrations()
        .map(|network| {
            (
                network.network_identifier.as_deref(),
                network.directory_address.as_deref(),
                network.valid_from.as_deref(),
            )
        })
        .collect();

    assert_eq!(
        networks,
        [
            (
                Some("PEPPOL_INTERNATIONAL"),
                Some("0225:980217632"),
                Some("2026-07-28")
            ),
            (
                Some("DOMESTIC_FR"),
                Some("0225:980217632"),
                Some("2026-07-29")
            ),
        ]
    );
    Ok(())
}

#[test]
fn everything_is_optional() -> Result<()> {
    let entities = parse_entities(r#"{"data":[{}]}"#)?;

    assert_eq!(entities, [BusinessEntity::default()]);
    assert_eq!(entities[0].company_uid(), None);
    assert_eq!(entities[0].legal_identifier(), None);
    assert_eq!(entities[0].network_registrations().count(), 0);
    Ok(())
}

#[test]
fn query_matches_the_exact_identifier_value() {
    assert_eq!(
        identifier_query("980217632"),
        r#"identifierValue:"980217632""#
    );
}

#[test]
fn query_escapes_quotes() {
    assert_eq!(identifier_query(r#"a"b\c"#), r#"identifierValue:"a\"b\\c""#);
}
