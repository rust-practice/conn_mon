use std::{fs, path::Path};

use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};

use crate::{Seconds, Target};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Targets to ping
    pub targets: Vec<Target>,

    /// Default timeout to use if not specified for a target
    #[serde(default = "Config::default_timeout")]
    pub default_timeout: Seconds,

    /// Frequency to Repeat Pings
    #[serde(default = "Config::default_ping_repeat_freq")]
    pub ping_repeat_freq: Seconds,

    /// Minimum time between writing to the same file on disk
    #[serde(default = "Config::default_min_time_between_write")]
    pub min_time_between_write: Seconds,

    /// Frequency at which reminders are sent
    #[serde(default = "Config::default_notify_remind_interval")]
    pub notify_remind_interval: Seconds,

    /// Minimum time before sending the first notification that a host went down
    #[serde(default = "Config::default_min_time_before_first_down_notification")]
    pub min_time_before_first_down_notification: Seconds,
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

    fn default_min_time_between_write() -> Seconds {
        300.into()
    }

    fn default_notify_remind_interval() -> Seconds {
        3600.into()
    }

    fn default_min_time_before_first_down_notification() -> Seconds {
        30.into()
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    /// Ensure sample files are valid json
    #[rstest]
    #[case("sample_config_full/config.json")]
    #[case("sample_config_minimal/config.json")]
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
