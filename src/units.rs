use std::fmt::Display;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Milliseconds(u16);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
pub struct Seconds(u8);
impl Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for Milliseconds {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<u8> for Seconds {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Seconds> for u64 {
    fn from(value: Seconds) -> Self {
        value.0 as u64
    }
}

impl TryFrom<(&str, &str)> for Milliseconds {
    type Error = anyhow::Error;

    fn try_from((ms, ms_frac): (&str, &str)) -> Result<Self, Self::Error> {
        let mut ms: u16 = ms.parse().context("Failed to parse ms in ping")?;
        let ms_frac: u16 = ms_frac.parse().context("Failed to parse ms_frac in ping")?;
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
