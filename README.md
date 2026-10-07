# iopole

Command-line tool for the Iopole platform.

## Usage

```sh
cargo run -- --help
cargo run -- login
cargo run -- logout
cargo run -- sync --from 2026-09-01 [--to 2026-10-01] [--direction inbound|outbound|both]
```

### Sync

`sync` stores the general information of Iopole invoices (never payment
means) in an encrypted SQLite database whose key lives in the system keyring.

- Invoices are searched by ranges of their UUID v7 identifier, one day at a
  time; days holding more than the 1200 results a search returns are halved
  down to one minute.
- Each search also replays the last 10% of the previous window, so that no
  invoice falls between two searches. Invoices found again only get their
  status refreshed, and only when it is not older than the stored one.
- Ranges already synchronized are never searched again: running `sync` twice
  only fills the gaps. The last hour stays open and is searched again by the
  next run.
- Production is used by default. Set `IOPOLE_AUTH_URL` and `IOPOLE_PDP_URL`
  (and optionally `IOPOLE_AUTH_ENDPOINT`) to target another environment, for
  instance `https://auth.ppd.iopole.fr` and `https://api.ppd.iopole.fr`.

`sync` shows its progress, then the number of invoices retrieved:

```
████████████████████████████████████████ 100%
1234 invoices retrieved. 1000 inbound. 234 outbound.
```

## Installation

```sh
cargo install --path .
scripts/sign.sh ~/.cargo/bin/iopole
```

## Keychain access on macOS

Secrets are stored in a single keychain entry, which macOS only lets the
binary that created it read. Binaries built by cargo are signed ad hoc, so each
rebuild looks like a new application and macOS asks for access again. Sign them
with a stable identity to keep "Always Allow" across rebuilds:

```sh
scripts/create-signing-identity.sh
```

`cargo run` and `cargo test` then sign binaries automatically (see
`.cargo/config.toml`); installed binaries are signed with `scripts/sign.sh`.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
