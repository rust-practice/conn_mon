mod cli;
mod config;
mod event_recorder;
mod ping;
mod units;

use std::thread::JoinHandle;

use crate::event_recorder::EventManager;
pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
pub use cli::Cli;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load_from(&cli.get_config_path()).context("Failed to load config")?;

    // TODO Create channel and give receiver to manager and a sender to each thread created

    let mut event_manager = EventManager::new();

    let mut thread_handles = vec![];

    // TODO: Start up a thread for each host then await the threads
    // https://doc.rust-lang.org/book/ch16-02-message-passing.html
    for target in config.targets.iter() {
        let target_id = event_manager
            .register_target(target)
            .with_context(|| format!("Failed to register target: {target}"))?;
        let thread_handle = start_ping_thread(target_id, target);
        thread_handles.push(thread_handle);
    }

    // TODO Await threads

    // TODO implement graceful shutdown (need to write what is pending to disk)
    Ok(())
}

fn start_ping_thread(target_id: event_recorder::TargetID, target: &Target) -> JoinHandle<()> {
    todo!()
}
