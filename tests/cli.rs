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
fn help_lists_commands() {
    iopole()
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("document"))
        .stdout(predicate::str::contains("entity"))
        .stdout(predicate::str::contains("directory"));
}

#[test]
fn auth_help_lists_login_and_logout() {
    iopole()
        .args(["auth", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("logout"));
}

#[test]
fn login_lives_under_auth() {
    iopole().arg("login").assert().failure();
    iopole().arg("logout").assert().failure();
}

#[test]
fn document_help_lists_its_commands() {
    iopole()
        .args(["document", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn document_status_takes_an_invoice_uuid() {
    iopole()
        .args(["document", "status", "260902440"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a UUID"));
}

#[test]
fn directory_help_lists_french() {
    iopole()
        .args(["directory", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("french"));
}

#[test]
fn directory_french_requires_a_siren_a_siret_or_a_name() {
    for args in [&[][..], &["--type", "office"]] {
        iopole()
            .args(["directory", "french"])
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "<SIREN|--siret <SIRET>|--name <NAME>>",
            ));
    }
}

#[test]
fn directory_french_rejects_malformed_criteria() {
    for (args, message) in [
        (&["12345678"][..], "a SIREN has exactly 9 digits"),
        (&["12345678A"], "a SIREN has exactly 9 digits"),
        (&["--siret", "007350101"], "a SIRET has exactly 14 digits"),
        (&["--name", "OB*"], "at least 3 characters are required"),
        (&["123456789", "--limit", "0"], "--limit"),
        (&["123456789", "--limit", "201"], "--limit"),
    ] {
        iopole()
            .args(["directory", "french"])
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains(message));
    }
}
