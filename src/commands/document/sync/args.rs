use clap::Args as ClapArgs;
use jiff::Timestamp;

use super::super::{DirectionFilter, parse_instant};
use crate::clock::Clock;
use crate::iopole::UuidBound;
use crate::sync::TimeWindow;
use crate::{Error, Result};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Start of the period, included: a UTC date (2026-09-01) or an RFC 3339 instant
    #[arg(long, value_parser = parse_instant)]
    pub(crate) from: Timestamp,

    /// End of the period, excluded [default: now]
    #[arg(long, value_parser = parse_instant)]
    pub(crate) to: Option<Timestamp>,

    /// Invoice flow to synchronize
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub(crate) direction: DirectionFilter,
}

impl Args {
    pub(crate) fn period(&self, clock: &impl Clock) -> Result<TimeWindow> {
        let (from, to) = (self.from, self.to.unwrap_or_else(|| clock.now()));
        UuidBound::try_from(from)?;
        UuidBound::try_from(to)?;
        TimeWindow::new(from, to).ok_or(Error::EmptyPeriod { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;
    use crate::clock::testing::FixedClock;

    fn now() -> Result<FixedClock> {
        Ok(FixedClock("2026-10-07T12:00:00Z".parse()?))
    }

    #[test]
    fn syncs_both_directions_up_to_now_by_default() -> Result<()> {
        let args =
            parse_args::<Args>(&["--from", "2026-09-01"]).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(args.direction, DirectionFilter::Both);
        assert_eq!(
            args.period(&now()?)?,
            TimeWindow::new(
                "2026-09-01T00:00:00Z".parse()?,
                "2026-10-07T12:00:00Z".parse()?
            )
            .unwrap_or_else(|| panic!("valid period"))
        );
        Ok(())
    }

    #[test]
    fn refuses_a_period_ending_before_it_starts() -> Result<()> {
        let args = parse_args::<Args>(&["--from", "2026-09-02", "--to", "2026-09-01"])
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(
            args.period(&now()?),
            Err(Error::EmptyPeriod { .. })
        ));
        Ok(())
    }
}
