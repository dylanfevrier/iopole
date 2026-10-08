# iopole

Command-line tool for the Iopole platform.

## Usage

```sh
cargo run -- --help
cargo run -- auth login
cargo run -- auth logout
cargo run -- document sync --from 2026-09-01 [--to 2026-10-01] [--direction inbound|outbound|both]
cargo run -- document search 352862346
cargo run -- entity search 352862346
cargo run -- directory french 352862346
```

### Document sync

`document sync` stores the general information of Iopole invoices (never payment
means) in an encrypted SQLite database whose key lives in the system keyring.

- Invoices are searched by ranges of their UUID v7 identifier, one day at a
  time; days holding more than the 1200 results a search returns are halved
  down to one minute.
- Each search also replays the last 10% of the previous window, so that no
  invoice falls between two searches. Invoices found again only get their
  status refreshed, and only when it is not older than the stored one.
- Ranges already synchronized are never searched again: running
  `document sync` twice only fills the gaps. The last hour stays open and is
  searched again by the next run.
- Production is used by default. Set `IOPOLE_AUTH_URL` and `IOPOLE_PDP_URL`
  (and optionally `IOPOLE_AUTH_ENDPOINT`) to target another environment, for
  instance `https://auth.ppd.iopole.fr` and `https://api.ppd.iopole.fr`.

`document sync` shows its progress, then the number of invoices retrieved:

```
████████████████████████████████████████ 100%
1234 invoices retrieved. 1000 inbound. 234 outbound.
```

### Document search

`document search` looks for the invoices of a company, as seller or buyer, in
the local database only. It matches the value of the electronic address whatever its
scheme: `352862346` finds `0225:352862346` and `0002:352862346`, `0123456789`
finds `0208:0123456789`.

```sh
cargo run -- document search 352862346 [--role seller|buyer] [--direction inbound|outbound|both] \
    [--state VALID,DELIVERED] [--status REJECTED] [--from 2026-09-01] [--to 2026-10-01] [--limit 50]
```

When nothing matches, it suggests the `document sync` command to run.

### Entity search

`entity search` asks Iopole, live, which of the operator's business entities
have the given identifier value (SIREN, SIRET, BCE...), and shows their name,
company UID, legal identifier, country and network registrations. It tells
when the company is not registered.

### French directory

`directory french` searches the French e-invoicing directory (annuaire), live,
for any company, client of the operator or not. It takes a SIREN by default, or
other criteria, combined when several are given:

```sh
cargo run -- directory french 352862346
cargo run -- directory french --name OBAT [--siret 35286234600012] [--type legal-unit|office] \
    [--limit 20] [--offset 0] [--json]
```

`--name` is case insensitive, needs at least 3 characters and accepts the `*`
wildcard (`--name "BVS RENO*"`). Platform details are requested
(`withPlatformDetails`). Each entry shows:

```
Name:        MUCHA CONCIERGERIE
Type:        Office (PRIMARY)
Legal unit:  MADAME FLORIA DJOMO-NDJILO (SIREN 944870435)
Address:     62 RUE DANIELLE CASANOVA, 93200 SAINT-DENIS

TYPE               IDENTIFIER           NETWORK
OFFICE_IDENTIFIER  0009:94487043500012  -
```

Entries are separated by a line of `=`. An identifier registered on several
networks takes one row per network. A column
empty on every row, such as `VALID FROM` when the directory omits that date,
is left out.

The directory does
not give the total number of matches: when a page is full, the command tells
which `--offset` shows the next one.

`--json` prints the response of Iopole as received, indented, instead of the
entries, e.g. to debug or to pipe into `jq`.

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
