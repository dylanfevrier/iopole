mod args;
mod render;

use std::io::Write;

pub(crate) use args::Args;

use crate::credentials::{CredentialStore, Credentials};
use crate::iopole::FrenchDirectory;
use crate::{Error, Result};

pub(crate) fn execute<D: FrenchDirectory>(
    credentials: &impl CredentialStore,
    connect: impl FnOnce(Credentials) -> D,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let page = connect(credentials).search_french(&args.query(), args.offset, args.limit)?;

    if args.json {
        return render::json(out, &page.body);
    }
    let page_is_full = page.entries.len() >= usize::try_from(args.limit).unwrap_or(usize::MAX);
    let next_offset = page_is_full.then(|| u64::from(args.offset) + u64::from(args.limit));
    render::entries(out, &page.entries, next_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::iopole::testing::{FakeFrenchDirectory, legal_unit_with_suffix};

    fn args() -> Args {
        Args {
            siren: Some("009460134".to_owned()),
            siret: None,
            name: None,
            entry_type: None,
            limit: 20,
            offset: 0,
            json: false,
        }
    }

    fn run(directory: &mut FakeFrenchDirectory, args: &Args) -> Result<String> {
        let mut out = Vec::new();
        execute(&InMemoryStore::logged_in(), |_| directory, args, &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn asks_the_directory_for_the_requested_page() -> Result<()> {
        let mut directory = FakeFrenchDirectory::default();
        let args = Args {
            limit: 5,
            offset: 10,
            ..args()
        };

        run(&mut directory, &args)?;

        assert_eq!(directory.searched, [(args.query(), 10, 5)]);
        Ok(())
    }

    #[test]
    fn shows_the_entries() -> Result<()> {
        let mut directory = FakeFrenchDirectory {
            entries: vec![legal_unit_with_suffix()],
            ..FakeFrenchDirectory::default()
        };

        let output = run(&mut directory, &args())?;

        assert!(output.starts_with("Name:        Legal unit Example 2\n"));
        assert!(!output.contains("More may match"));
        Ok(())
    }

    #[test]
    fn a_full_page_points_to_the_next_one() -> Result<()> {
        let mut directory = FakeFrenchDirectory {
            entries: vec![legal_unit_with_suffix(); 9],
            ..FakeFrenchDirectory::default()
        };
        let args = Args {
            limit: 2,
            offset: 4,
            ..args()
        };

        let output = run(&mut directory, &args)?;

        assert!(output.ends_with("More may match: run again with --offset 6.\n"));
        Ok(())
    }

    #[test]
    fn prints_the_raw_response_with_json() -> Result<()> {
        let mut directory = FakeFrenchDirectory {
            entries: vec![legal_unit_with_suffix()],
            body: Some(r#"{"data":[{"name":"Legal unit Example 2"}]}"#.to_owned()),
            ..FakeFrenchDirectory::default()
        };
        let args = Args {
            json: true,
            ..args()
        };

        let output = run(&mut directory, &args)?;

        assert!(output.starts_with("{\n  \"data\": [\n"));
        assert!(!output.contains("Name:"));
        Ok(())
    }

    #[test]
    fn refuses_when_not_logged_in() {
        let result = execute(
            &InMemoryStore::default(),
            |_| -> FakeFrenchDirectory { panic!("must not connect without credentials") },
            &args(),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
    }
}
