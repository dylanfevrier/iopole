use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use jiff::Timestamp;

use crate::Result;
use crate::catalog::{self, InvoiceFilter, InvoiceRow, Page, Role};
use crate::cli::{DirectionFilter, RoleFilter, SearchArgs};
use crate::database::{self, DatabaseKeyStore};
use crate::iopole::UuidBound;

const NAME_WIDTH: usize = 30;
const HEADERS: [&str; 8] = [
    "DATE",
    "DIRECTION",
    "NUMBER",
    "SELLER",
    "BUYER",
    "AMOUNT",
    "STATE",
    "STATUS",
];

pub(crate) fn execute(
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    args: &SearchArgs,
    out: &mut impl Write,
) -> Result<()> {
    let filter = filter(args)?;
    let page = if database_path.exists() {
        catalog::find(&database::open(database_path, keys)?, &filter)?
    } else {
        Page::default()
    };
    if page.total == 0 {
        writeln!(out, "No invoice found.")?;
        writeln!(
            out,
            "Run `{}` to retrieve invoices from Iopole, then search again.",
            sync_suggestion(args)
        )?;
        return Ok(());
    }
    render(&page, out)
}

fn sync_suggestion(args: &SearchArgs) -> String {
    let mut command = match args.from {
        Some(from) => format!("iopole document sync --from {from}"),
        None => "iopole document sync --from <date>".to_owned(),
    };
    if let Some(to) = args.to {
        let _ = write!(command, " --to {to}");
    }
    match args.direction {
        DirectionFilter::Inbound => command.push_str(" --direction inbound"),
        DirectionFilter::Outbound => command.push_str(" --direction outbound"),
        DirectionFilter::Both => {}
    }
    command
}

fn filter(args: &SearchArgs) -> Result<InvoiceFilter> {
    Ok(InvoiceFilter {
        identifier_value: args.identifier_value.clone(),
        role: args.role.map(|role| match role {
            RoleFilter::Seller => Role::Seller,
            RoleFilter::Buyer => Role::Buyer,
        }),
        directions: args.direction.directions().to_vec(),
        states: args.states.clone(),
        statuses: args.statuses.clone(),
        from: args.from.map(UuidBound::try_from).transpose()?,
        to: args.to.map(UuidBound::try_from).transpose()?,
        limit: args.limit,
    })
}

fn render(page: &Page, out: &mut impl Write) -> Result<()> {
    let rows: Vec<[String; 8]> = page.invoices.iter().map(cells).collect();
    let mut widths = HEADERS.map(str::len);
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }

    write_row(out, &HEADERS.map(str::to_owned), &widths)?;
    for row in &rows {
        write_row(out, row, &widths)?;
    }

    if page.invoices.len() < page.total {
        writeln!(
            out,
            "\nShowing {} of {} invoices; use --limit to see more.",
            page.invoices.len(),
            page.total
        )?;
    } else {
        writeln!(out, "\n{} invoices found.", page.total)?;
    }
    Ok(())
}

fn cells(invoice: &InvoiceRow) -> [String; 8] {
    let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| "-".to_owned());
    [
        date(&invoice.created_at),
        invoice.direction.to_lowercase(),
        optional(&invoice.number),
        truncate(&optional(&invoice.seller_name)),
        truncate(&optional(&invoice.buyer_name)),
        optional(&invoice.amount),
        invoice.state.clone(),
        optional(&invoice.last_status_code),
    ]
}

fn write_row(out: &mut impl Write, cells: &[String; 8], widths: &[usize; 8]) -> Result<()> {
    let line: Vec<String> = cells
        .iter()
        .zip(widths)
        .zip(HEADERS)
        .map(|((cell, &width), header)| {
            if header == "AMOUNT" {
                format!("{cell:>width$}")
            } else {
                format!("{cell:<width$}")
            }
        })
        .collect();
    writeln!(out, "{}", line.join("  ").trim_end())?;
    Ok(())
}

fn date(created_at: &str) -> String {
    created_at.parse::<Timestamp>().map_or_else(
        |_| created_at.chars().take(10).collect(),
        |instant| instant.strftime("%Y-%m-%d").to_string(),
    )
}

fn truncate(name: &str) -> String {
    if name.chars().count() <= NAME_WIDTH {
        name.to_owned()
    } else {
        let kept: String = name.chars().take(NAME_WIDTH - 1).collect();
        format!("{kept}…")
    }
}

#[cfg(test)]
mod tests;
