//! Tests d'intégration du binaire `iopole`.

use assert_cmd::Command;
use predicates::prelude::*;

fn iopole() -> Command {
    Command::cargo_bin("iopole").unwrap_or_else(|err| panic!("binaire introuvable : {err}"))
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
    iopole().arg("inconnue").assert().failure();
}

#[test]
fn version_flag_prints_version() {
    iopole()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
