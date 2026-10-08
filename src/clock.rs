use jiff::Timestamp;

pub(crate) trait Clock {
    fn now(&self) -> Timestamp;
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub(crate) struct FixedClock(pub(crate) Timestamp);

    impl Clock for FixedClock {
        fn now(&self) -> Timestamp {
            self.0
        }
    }
}
