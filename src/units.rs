use std::fmt::Display;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Milliseconds(u64);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct Seconds(u64);

impl Seconds {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
    pub(crate) fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Milliseconds {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut seconds = self.as_u64();
        let seconds_per_minute = 60;
        let seconds_per_hour = seconds_per_minute * 60;
        let seconds_per_day = seconds_per_hour * 24;
        let days = seconds / seconds_per_day;
        seconds -= days * seconds_per_day;
        let hours = seconds / seconds_per_hour;
        seconds -= hours * seconds_per_hour;
        let minutes = seconds / seconds_per_minute;
        seconds -= minutes * seconds_per_minute;
        write!(f, "{days} days {hours:0>2}:{minutes:0>2}:{seconds:0>2}")
    }
}

impl From<Seconds> for std::time::Duration {
    fn from(value: Seconds) -> Self {
        Self::from_secs(value.into())
    }
}

impl From<u64> for Milliseconds {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<Seconds> for u64 {
    fn from(value: Seconds) -> Self {
        value.0
    }
}

impl From<u64> for Seconds {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl TryFrom<(&str, &str)> for Milliseconds {
    type Error = anyhow::Error;

    fn try_from((ms, ms_frac): (&str, &str)) -> Result<Self, Self::Error> {
        let mut ms: u64 = ms.parse().context("failed to parse ms in ping")?;
        let ms_frac: u64 = ms_frac.parse().context("failed to parse ms_frac in ping")?;
        if ms_frac >= 50 {
            ms += 1;
        }
        Ok(Self(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("8", "00", Milliseconds(8))]
    #[case("8", "49", Milliseconds(8))]
    #[case("8", "50", Milliseconds(9))]
    #[case("8", "99", Milliseconds(9))]
    fn milliseconds(#[case] ms: &str, #[case] ms_frac: &str, #[case] expected: Milliseconds) {
        let actual = Milliseconds::try_from((ms, ms_frac)).unwrap();
        assert_eq!(actual, expected);
    }
}
