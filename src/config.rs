use std::path::Path;

use crate::{Seconds, Target};

pub struct Config {
    /// Targets to ping
    pub targets: Vec<Target>,

    /// Default timeout to use if not specified for a target
    pub default_timeout: Seconds,

    /// Frequency to Repeat Pings
    pub ping_repeat_freq: Seconds, // TODO: Implement repeated pings
}

impl Config {
    pub fn load_from(get_config_path: &Path) -> anyhow::Result<Config> {
        todo!()
    }
}
