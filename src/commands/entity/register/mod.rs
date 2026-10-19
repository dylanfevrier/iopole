mod args;
mod render;

use std::io::Write;

pub(crate) use args::Args;

use super::single_entity;
use crate::credentials::{CredentialStore, Credentials};
use crate::iopole::{BusinessEntity, BusinessEntityDirectory, IdentifierRegistration};
use crate::{Error, Result};

pub(crate) fn execute<C: BusinessEntityDirectory + IdentifierRegistration>(
    credentials: &impl CredentialStore,
    connect: impl FnOnce(Credentials) -> C,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let mut iopole = connect(credentials);
    let entity = single_entity(iopole.find(&args.identifier_value)?, &args.identifier_value)?;
    let scheme = scheme_of(&entity, &args.identifier_value)?;
    render::entity(out, &entity, &scheme, &args.identifier_value)?;
    // Each outcome is shown as it comes, so that a refusal on the second
    // network still tells what happened on the first.
    for &network in args.network.networks() {
        let registration = iopole.register_identifier(&scheme, &args.identifier_value, network)?;
        render::registration(out, network, registration)?;
    }
    Ok(())
}

/// The scheme Iopole knows the identifier under: `0002` for a SIREN, `0009`
/// for a SIRET...
fn scheme_of(entity: &BusinessEntity, identifier_value: &str) -> Result<String> {
    entity
        .identifiers
        .iter()
        .find(|identifier| identifier.value.as_deref() == Some(identifier_value))
        .and_then(|identifier| identifier.scheme.clone())
        .ok_or_else(|| {
            Error::Payload(serde::de::Error::custom(format!(
                "the business entity of {identifier_value} has no scheme for it"
            )))
        })
}

#[cfg(test)]
mod tests {
    use super::args::NetworkChoice;
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::iopole::Network;
    use crate::iopole::testing::{FakeDirectory, FakeRegistrations, bvs_renovation};

    const SIREN: &str = "980217632";

    fn args(network: NetworkChoice) -> Args {
        Args {
            identifier_value: SIREN.to_owned(),
            network,
        }
    }

    fn registered(entities: Vec<BusinessEntity>) -> FakeRegistrations {
        FakeRegistrations {
            directory: FakeDirectory {
                entities,
                ..FakeDirectory::default()
            },
            ..FakeRegistrations::default()
        }
    }

    fn run(registrations: &mut FakeRegistrations, network: NetworkChoice) -> (Result<()>, String) {
        let mut out = Vec::new();
        let result = execute(
            &InMemoryStore::logged_in(),
            |_| &mut *registrations,
            &args(network),
            &mut out,
        );
        (result, String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn registers_the_identifier_under_its_scheme_on_both_networks() -> Result<()> {
        let mut registrations = FakeRegistrations {
            up_to_date: vec![Network::DomesticFr],
            ..registered(vec![bvs_renovation()])
        };

        let (result, output) = run(&mut registrations, NetworkChoice::Both);

        result?;
        assert_eq!(registrations.directory.searched, [SIREN]);
        assert_eq!(
            registrations.registered,
            [(
                "0002".to_owned(),
                SIREN.to_owned(),
                Network::PeppolInternational
            )]
        );
        assert_eq!(
            output,
            "Name:         BVS RENOVATION\nIdentifier:   0002:980217632\n\n\
             DOMESTIC_FR: already up to date\n\
             PEPPOL_INTERNATIONAL: registered\n"
        );
        Ok(())
    }

    #[test]
    fn registers_on_the_chosen_network_only() -> Result<()> {
        let mut registrations = registered(vec![bvs_renovation()]);

        let (result, _) = run(&mut registrations, NetworkChoice::PeppolInternational);

        result?;
        assert_eq!(registrations.registered.len(), 1);
        assert_eq!(registrations.registered[0].2, Network::PeppolInternational);
        Ok(())
    }

    #[test]
    fn refuses_an_unregistered_identifier() {
        let mut registrations = registered(Vec::new());

        let (result, output) = run(&mut registrations, NetworkChoice::Both);

        assert!(matches!(result, Err(Error::UnregisteredIdentifier(value)) if value == SIREN));
        assert_eq!(output, "");
        assert_eq!(registrations.registered, []);
    }

    #[test]
    fn refuses_to_guess_between_several_entities() {
        let mut registrations = registered(vec![bvs_renovation(), bvs_renovation()]);

        let (result, _) = run(&mut registrations, NetworkChoice::Both);

        assert!(matches!(
            result,
            Err(Error::AmbiguousIdentifier { count: 2, .. })
        ));
        assert_eq!(registrations.registered, []);
    }

    #[test]
    fn reports_a_refusal_without_claiming_success() {
        let mut registrations = FakeRegistrations {
            refusal: Some(Error::UnexpectedStatus {
                status: 404,
                code: Some("BUSINESS_ENTITY_NOT_FOUND".to_owned()),
            }),
            ..registered(vec![bvs_renovation()])
        };

        let (result, output) = run(&mut registrations, NetworkChoice::Both);

        assert!(matches!(result, Err(Error::UnexpectedStatus { .. })));
        assert!(!output.contains("registered"));
    }

    #[test]
    fn refuses_when_not_logged_in() {
        let result = execute(
            &InMemoryStore::default(),
            |_| -> &mut FakeRegistrations { panic!("must not connect without credentials") },
            &args(NetworkChoice::Both),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
    }
}
