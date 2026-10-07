use clap::CommandFactory;

use super::*;

#[test]
fn cli_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn dates_are_utc_midnight() -> crate::Result<()> {
    assert_eq!(
        parse_instant("2026-09-01"),
        Ok("2026-09-01T00:00:00Z".parse()?)
    );
    Ok(())
}

#[test]
fn instants_keep_their_offset() -> crate::Result<()> {
    assert_eq!(
        parse_instant("2026-09-01T02:00:00+02:00"),
        Ok("2026-09-01T00:00:00Z".parse()?)
    );
    Ok(())
}

#[test]
fn rejects_other_formats() {
    assert!(parse_instant("01/09/2026").is_err());
}

#[test]
fn sync_defaults_to_both_directions() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from(["iopole", "document", "sync", "--from", "2026-09-01"])?;

    let Command::Document {
        command: DocumentCommand::Sync(args),
    } = cli.command
    else {
        panic!("expected sync, got {:?}", cli.command);
    };
    assert_eq!(args.direction, DirectionFilter::Both);
    assert_eq!(args.to, None);
    Ok(())
}

#[test]
fn search_accepts_several_states_in_any_case() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from([
        "iopole",
        "document",
        "search",
        "352862346",
        "--state",
        "valid,Delivered",
        "--state",
        "pending",
    ])?;

    let Command::Document {
        command: DocumentCommand::Search(args),
    } = cli.command
    else {
        panic!("expected search, got {:?}", cli.command);
    };
    assert_eq!(args.states, ["VALID", "DELIVERED", "PENDING"]);
    assert_eq!(args.role, None);
    assert_eq!(args.limit, 50);
    Ok(())
}

#[test]
fn search_accepts_identifiers_of_any_scheme() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from(["iopole", "document", "search", "0123456789"])?;

    let Command::Document {
        command: DocumentCommand::Search(args),
    } = cli.command
    else {
        panic!("expected search, got {:?}", cli.command);
    };
    assert_eq!(args.identifier_value, "0123456789");
    Ok(())
}

#[test]
fn search_rejects_identifier_with_scheme() {
    assert!(Cli::try_parse_from(["iopole", "document", "search", "0225:352862346"]).is_err());
    assert!(Cli::try_parse_from(["iopole", "document", "search", " "]).is_err());
}
