use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, ValueEnum};
use log::LevelFilter;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(
    author,
    version,
    about,
    long_about = "A program to monitor the quality of a connection."
)]
pub struct Cli {
    /// Specify directory to use for loading configs and outputting log records
    ///
    /// If not specified uses current working directory
    #[arg(long = "dir", short = 'd', value_name = "PATH")]
    pub working_dir: Option<String>,

    /// Set logging level to use
    #[arg(long, short, value_enum, default_value_t = LogLevel::Warn)]
    pub log_level: LogLevel,
}

impl Cli {
    pub fn get_config_path(&self) -> PathBuf {
        PathBuf::from("config.json")
    }
    /// Changes the current working directory to path if one is given
    pub fn update_current_working_dir(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.working_dir {
            std::env::set_current_dir(path)
                .with_context(|| format!("failed to set current dir to: '{path}'"))?;
        }
        Ok(())
    }
}

/// Exists to provide better help messages variants copied from LevelFilter as
/// that's the type that is actually needed
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug, Default)]
pub enum LogLevel {
    /// Nothing emitted in this mode
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}
