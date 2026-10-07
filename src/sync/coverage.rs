use super::window::TimeWindow;

pub(crate) fn gaps(requested: TimeWindow, covered: &[TimeWindow]) -> Vec<TimeWindow> {
    covered.iter().fold(vec![requested], |remaining, &done| {
        remaining
            .into_iter()
            .flat_map(|gap| gap.subtract(done))
            .collect()
    })
}

#[cfg(test)]
mod tests;
