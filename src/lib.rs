// TODO: Add option to set timeout per host
// TODO: Add timestamp to discord messages
mod cli;
mod config;
mod event_recorder;
mod logging;
mod notification;
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
    notification::{discord::Discord, email::Email},
    ping::{ping, Target},
    units::{Milliseconds, Seconds},
};
use anyhow::Context;
use event_recorder::{ResponseMessage, TargetID};
use log::{debug, warn};

pub use crate::{cli::Cli, event_recorder::TimestampedResponse};

pub fn run(cli: Cli) -> anyhow::Result<()> {
    cli.update_current_working_dir()
        .context("failed to update current working directory")?;
    logging::init_logging(cli.log_level.into())?;
    warn!(
        "Starting up in dir: {:?}",
        std::env::current_dir()
            .context("failed to get cwd")?
            .display()
    );
    let config = Config::load_from(&cli.get_config_path()).context("failed to load config")?;

    let (tx, rx) = mpsc::channel();
    let mut response_manager =
        ResponseManager::new(rx, &config).context("failed to start response manager")?;

    // Start up a thread for each host then await the threads
    for target in config.targets.iter().filter(|t| !t.disabled) {
        let target_id = response_manager
            .register_target(target)
            .with_context(|| format!("failed to register target: {target}"))?;
        start_ping_thread(target_id, target, tx.clone(), &config)?;
    }
    drop(tx); // Drop last handle that is not used

    response_manager
        .log_events_output_folder()
        .context("failed to log output folder")?;
    response_manager.start_keep_alive()?;
    response_manager.start_receive_loop();

    unreachable!("Should block on receive loop")
    // TODO Add graceful shutdown https://rust-cli.github.io/book/in-depth/signals.html (See zero to prod)
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
                .expect("failed to send response update");
            thread::sleep(Duration::from_secs(time_between_pings));
        })
        .context("failed to start thread")?;
    Ok(result)
}
