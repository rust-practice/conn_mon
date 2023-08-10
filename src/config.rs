use std::{fs, path::Path};

use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{Seconds, Target};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Targets to ping
    pub targets: Vec<Target>,

    /// Default timeout to use if not specified for a target
    #[serde(default = "Config::default_timeout")]
    pub default_timeout: Seconds,

    /// Frequency to Repeat Pings
    #[serde(default = "Config::default_ping_repeat_freq")]
    pub ping_repeat_freq: Seconds, // TODO: Implement repeated pings
}

impl Config {
    pub fn load_from(config_path: &Path) -> anyhow::Result<Config> {
        debug!("Loading Config from: {config_path:?}");
        let file_contents = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read contents of {config_path:?}"))?;
        let result = serde_json::from_str(&file_contents)
            .with_context(|| format!("Failed to parse contents of {config_path:?}"))?;
        Ok(result)
    }

    fn default_timeout() -> Seconds {
        3.into()
    }

    fn default_ping_repeat_freq() -> Seconds {
        5.into()
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    /// Ensure sample files are valid json
    #[rstest]
    #[case("sample_config_full.json")]
    #[case("sample_config_minimal.json")]
    fn load_sample_config(#[case] filename: &str) {
        // Arrange
        let path = Path::new(filename);

        // Act
        let actual = Config::load_from(path);

        // Assert
        assert!(
            actual.is_ok(),
            "Failed to load a sample config file: {path:?} because {:#?}",
            actual.unwrap_err()
        );
    }
}
