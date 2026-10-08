use clap::Args as ClapArgs;

use crate::cli::parse_identifier_value;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Identifier value of the business entity (SIREN, SIRET, BCE...), without its scheme
    #[arg(value_parser = parse_identifier_value)]
    pub(crate) identifier_value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    #[test]
    fn takes_an_identifier_value_without_its_scheme() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&["0114445670"])?;

        assert_eq!(args.identifier_value, "0114445670");
        assert!(parse_args::<Args>(&["0208:0114445670"]).is_err());
        Ok(())
    }
}
