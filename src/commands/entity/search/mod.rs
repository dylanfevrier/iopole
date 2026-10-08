mod args;
mod render;

use std::io::Write;

pub(crate) use args::Args;

use crate::credentials::{CredentialStore, Credentials};
use crate::iopole::BusinessEntityDirectory;
use crate::{Error, Result};

pub(crate) fn execute<D: BusinessEntityDirectory>(
    credentials: &impl CredentialStore,
    connect: impl FnOnce(Credentials) -> D,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let mut directory = connect(credentials);
    if args.json {
        return render::json(out, &directory.find_raw(&args.identifier_value)?);
    }
    let entities = directory.find(&args.identifier_value)?;
    render::entities(out, &args.identifier_value, &entities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::iopole::testing::{FakeDirectory, bvs_renovation};

    fn args(identifier_value: &str) -> Args {
        Args {
            identifier_value: identifier_value.to_owned(),
            json: false,
        }
    }

    #[test]
    fn shows_the_entities_having_the_identifier() -> Result<()> {
        let mut out = Vec::new();
        let directory = FakeDirectory {
            entities: vec![bvs_renovation()],
            ..FakeDirectory::default()
        };

        execute(
            &InMemoryStore::logged_in(),
            |_| directory,
            &args("980217632"),
            &mut out,
        )?;

        assert!(String::from_utf8_lossy(&out).starts_with("Name:         BVS RENOVATION\n"));
        Ok(())
    }

    #[test]
    fn prints_the_entities_as_iopole_returns_them_with_json() -> Result<()> {
        let mut out = Vec::new();
        let directory = FakeDirectory {
            raw: vec![serde_json::json!({ "name": "BVS RENOVATION" })],
            ..FakeDirectory::default()
        };

        execute(
            &InMemoryStore::logged_in(),
            |_| directory,
            &Args {
                json: true,
                ..args("980217632")
            },
            &mut out,
        )?;

        assert_eq!(
            String::from_utf8_lossy(&out),
            "[\n  {\n    \"name\": \"BVS RENOVATION\"\n  }\n]\n"
        );
        Ok(())
    }

    #[test]
    fn prints_an_empty_list_with_json_when_nothing_matches() -> Result<()> {
        let mut out = Vec::new();

        execute(
            &InMemoryStore::logged_in(),
            |_| FakeDirectory::default(),
            &Args {
                json: true,
                ..args("352862346")
            },
            &mut out,
        )?;

        assert_eq!(String::from_utf8_lossy(&out), "[]\n");
        Ok(())
    }

    #[test]
    fn refuses_when_not_logged_in() {
        let result = execute(
            &InMemoryStore::default(),
            |_| -> FakeDirectory { panic!("must not connect without credentials") },
            &args("352862346"),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
    }
}
