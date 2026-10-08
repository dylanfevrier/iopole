mod args;
mod render;

use std::io::Write;

pub(crate) use args::Args;

use crate::credentials::{CredentialStore, Credentials};
use crate::iopole::{BusinessEntity, BusinessEntityDirectory, ClaimConfiguration};
use crate::{Error, Result};

pub(crate) fn execute<C: BusinessEntityDirectory + ClaimConfiguration>(
    credentials: &impl CredentialStore,
    connect: impl FnOnce(Credentials) -> C,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let mut iopole = connect(credentials);
    let entity = single_entity(iopole.find(&args.identifier_value)?, &args.identifier_value)?;
    let business_entity_id = entity
        .id
        .as_deref()
        .ok_or_else(|| missing_id(&args.identifier_value))?;
    iopole.update_claim(business_entity_id, &args.claim())?;
    render::updated(out, &entity, business_entity_id, args)
}

/// Looked up as `entity search` does: the claim must not land on a guess.
fn single_entity(
    mut entities: Vec<BusinessEntity>,
    identifier_value: &str,
) -> Result<BusinessEntity> {
    match entities.len() {
        0 => Err(Error::UnregisteredIdentifier(identifier_value.to_owned())),
        1 => Ok(entities.remove(0)),
        count => Err(Error::AmbiguousIdentifier {
            identifier_value: identifier_value.to_owned(),
            count,
        }),
    }
}

fn missing_id(identifier_value: &str) -> Error {
    Error::Payload(serde::de::Error::custom(format!(
        "the business entity of {identifier_value} has no businessEntityId"
    )))
}

#[cfg(test)]
mod tests {
    use super::args::ClaimDirection;
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::iopole::Claim;
    use crate::iopole::testing::{BVS_RENOVATION_ID, FakeClaims, FakeDirectory, bvs_renovation};

    const SIREN: &str = "980217632";

    fn args() -> Args {
        Args {
            identifier_value: SIREN.to_owned(),
            company_uid: "6071797".to_owned(),
            direction: ClaimDirection::Both,
        }
    }

    fn registered(entities: Vec<BusinessEntity>) -> FakeClaims {
        FakeClaims {
            directory: FakeDirectory {
                entities,
                ..FakeDirectory::default()
            },
            ..FakeClaims::default()
        }
    }

    fn run(claims: &mut FakeClaims) -> Result<String> {
        let mut out = Vec::new();
        execute(
            &InMemoryStore::logged_in(),
            |_| &mut *claims,
            &args(),
            &mut out,
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn replaces_the_claim_of_the_entity_having_the_identifier() -> Result<()> {
        let mut claims = registered(vec![bvs_renovation()]);

        let output = run(&mut claims)?;

        assert_eq!(claims.directory.searched, [SIREN]);
        assert_eq!(
            claims.updated,
            [(
                BVS_RENOVATION_ID.to_owned(),
                Claim {
                    company_uid: "6071797".to_owned(),
                    direction: None,
                }
            )]
        );
        assert_eq!(
            output,
            format!(
                "Claim updated.\n\nName:         BVS RENOVATION\nID:           {BVS_RENOVATION_ID}\nCompany UID:  6071797\nDirection:    both\n"
            )
        );
        Ok(())
    }

    #[test]
    fn refuses_an_unregistered_identifier() {
        let mut claims = registered(Vec::new());

        let result = run(&mut claims);

        assert!(matches!(result, Err(Error::UnregisteredIdentifier(value)) if value == SIREN));
        assert_eq!(claims.updated, []);
    }

    #[test]
    fn refuses_to_guess_between_several_entities() {
        let mut claims = registered(vec![bvs_renovation(), bvs_renovation()]);

        let result = run(&mut claims);

        assert!(matches!(
            result,
            Err(Error::AmbiguousIdentifier { count: 2, .. })
        ));
        assert_eq!(claims.updated, []);
    }

    #[test]
    fn reports_a_refusal_without_claiming_success() {
        let mut claims = FakeClaims {
            refusal: Some(Error::UnclaimedBusinessEntity(BVS_RENOVATION_ID.to_owned())),
            ..registered(vec![bvs_renovation()])
        };

        let result = run(&mut claims);

        assert!(matches!(result, Err(Error::UnclaimedBusinessEntity(_))));
    }

    #[test]
    fn refuses_when_not_logged_in() {
        let result = execute(
            &InMemoryStore::default(),
            |_| -> FakeClaims { panic!("must not connect without credentials") },
            &args(),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
    }
}
