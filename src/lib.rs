mod cli;
mod config;
mod ping;
mod units;

pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
pub use cli::Cli;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load_from(&cli.get_config_path())?;

    for target in config.targets.iter() {
        let result = ping(target);
        let _ = dbg!(result);
        println!("-----");
    }
    println!("Completed");
    Ok(())
}