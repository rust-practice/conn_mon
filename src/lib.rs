mod cli;
mod config;
mod ping;
mod units;

pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
pub use cli::Cli;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load_from(&cli.get_config_path()).context("Failed to load config")?;

    for target in config.targets.iter() {
        dbg!(target);
        let result = ping(target, &config.default_timeout);
        let _ = dbg!(result);
        println!("----------------------------------------------");
    }
    println!("Completed");
    Ok(())
}
