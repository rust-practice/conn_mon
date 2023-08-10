mod cli;
mod ping;
mod units;

pub use cli::Cli;
use ping::ping;
pub(crate) use units::{Milliseconds, Seconds};

use crate::ping::Target;

pub fn run(_cli: Cli) -> anyhow::Result<()> {
    let targets = vec![
        Target::new("127.0.0.1".to_string()),
        Target::new("8.8.8.8".to_string()),
        Target::new("192.168.1.205".to_string()),
        Target::new("192.168.8.8".to_string()),
    ];

    for target in targets {
        let result = ping(&target);
        let _ = dbg!(result);
        println!("-----");
    }
    println!("Completed");
    Ok(())
}
