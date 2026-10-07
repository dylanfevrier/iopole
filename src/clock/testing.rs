use super::*;

pub(crate) struct FixedClock(pub(crate) Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}
