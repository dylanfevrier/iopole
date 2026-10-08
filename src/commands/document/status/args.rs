use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Iopole identifier of the invoice, as shown by `document search`
    #[arg(value_parser = parse_uuid)]
    pub(crate) invoice_id: String,
}

fn parse_uuid(value: &str) -> Result<String, String> {
    let id = value.trim().to_lowercase();
    let groups: Vec<&str> = id.split('-').collect();
    let well_formed = groups.iter().map(|group| group.len()).eq([8, 4, 4, 4, 12])
        && groups
            .iter()
            .all(|group| group.chars().all(|c| c.is_ascii_hexdigit()));
    if well_formed {
        Ok(id)
    } else {
        Err(format!("`{value}` is not a UUID"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    #[test]
    fn takes_the_uuid_shown_by_document_search() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&["01A11587-E1D8-712D-8951-ED5072005067"])?;

        assert_eq!(args.invoice_id, "01a11587-e1d8-712d-8951-ed5072005067");
        Ok(())
    }

    #[test]
    fn rejects_invoice_numbers() {
        assert!(parse_args::<Args>(&["260902440"]).is_err());
        assert!(parse_args::<Args>(&["01a11587-e1d8-712d-8951-ed507200506z"]).is_err());
    }
}
