mod cli;
mod config;
mod event_recorder;
mod ping;
mod units;

use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::event_recorder::EventManager;
pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
pub use cli::Cli;
use log::{error, info, trace};

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
        let thread_handle = start_ping_thread(target_id, target, &config);
        thread_handles.push(thread_handle);
    }

    // TODO Await threads
    for (i, x) in thread_handles.into_iter().enumerate() {
        match x.join() {
            Ok(_) => info!("Thread for {} shutdown successfully.", config.targets[i]),
            Err(e) => error!(
                "Error shutting down thread: {} Message{e:?}",
                config.targets[i]
            ),
        };
    }

    // TODO implement graceful shutdown (need to write what is pending to disk)
    Ok(())
}

fn start_ping_thread(
    target_id: event_recorder::TargetID,
    target: &Target,
    config: &Config,
) -> JoinHandle<()> {
    let default_timeout = config.default_timeout;
    let target: Target = (*target).clone();
    let time_between_pings = config.ping_repeat_freq.into();
    thread::spawn(move || loop {
        let response = ping(&target, &default_timeout);
        trace!("Response for {target} was {response:?}");
        thread::sleep(Duration::from_secs(time_between_pings))
    })
}
