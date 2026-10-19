//! `iopole mcp`: the commands as tools for AI assistants, over stdio.

use std::path::PathBuf;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{Json, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::catalog::{InvoiceRow, Page};
use crate::cli::parse_identifier_value;
use crate::commands::document::search::{self, RoleFilter, parse_code};
use crate::commands::document::stat::{self, StateSummary};
use crate::commands::document::{DirectionFilter, parse_instant};
use crate::secrets_file::SecretsFile;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
const SYNC_HINT: &str = "Only synchronized invoices are known: the user runs `iopole document sync --from <date>` to retrieve them from Iopole.";

const INSTRUCTIONS: &str = "\
Iopole is the e-invoicing platform (French PDP) through which the operator exchanges invoices.
These tools read the local copy of its invoices, synchronized by the user with `iopole document sync`; they never call Iopole.
An invoice has a state, its processing state on the platform (VALID, DELIVERED, WRONG_ROUTING...), and a last status, \
its lifecycle status exchanged on the network (RECEIVED, APPROVED, REJECTED...), which may be missing.
Companies are designated by the value of their electronic address without its scheme: a SIREN (9 digits), a SIRET or a BCE number.";

pub(crate) fn serve(database_path: PathBuf, secrets_path: PathBuf) -> crate::Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let server = IopoleServer::new(database_path, secrets_path)
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|err| Error::Mcp(err.to_string()))?;
        server
            .waiting()
            .await
            .map_err(|err| Error::Mcp(err.to_string()))?;
        Ok(())
    })
}

#[derive(Debug, Clone)]
pub(crate) struct IopoleServer {
    database_path: PathBuf,
    secrets_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct SearchInvoicesInput {
    /// Value of the electronic address of the seller or the buyer, without its
    /// scheme: a SIREN, a SIRET or a BCE number. Every company when omitted.
    #[serde(default)]
    identifier: Option<String>,
    /// Only invoices where the company has this role; needs `identifier`.
    #[serde(default)]
    role: Option<RoleFilter>,
    /// Invoice flow: inbound (received), outbound (issued) or both (default).
    #[serde(default)]
    direction: Option<DirectionFilter>,
    /// Only these processing states, e.g. `DELIVERED` and `WRONG_ROUTING`.
    #[serde(default)]
    states: Vec<String>,
    /// Only these last lifecycle statuses, e.g. `REJECTED`.
    #[serde(default)]
    statuses: Vec<String>,
    /// Created at or after: a UTC date (2026-09-01) or an RFC 3339 instant.
    #[serde(default)]
    from: Option<String>,
    /// Created before: a UTC date or an RFC 3339 instant.
    #[serde(default)]
    to: Option<String>,
    /// Maximum number of invoices returned, most recent first: 50 by default, 200 at most.
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct SearchInvoicesOutput {
    /// Invoices matching the filters, beyond those returned.
    total: usize,
    invoices: Vec<InvoiceSummary>,
    /// What to do next, when the result is empty or cut by `limit`.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct InvoiceSummary {
    /// Iopole UUID of the invoice.
    id: String,
    /// Creation instant on Iopole.
    created_at: String,
    /// INBOUND (received) or OUTBOUND (issued).
    direction: String,
    /// Number given by the issuer.
    number: Option<String>,
    /// Total amount, as a decimal number with a dot.
    amount: Option<String>,
    seller: PartySummary,
    buyer: PartySummary,
    /// Processing state on the platform.
    state: String,
    /// Last lifecycle status exchanged on the network, if any.
    last_status: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct PartySummary {
    /// SIREN, SIRET or BCE number.
    identifier: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub(crate) struct InvoiceStatsInput {
    /// Invoice flow: inbound (received), outbound (issued) or both (default).
    #[serde(default)]
    direction: Option<DirectionFilter>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct InvoiceStatsOutput {
    /// Every synchronized invoice of the direction.
    total: usize,
    /// States in alphabetical order, each detailed by last status.
    states: Vec<StateSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[tool_router(router = tool_router)]
impl IopoleServer {
    pub(crate) fn new(database_path: PathBuf, secrets_path: PathBuf) -> Self {
        Self {
            database_path,
            secrets_path,
            tool_router: Self::tool_router(),
        }
    }

    /// Search synchronized invoices.
    #[tool(
        name = "search_invoices",
        description = "Search the synchronized invoices, of one company or of every company, most recent first. \
Use it to list or find invoices; use invoice_stats to count them.",
        annotations(
            title = "Search invoices",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub(crate) async fn search_invoices(
        &self,
        Parameters(input): Parameters<SearchInvoicesInput>,
    ) -> Result<Json<SearchInvoicesOutput>, String> {
        let (database_path, secrets_path) = (self.database_path.clone(), self.secrets_path.clone());
        blocking(move || {
            let filter = search_args(input)?
                .filter()
                .map_err(|err| err.to_string())?;
            let page = search::find(&database_path, &SecretsFile::at(secrets_path), &filter)
                .map_err(|err| err.to_string())?;
            Ok(search_output(page))
        })
        .await
        .map(Json)
    }

    /// Count synchronized invoices by state and status.
    #[tool(
        name = "invoice_stats",
        description = "Count the synchronized invoices by processing state, each detailed by last lifecycle status. \
Use it for totals and distributions rather than listing invoices.",
        annotations(
            title = "Invoice statistics",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub(crate) async fn invoice_stats(
        &self,
        Parameters(input): Parameters<InvoiceStatsInput>,
    ) -> Result<Json<InvoiceStatsOutput>, String> {
        let (database_path, secrets_path) = (self.database_path.clone(), self.secrets_path.clone());
        let directions = input
            .direction
            .unwrap_or(DirectionFilter::Both)
            .directions();
        blocking(move || {
            let states =
                stat::summarize(&database_path, &SecretsFile::at(secrets_path), directions)
                    .map_err(|err| err.to_string())?;
            let total = states.iter().map(|state| state.count).sum();
            Ok(InvoiceStatsOutput {
                total,
                note: (total == 0).then(|| format!("No synchronized invoice. {SYNC_HINT}")),
                states,
            })
        })
        .await
        .map(Json)
    }
}

// The macro writes `async` handlers that only forward to the router.
#[allow(clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for IopoleServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("iopole", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

/// The database and the secrets file are read synchronously, out of the
/// threads serving the protocol.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| format!("internal failure: {err}"))?
}

/// The same validation as `iopole document search`.
fn search_args(input: SearchInvoicesInput) -> Result<search::Args, String> {
    let identifier_value = input
        .identifier
        .as_deref()
        .map(parse_identifier_value)
        .transpose()?;
    if input.role.is_some() && identifier_value.is_none() {
        return Err("`role` needs an `identifier`".to_owned());
    }
    let instant = |value: Option<String>| value.as_deref().map(parse_instant).transpose();
    let codes = |values: Vec<String>| {
        values
            .iter()
            .map(|value| parse_code(value))
            .collect::<Result<Vec<_>, _>>()
    };
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(format!("`limit` must be between 1 and {MAX_LIMIT}"));
    }
    Ok(search::Args {
        identifier_value,
        role: input.role,
        direction: input.direction.unwrap_or(DirectionFilter::Both),
        states: codes(input.states)?,
        statuses: codes(input.statuses)?,
        from: instant(input.from)?,
        to: instant(input.to)?,
        limit,
        json: false,
        identifiers: false,
    })
}

fn search_output(page: Page) -> SearchInvoicesOutput {
    let note = if page.total == 0 {
        Some(format!("No invoice matches. {SYNC_HINT}"))
    } else if page.invoices.len() < page.total {
        Some(format!(
            "Only the {} most recent of {} invoices are returned: raise `limit` (at most {MAX_LIMIT}) or narrow the filters.",
            page.invoices.len(),
            page.total
        ))
    } else {
        None
    };
    SearchInvoicesOutput {
        total: page.total,
        invoices: page.invoices.into_iter().map(summary).collect(),
        note,
    }
}

fn summary(invoice: InvoiceRow) -> InvoiceSummary {
    InvoiceSummary {
        id: invoice.id,
        created_at: invoice.created_at,
        direction: invoice.direction,
        number: invoice.number,
        amount: invoice.amount,
        seller: PartySummary {
            identifier: invoice.seller_identifier,
            name: invoice.seller_name,
        },
        buyer: PartySummary {
            identifier: invoice.buyer_identifier,
            name: invoice.buyer_name,
        },
        state: invoice.state,
        last_status: invoice.last_status_code,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::database::{self, testing::InvoiceRecord};

    struct Fixture {
        tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Result<Self, Error> {
            Ok(Self {
                tmp: tempfile::tempdir()?,
            })
        }

        fn database_path(&self) -> PathBuf {
            self.tmp.path().join("data").join("iopole.db")
        }

        fn secrets_path(&self) -> PathBuf {
            self.tmp.path().join("config").join("secrets.json")
        }

        fn with(records: &[InvoiceRecord<'_>]) -> Result<Self, Error> {
            let fixture = Self::new()?;
            let connection = database::open(
                &fixture.database_path(),
                &SecretsFile::at(fixture.secrets_path()),
            )?;
            for record in records {
                record.insert(&connection)?;
            }
            Ok(fixture)
        }

        fn server(&self) -> IopoleServer {
            IopoleServer::new(self.database_path(), self.secrets_path())
        }
    }

    fn records() -> [InvoiceRecord<'static>; 3] {
        [
            InvoiceRecord {
                id: "01a1",
                seller_identifier: Some("352862346"),
                seller_name: Some("CCLS"),
                state: "DELIVERED",
                last_status_code: Some("RECEIVED"),
                amount: Some("1250.5"),
                ..InvoiceRecord::default()
            },
            InvoiceRecord {
                id: "01a2",
                direction: "OUTBOUND",
                buyer_identifier: Some("352862346"),
                state: "DELIVERED",
                ..InvoiceRecord::default()
            },
            InvoiceRecord {
                id: "01a3",
                seller_identifier: Some("517657334"),
                state: "WRONG_ROUTING",
                ..InvoiceRecord::default()
            },
        ]
    }

    async fn search(
        fixture: &Fixture,
        input: SearchInvoicesInput,
    ) -> Result<SearchInvoicesOutput, String> {
        let Json(output) = fixture.server().search_invoices(Parameters(input)).await?;
        Ok(output)
    }

    fn ids(output: &SearchInvoicesOutput) -> Vec<&str> {
        output
            .invoices
            .iter()
            .map(|invoice| invoice.id.as_str())
            .collect()
    }

    #[test]
    fn exposes_two_read_only_local_tools_with_output_schemas() {
        let server = IopoleServer::new(PathBuf::new(), PathBuf::new());
        let mut tools = server.tool_router.list_all();
        tools.sort_by(|a, b| a.name.cmp(&b.name));

        let names: Vec<_> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(names, ["invoice_stats", "search_invoices"]);
        for tool in &tools {
            let annotations = tool.annotations.as_ref();
            assert_eq!(annotations.and_then(|a| a.read_only_hint), Some(true));
            assert_eq!(annotations.and_then(|a| a.open_world_hint), Some(false));
            assert!(
                tool.output_schema.is_some(),
                "{} has no output schema",
                tool.name
            );
        }
    }

    #[tokio::test]
    async fn searches_the_invoices_of_a_company_with_their_parties() -> Result<(), Error> {
        let fixture = Fixture::with(&records())?;

        let output = search(
            &fixture,
            SearchInvoicesInput {
                identifier: Some("352862346".to_owned()),
                ..SearchInvoicesInput::default()
            },
        )
        .await
        .map_err(Error::Mcp)?;

        assert_eq!(ids(&output), ["01a2", "01a1"]);
        assert_eq!(output.total, 2);
        assert!(output.note.is_none());
        let first = &output.invoices[1];
        assert_eq!(first.seller.identifier.as_deref(), Some("352862346"));
        assert_eq!(first.seller.name.as_deref(), Some("CCLS"));
        assert_eq!(first.amount.as_deref(), Some("1250.5"));
        assert_eq!(first.last_status.as_deref(), Some("RECEIVED"));
        Ok(())
    }

    #[tokio::test]
    async fn applies_the_filters_of_the_command() -> Result<(), Error> {
        let fixture = Fixture::with(&records())?;

        let output = search(
            &fixture,
            SearchInvoicesInput {
                states: vec!["wrong_routing".to_owned()],
                direction: Some(DirectionFilter::Inbound),
                ..SearchInvoicesInput::default()
            },
        )
        .await
        .map_err(Error::Mcp)?;

        assert_eq!(ids(&output), ["01a3"]);
        Ok(())
    }

    #[tokio::test]
    async fn tells_when_the_limit_cuts_the_result() -> Result<(), Error> {
        let fixture = Fixture::with(&records())?;

        let output = search(
            &fixture,
            SearchInvoicesInput {
                limit: Some(1),
                ..SearchInvoicesInput::default()
            },
        )
        .await
        .map_err(Error::Mcp)?;

        assert_eq!((output.invoices.len(), output.total), (1, 3));
        assert!(
            output
                .note
                .is_some_and(|note| note.contains("raise `limit`"))
        );
        Ok(())
    }

    #[tokio::test]
    async fn rejects_invalid_input_with_a_reason() -> Result<(), Error> {
        let fixture = Fixture::new()?;
        let reason = |input| async { search(&fixture, input).await.err().unwrap_or_default() };

        assert!(
            reason(SearchInvoicesInput {
                role: Some(RoleFilter::Seller),
                ..SearchInvoicesInput::default()
            })
            .await
            .contains("needs an `identifier`")
        );
        assert!(
            reason(SearchInvoicesInput {
                limit: Some(500),
                ..SearchInvoicesInput::default()
            })
            .await
            .contains("between 1 and 200")
        );
        assert!(
            reason(SearchInvoicesInput {
                from: Some("01/09/2026".to_owned()),
                ..SearchInvoicesInput::default()
            })
            .await
            .contains("neither a date")
        );
        assert!(
            reason(SearchInvoicesInput {
                identifier: Some("0225:352862346".to_owned()),
                ..SearchInvoicesInput::default()
            })
            .await
            .contains("scheme")
        );
        Ok(())
    }

    #[tokio::test]
    async fn counts_by_state_and_status() -> Result<(), Error> {
        let fixture = Fixture::with(&records())?;

        let Json(output) = fixture
            .server()
            .invoice_stats(Parameters(InvoiceStatsInput::default()))
            .await
            .map_err(Error::Mcp)?;

        assert_eq!(output.total, 3);
        let states: Vec<_> = output
            .states
            .iter()
            .map(|state| (state.state.as_str(), state.count))
            .collect();
        assert_eq!(states, [("DELIVERED", 2), ("WRONG_ROUTING", 1)]);
        Ok(())
    }

    #[tokio::test]
    async fn points_to_the_sync_without_database_and_creates_none() -> Result<(), Error> {
        let fixture = Fixture::new()?;

        let Json(stats) = fixture
            .server()
            .invoice_stats(Parameters(InvoiceStatsInput::default()))
            .await
            .map_err(Error::Mcp)?;
        let found = search(&fixture, SearchInvoicesInput::default())
            .await
            .map_err(Error::Mcp)?;

        for note in [stats.note, found.note] {
            assert!(note.is_some_and(|note| note.contains("iopole document sync")));
        }
        assert!(!fixture.database_path().exists());
        assert!(!Path::new(&fixture.secrets_path()).exists());
        Ok(())
    }
}
