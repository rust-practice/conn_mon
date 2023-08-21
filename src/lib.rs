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

use crate::event_recorder::ResponseManager;
pub(crate) use crate::{
    config::Config,
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
use event_recorder::{ResponseMessage, TargetID};
use log::debug;

pub use crate::{cli::Cli, event_recorder::TimestampedResponse};

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let config = Config::load_from(&cli.get_config_path()).context("Failed to load config")?;

    let (tx, rx) = mpsc::channel();
    let mut response_manager =
        ResponseManager::new(rx, &config).context("Failed to start response manager")?;

    // Start up a thread for each host then await the threads
    for target in config.targets.iter() {
        let target_id = response_manager
            .register_target(target)
            .with_context(|| format!("Failed to register target: {target}"))?;
        start_ping_thread(target_id, target, tx.clone(), &config)?;
    }
    drop(tx); // Drop last handle that is not used

    response_manager.start_receive_loop();

    unreachable!("Should block on receive loop")
}

fn start_ping_thread(
    target_id: TargetID,
    target: &Target,
    tx: Sender<ResponseMessage>,
    config: &Config,
) -> anyhow::Result<JoinHandle<()>> {
    let default_timeout = config.default_timeout;
    let target: Target = (*target).clone();
    let time_between_pings = config.ping_repeat_freq.into();
    let result = thread::Builder::new()
        .name(format!("{target}"))
        .spawn(move || loop {
            let response = ping(&target, &default_timeout);
            debug!("Response for {target} was {response:?}");
            tx.send(ResponseMessage::new(target_id, response))
                .expect("Failed to send response update");
            thread::sleep(Duration::from_secs(time_between_pings));
        })
        .context("Failed to start thread")?;
    Ok(result)
}
