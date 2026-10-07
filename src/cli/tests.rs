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
    let cli = Cli::try_parse_from(["iopole", "sync", "--from", "2026-09-01"])?;

    let Command::Sync(args) = cli.command else {
        panic!("expected sync, got {:?}", cli.command);
    };
    assert_eq!(args.direction, DirectionFilter::Both);
    assert_eq!(args.to, None);
    Ok(())
}
