use clap::Parser;
use conn_mon::{run, Cli};
use env_logger::Builder;
use log::LevelFilter;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logging(cli.log_level.into())?;
    run(cli)?;
    Ok(())
}

fn init_logging(level: LevelFilter) -> anyhow::Result<()> {
    // TODO: Change to log to file
    Builder::new().filter(None, level).try_init()?;
    Ok(())
}
