use std::io::Write;

use indicatif::ProgressStyle;
use jiff::SignedDuration;

use crate::Result;
use crate::invoice::Direction;
use crate::sync::Progress;

pub(super) struct ProgressBar(indicatif::ProgressBar);

impl ProgressBar {
    pub(super) fn new(total: SignedDuration) -> Self {
        let style = ProgressStyle::with_template("{bar:40} {percent:>3}%")
            .unwrap_or_else(|_| ProgressStyle::default_bar());
        Self(indicatif::ProgressBar::new(millis(total)).with_style(style))
    }

    pub(super) fn finish(&self) {
        self.0.finish();
    }
}

impl Progress for ProgressBar {
    fn advance(&mut self, synchronized: SignedDuration) {
        self.0.inc(millis(synchronized));
    }
}

fn millis(duration: SignedDuration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(0)
}

pub(super) fn report(
    out: &mut impl Write,
    retrieved: &[(Direction, usize)],
    updated: Option<usize>,
    truncated_windows: usize,
) -> Result<()> {
    let total: usize = retrieved.iter().map(|(_, inserted)| inserted).sum();
    write!(out, "{total} invoices retrieved.")?;
    for (direction, inserted) in retrieved {
        write!(out, " {inserted} {}.", direction.as_str().to_lowercase())?;
    }
    writeln!(out)?;
    if let Some(updated) = updated {
        writeln!(out, "{updated} invoices updated.")?;
    }
    if truncated_windows > 0 {
        writeln!(
            out,
            "warning: {truncated_windows} one-minute windows hold more invoices than Iopole returns and may be incomplete"
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reported(retrieved: &[(Direction, usize)], truncated_windows: usize) -> String {
        let mut out = Vec::new();
        report(&mut out, retrieved, None, truncated_windows)
            .unwrap_or_else(|err| panic!("cannot render: {err}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn counts_retrieved_invoices_per_direction() {
        assert_eq!(
            reported(&[(Direction::Inbound, 2), (Direction::Outbound, 1)], 0),
            "3 invoices retrieved. 2 inbound. 1 outbound.\n"
        );
    }

    #[test]
    fn warns_about_windows_that_may_be_incomplete() {
        assert_eq!(
            reported(&[(Direction::Inbound, 1200)], 2),
            "1200 invoices retrieved. 1200 inbound.\nwarning: 2 one-minute windows hold more invoices than Iopole returns and may be incomplete\n"
        );
    }
}
