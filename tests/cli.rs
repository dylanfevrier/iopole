use assert_cmd::Command;
use predicates::prelude::*;

fn iopole() -> Command {
    Command::cargo_bin("iopole").unwrap_or_else(|err| panic!("binary not found: {err}"))
}

#[test]
fn help_flag_prints_usage() {
    iopole()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn unknown_command_fails() {
    iopole().arg("unknown").assert().failure();
}

#[test]
fn version_flag_prints_version() {
    iopole()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_auth_commands() {
    iopole()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("logout"));
}
