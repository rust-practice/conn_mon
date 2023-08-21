mod cli;
mod config;
mod event_recorder;
mod ping;
mod state_management;
mod units;

use std::{
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::event_recorder::EventSubscriber;
pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
pub use cli::Cli;
use event_recorder::{EventMessage, TargetID};
use log::trace;
use state_management::EventPublisher;

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load_from(&cli.get_config_path()).context("Failed to load config")?;

    let (tx, rx) = mpsc::channel();
    let mut event_manager = EventSubscriber::new(rx);

    // Start up a thread for each host then await the threads
    for target in config.targets.iter() {
        let target_id = event_manager
            .register_target(target)
            .with_context(|| format!("Failed to register target: {target}"))?;
        start_ping_thread(target_id, target, tx.clone(), &config)?;
    }

    event_manager.start_receive_loop();

    unreachable!("Should block on receive loop")
}

fn start_ping_thread(
    target_id: TargetID,
    target: &Target,
    tx: Sender<EventMessage>,
    config: &Config,
) -> anyhow::Result<JoinHandle<()>> {
    let default_timeout = config.default_timeout;
    let target: Target = (*target).clone();
    let time_between_pings = config.ping_repeat_freq.into();
    let result = thread::Builder::new()
        .name(format!("{target}"))
        .spawn(move || {
            let mut publisher = EventPublisher::new(target_id, tx);
            loop {
                let response = ping(&target, &default_timeout);
                trace!("Response for {target} was {response:?}");
                publisher.process_response(response);
                thread::sleep(Duration::from_secs(time_between_pings))
            }
        })
        .context("Failed to start thread")?;
    Ok(result)
}
