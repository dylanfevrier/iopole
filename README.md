# iopole

Command-line tool for the Iopole platform.

## Usage

```sh
cargo run -- --help
cargo run -- auth login
cargo run -- auth logout
cargo run -- document sync --from 2026-09-01 [--to 2026-10-01] [--direction inbound|outbound|both] [--refresh]
cargo run -- document search 352862346
cargo run -- document stat [--direction inbound|outbound|both]
cargo run -- document status 01a10b05-e3fd-74c8-b9cd-ce8e4c012777
cargo run -- entity search 352862346
cargo run -- entity update-claim 980217632 --company-uid 6071797 [--direction inbound|outbound|both]
cargo run -- entity register 982740128 [--network domestic-fr|peppol-international|both]
cargo run -- directory french 352862346
cargo run -- mcp
```

### Auth

`auth login` asks for the Iopole client, secret and customer ID, and stores
them in clear in `~/.config/iopole/secrets.json`, readable only by its owner
(file `600`, directory `700`). The encryption key of the local database lives
in the same file. Every command refuses to read the file once other users can
read it.

`auth logout` removes the credentials, the local database and its key; the file
goes away with them.

### Document sync

`document sync` stores the general information of Iopole invoices (never payment
means) in an encrypted SQLite database whose key lives in the secrets file.

- Invoices are searched by ranges of their UUID v7 identifier, one day at a
  time; days holding more than the 1200 results a search returns are halved
  down to one minute.
- Each search also replays the last 10% of the previous window, so that no
  invoice falls between two searches. Invoices found again only get their
  status refreshed, and only when it is not older than the stored one.
- Ranges already synchronized are never searched again: running
  `document sync` twice only fills the gaps. The last hour stays open and is
  searched again by the next run.
- `--refresh` searches the whole period again, ranges already synchronized
  included, and rewrites every stored invoice that differs from what Iopole
  returns now: status, business data and raw JSON alike, even when the status
  looks older. It then also tells how many invoices were updated.
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
finds `0208:0123456789`. Without it, the search covers every company, and
`--role` is refused.

```sh
cargo run -- document search [352862346 [--role seller|buyer]] [--direction inbound|outbound|both] \
    [--state VALID,DELIVERED] [--status REJECTED] [--from 2026-09-01] [--to 2026-10-01] [--limit 50] \
    [--identifiers]
```

`--identifiers` shows the seller and the buyer as `352862346 - CCLS`, so that
the identifier can be passed to `entity search`.

When nothing matches, it suggests the `document sync` command to run.

`--json` prints the matching invoices as Iopole returned them when
synchronized, pretty-printed, as a list (`[]` when nothing matches).

### Document stat

`document stat` counts the invoices of the local database by state, each
detailed by last status, the most frequent first. `--direction` keeps one
invoice flow; it counts both by default:

```
STATE          STATUS       COUNT
DELIVERED      -             1000
-              RECEIVED       580
-              ISSUED         420
WRONG_ROUTING  -             3000
-              (no status)   3000

4000 invoices.
```

### Document status

`document status` asks Iopole, live, for the status history of an invoice,
given its UUID as shown by `document search`, most recent first:

```
ID:           01a10b05-e3fd-74c8-b9cd-ce8e4c012777
Seller:       Leroy Merlin France (SIREN 384560942)
Buyer:        BVS RENOVATION (SIREN 980217632)

DATE (UTC)           STATUS
2026-10-05 09:45:07  PAYMENT_RECEIVED (212)
```

- The history only holds the lifecycle statuses exchanged on the network. The
  statuses of the platform, such as `MADE_AVAILABLE` or `REJECTED`, are not in
  it: an invoice may show them in `document search` and have no history.
- Statuses name the parties generically (`VENDEUR`, `ACHETEUR`): the names come
  from the local database when the invoice is synchronized, the SIRENs from
  the statuses.
- Iopole answers an unknown UUID with an empty history, as for an invoice
  without status.
- `--json` prints the history as Iopole returns it, pretty-printed, without
  reading the local database.

### Entity search

`entity search` asks Iopole, live, which of the operator's business entities
have the given identifier value (SIREN, SIRET, BCE...). It shows them like
`directory french` does, with the company UID and the country, and the date
each address is available since on every network. It tells when the company is
not registered.

`--json` prints the matching entities as Iopole returns them, pretty-printed,
as a list (`[]` when nothing matches).

### Entity claim update

`entity update-claim` replaces, live, how the operator claims one of its
business entities. It finds the entity by its identifier value (SIREN, SIRET,
BCE...) as `entity search` does, and refuses when none or several match:

- `--company-uid` (required) is sent back by Iopole in the `X-Company-UID`
  header of the webhooks about this entity.
- `--direction` restricts the claim to one invoice flow; it covers both by
  default.

Iopole replaces the whole configuration: any other header, query or path
parameter set on the claim is removed. It needs the `admin` role.

### Entity registration

`entity register` publishes, live, an identifier of one of the operator's
business entities on the e-invoicing networks, so that other platforms route
invoices to it. It finds the entity by its identifier value as
`entity update-claim` does, and registers the identifier under the scheme Iopole
knows it by (`0002` for a SIREN). `--network` picks `domestic-fr` (the French
directory) or `peppol-international`; it registers on both by default, the
French directory first.

A network that already publishes the identifier is reported as
`already up to date` and the next one is still registered. Run
`entity search` afterwards to see the networks the identifier is available on.

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
Name:         MUCHA CONCIERGERIE
Type:         Office (PRIMARY)
Legal unit:   MADAME FLORIA DJOMO-NDJILO (SIREN 944870435)
Address:      62 RUE DANIELLE CASANOVA, 93200 SAINT-DENIS

TYPE               IDENTIFIER           NETWORK
OFFICE_IDENTIFIER  0009:94487043500012  -
```

Entries are separated by a line of `=`. An identifier registered on several
networks takes one row per network. A column
empty on every row, such as `AVAILABLE SINCE` when the directory omits that date,
is left out.

The directory does
not give the total number of matches: when a page is full, the command tells
which `--offset` shows the next one.

`--json` prints the response of Iopole as received, indented, instead of the
entries, e.g. to debug or to pipe into `jq`.

## MCP server

`iopole mcp` serves the local invoices to AI assistants as MCP tools over
stdio. It never calls Iopole: the invoices come from `document sync`.

| Tool | Does |
|---|---|
| `search_invoices` | lists invoices like `document search`, as JSON |
| `invoice_stats` | counts invoices by state and last status like `document stat` |

Both are read-only. After `cargo install --path .`:

```sh
# Claude Code
claude mcp add iopole -- iopole mcp
```

```toml
# Codex: ~/.codex/config.toml
[mcp_servers.iopole]
command = "iopole"
args = ["mcp"]
```

```json
// Claude Desktop: claude_desktop_config.json, with the absolute path
{ "mcpServers": { "iopole": { "command": "/Users/<you>/.cargo/bin/iopole", "args": ["mcp"] } } }
```

To try the tools by hand: `npx @modelcontextprotocol/inspector iopole mcp`.

## Installation

```sh
cargo install --path .
```

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
