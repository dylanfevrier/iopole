use clap::{ArgGroup, Args as ClapArgs, ValueEnum};

use crate::iopole::{DirectoryEntryType, DirectoryQuery, MAX_PAGE_LIMIT};

#[derive(Debug, ClapArgs)]
#[command(group(
    ArgGroup::new("criteria")
        .args(["siren", "siret", "name"])
        .required(true)
        .multiple(true)
))]
pub(crate) struct Args {
    /// SIREN of the company (9 digits)
    #[arg(value_parser = parse_siren)]
    pub(crate) siren: Option<String>,

    /// SIRET of an office (14 digits)
    #[arg(long, value_parser = parse_siret)]
    pub(crate) siret: Option<String>,

    /// Corporate name, case insensitive; `*` is a wildcard (OBAT*)
    #[arg(long, value_parser = parse_name)]
    pub(crate) name: Option<String>,

    /// Only legal units or only offices
    #[arg(long = "type", value_enum)]
    pub(crate) entry_type: Option<EntryTypeFilter>,

    /// Maximum number of entries to show
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=i64::from(MAX_PAGE_LIMIT)))]
    pub(crate) limit: u32,

    /// Number of entries to skip
    #[arg(long, default_value_t = 0)]
    pub(crate) offset: u32,

    /// Print the raw JSON response of Iopole instead of the entries
    #[arg(long)]
    pub(crate) json: bool,
}

impl Args {
    pub(crate) fn query(&self) -> DirectoryQuery {
        DirectoryQuery {
            siren: self.siren.clone(),
            siret: self.siret.clone(),
            name: self.name.clone(),
            entry_type: self.entry_type.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum EntryTypeFilter {
    LegalUnit,
    Office,
}

impl From<EntryTypeFilter> for DirectoryEntryType {
    fn from(filter: EntryTypeFilter) -> Self {
        match filter {
            EntryTypeFilter::LegalUnit => Self::LegalUnit,
            EntryTypeFilter::Office => Self::Office,
        }
    }
}

fn parse_siren(value: &str) -> Result<String, String> {
    parse_digits(value, 9, "SIREN")
}

fn parse_siret(value: &str) -> Result<String, String> {
    parse_digits(value, 14, "SIRET")
}

fn parse_digits(value: &str, length: usize, label: &str) -> Result<String, String> {
    let digits: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() == length && digits.chars().all(|c| c.is_ascii_digit()) {
        Ok(digits)
    } else {
        Err(format!("a {label} has exactly {length} digits"))
    }
}

fn parse_name(value: &str) -> Result<String, String> {
    let name = value.trim();
    if name.chars().filter(|&c| c != '*').count() < 3 {
        Err("at least 3 characters are required, wildcards aside".to_owned())
    } else {
        Ok(name.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    #[test]
    fn takes_a_siren_by_default() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&["123 456 789"])?;

        assert_eq!(args.siren.as_deref(), Some("123456789"));
        assert_eq!(args.name, None);
        assert_eq!(args.entry_type, None);
        assert_eq!((args.offset, args.limit), (0, 20));
        assert!(!args.json);
        Ok(())
    }

    #[test]
    fn combines_name_siret_and_type_into_one_query() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[
            "--name",
            "OBAT*",
            "--siret",
            "00735010100083",
            "--type",
            "office",
        ])?;

        assert_eq!(
            args.query(),
            DirectoryQuery {
                siren: None,
                siret: Some("00735010100083".to_owned()),
                name: Some("OBAT*".to_owned()),
                entry_type: Some(DirectoryEntryType::Office),
            }
        );
        Ok(())
    }
}
