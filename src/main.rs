use clap::Parser;
use conn_mon::{run, Cli};
use env_logger::Builder;
use log::LevelFilter;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logging(LevelFilter::Debug)?;
    run(cli)?;
    Ok(())
}

fn init_logging(level: LevelFilter) -> anyhow::Result<()> {
    Builder::new().filter(None, level).try_init()?;
    Ok(())
}
