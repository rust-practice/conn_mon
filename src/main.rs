use clap::Parser;
use conn_mon::{run, Cli};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run(cli)?;
    Ok(())
}
