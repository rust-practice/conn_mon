use std::{fs, path::Path};

use anyhow::Context;
use log::debug;
use serde::Deserialize;

use crate::{Seconds, Target};

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Targets to ping
    pub targets: Vec<Target>,

    /// Default timeout to use if not specified for a target
    pub default_timeout: Seconds,

    /// Frequency to Repeat Pings
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
}
