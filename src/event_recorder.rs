use std::{
    collections::HashMap,
    fmt::Display,
    fs::{canonicalize, create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Instant,
};

use anyhow::{bail, Context};
use chrono::{Local, NaiveTime, Timelike};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::Config,
    ping::{PingResponse, Target},
    state_management::{Event, MonitorState},
    Discord, Email,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampedResponse {
    pub timestamp: Timestamp,
    pub response: PingResponse,
}

#[derive(Debug)]
/// Manages a target, tracking things like where to write the info to disk and what is pending being written
pub struct TargetHandler<'a> {
    host_disp_name: String,
    pending_for_file: Vec<TimestampedResponse>,
    file_handle: File,
    file_path: PathBuf,
    time_sensitive_part_of_filename: String,
    state: MonitorState,
    last_write_to_disk_time: Option<Instant>,
    config: &'a Config,
}

impl<'a> TargetHandler<'a> {
    const BASE_FOLDER: &'static str = "events";
    fn new(target: &Target, config: &'a Config) -> anyhow::Result<Self> {
        debug!("Creating new TargetHandler for: {target}");
        let host_disp_name = format!("{target}");
        let time_sensitive_part_of_filename = Self::create_time_part_for_filename();
        let (file_path, file_handle) =
            Self::create_file_handle(&host_disp_name, &time_sensitive_part_of_filename)
                .context("failed creating file handle during TargetInfo initialization")?;
        let result = Self {
            host_disp_name,
            pending_for_file: Default::default(),
            file_handle,
            file_path,
            time_sensitive_part_of_filename,
            state: MonitorState::new(config),
            last_write_to_disk_time: None,
            config,
        };
        debug!("Succeeded in creating TargetHandler: {result:?}");
        Ok(result)
    }

    fn create_file_handle(
        host_identifier: &str,
        time_sensitive_part_of_filename: &str,
    ) -> anyhow::Result<(PathBuf, File)> {
        let new_filename = format!(
            "{} {} events.log",
            time_sensitive_part_of_filename, host_identifier
        );
        debug!("Creating new file handle for {new_filename:?}");

        let path = Path::new(Self::BASE_FOLDER);
        create_dir_all(path).context("failed to create base directory for events")?;

        let path = path.join(new_filename);
        let result = match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => {
                debug!("File created new for {path:?}");
                file
            }
            Err(err_new) => {
                // Try to open for append otherwise report both errors
                match File::options().append(true).open(&path) {
                    Ok(file) => {
                        debug!("File opened with append for {path:?}");
                        file
                    }
                    Err(err_append) => {
                        bail!("Unable to open {path:?} as new file with error: {err_new} nor as append with error: {err_append}");
                    }
                }
            }
        };

        Ok((path, result))
    }

    fn create_time_part_for_filename() -> String {
        format!("{}", Local::now().format("%F"))
    }

    /// Updates the file handle when it needs to roll over
    fn update_file_handle(&mut self) -> anyhow::Result<()> {
        let new_time_part = Self::create_time_part_for_filename();
        if self.time_sensitive_part_of_filename != new_time_part {
            debug!("Updating file handle for: {}", self.host_disp_name);
            let (new_path, new_handle) =
                Self::create_file_handle(&self.host_disp_name, &new_time_part)
                    .context("creating new file handle for update failed")?;
            self.time_sensitive_part_of_filename = new_time_part;
            self.file_handle = new_handle;
            self.file_path = new_path;
        }
        Ok(())
    }

    fn receive_response(
        &mut self,
        response: TimestampedResponse,
    ) -> anyhow::Result<Option<EventMessage>> {
        let event = self.state.process_response(&response);
        let result = if let Some(event) = event {
            Some(EventMessage::new(self.host_disp_name.to_string(), event))
        } else {
            None
        };
        self.pending_for_file.push(response);
        self.update_file_handle()
            .context("failed to update FileHandle")?;
        self.write_to_file().context("failed to write to file")?;
        Ok(result)
    }

    fn write_to_file(&mut self) -> anyhow::Result<()> {
        let min_time_between_write = self.config.min_time_between_write;
        if let Some(last) = self.last_write_to_disk_time {
            if last.elapsed().as_secs() < min_time_between_write.into()
                || self.pending_for_file.is_empty()
            {
                return Ok(()); // Do nothing enough time has not passed yet or nothing to write
            }
        }
        debug!(
            "{} has {} pending messages being written to disk at {:?}",
            self.host_disp_name,
            self.pending_for_file.len(),
            self.file_path
        );

        // Write all messages to disk
        for response in self.pending_for_file.drain(..) {
            writeln!(self.file_handle, "{}", &json!(response).to_string())
                .with_context(|| format!("failed to write to file: {:?}", self.file_path))?;
        }
        debug_assert!(self.pending_for_file.is_empty());

        self.last_write_to_disk_time = Some(Instant::now());
        Ok(())
    }
}

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TargetID(usize);

impl TargetID {
    fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Timestamp(String);

impl Timestamp {
    pub fn new() -> Self {
        Self(format!("{}", Local::now().format("%F %T")))
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct ResponseMessage {
    id: TargetID,
    timestamp: Timestamp,
    response: PingResponse,
}

impl ResponseMessage {
    pub fn new(id: TargetID, response: PingResponse) -> Self {
        Self {
            id,
            timestamp: Timestamp::new(),
            response,
        }
    }

    fn into_response(self) -> TimestampedResponse {
        TimestampedResponse {
            timestamp: self.timestamp,
            response: self.response,
        }
    }
}

#[derive(Debug)]
struct EventMessage {
    host_disp_name: String,
    timestamp: Timestamp,
    event: Event,
}

impl EventMessage {
    pub fn new(host_disp_name: String, event: Event) -> Self {
        Self {
            host_disp_name,
            timestamp: Timestamp::new(),
            event,
        }
    }

    fn system_message(event: Event) -> Self {
        Self::new("SYSTEM_MSG".to_string(), event)
    }
}

/// Handles all incoming events and sends them to the right handler based on the ID in the message
pub struct ResponseManager<'a> {
    rx_ping_response: Receiver<ResponseMessage>,
    tx_events: Sender<EventMessage>,
    target_map: HashMap<TargetID, TargetHandler<'a>>,
    next_id: TargetID,
    config: &'a Config,
}

impl<'a> ResponseManager<'a> {
    pub fn new(
        rx_ping_response: Receiver<ResponseMessage>,
        config: &'a Config,
    ) -> anyhow::Result<Self> {
        debug!("New event manager being created");
        let (tx_events, rx) = mpsc::channel();
        Self::start_event_thread(rx)?;
        Ok(Self {
            rx_ping_response,
            tx_events,
            target_map: Default::default(),
            next_id: Default::default(),
            config,
        })
    }

    pub fn register_target(&mut self, target: &Target) -> anyhow::Result<TargetID> {
        debug_assert!(!self.target_map.contains_key(&self.next_id));
        let result = self.next_id;
        self.target_map
            .insert(result, TargetHandler::new(target, self.config)?);
        self.next_id = result.next(); // Update ID for next call
        Ok(result)
    }

    /// Blocks forever receiving messages from ping threads
    pub fn start_receive_loop(&mut self) {
        debug!("Main Receive loop started for ping responses");
        loop {
            let msg = self.rx_ping_response.recv().expect("no Senders found");

            let handler = self
                .target_map
                .get_mut(&msg.id)
                .expect("failed to get handler for ID");

            match handler
                .receive_response(msg.into_response())
                .context("failed to handle response")
            {
                Ok(Some(event_msg)) => {
                    if let Err(err) = self
                        .tx_events
                        .send(event_msg)
                        .context("failed to send event. Event dispatch thread likely panicked")
                    {
                        error!("{err:?}");
                    };
                }
                Ok(None) => (), // No event nothing needed to be done
                Err(e) => {
                    error!("{e:?}");
                    if let Err(err) =
                        self.tx_events
                            .send(EventMessage::system_message(Event::SystemError(format!(
                                "{e:?}"
                            ))))
                    {
                        error!("{err:?}");
                    }
                }
            }
        }
    }

    fn start_event_thread(rx: Receiver<EventMessage>) -> anyhow::Result<()> {
        let discord: Option<Discord> = match Discord::new() {
            Ok(d) => Some(d),
            Err(e) => {
                error!(
                    "Unable to setup discord. Discord notifications will be disabled.Error:\n{e:?}"
                );
                None
            }
        };
        let email: Option<Email> = match Email::new() {
            Ok(client) => Some(client),
            Err(e) => {
                error!("Unable to setup email. Email notifications will be disabled. {e:?}");
                None
            }
        };
        thread::Builder::new()
            .name("EventDispatch".to_string())
            .spawn(move || loop {
                let event_message = rx.recv().expect("failed to receive event message");

                let EventMessage {
                    host_disp_name: name,
                    timestamp,
                    event,
                } = event_message;
                let notification_message = format!("{timestamp} - {name} - {event}",);
                let msg = &notification_message;

                if Event::Startup == event {
                    // Test all comms methods
                    if discord.is_some() && !Self::send_via_discord(discord.as_ref(), msg) {
                        error!("Test of discord failed");
                    }
                    if email.is_some() && !Self::send_via_email(email.as_ref(), msg) {
                        error!("Test of email failed");
                    }
                } else if !Self::send_via_discord(discord.as_ref(), msg)
                    && !Self::send_via_email(email.as_ref(), msg)
                {
                    error!("failed to send notification via all means. Message was: {msg:?}");
                }
            })
            .context("failed to start event loop thread")?;
        Ok(())
    }

    /// Attempts to send the message via discord, if there is no discord set or there is an error it returns false
    /// Not sure if a true is guaranteed message sent but at least we couldn't detect the error
    fn send_via_discord(discord: Option<&Discord>, msg: &str) -> bool {
        match discord {
            Some(discord) => match discord.send(msg) {
                Ok(()) => true,
                Err(e) => {
                    error!("failed to send message via discord: {e:?}");
                    false
                }
            },
            None => {
                debug!("Discord not set. Message not sent via discord");
                false
            }
        }
    }

    /// Attempts to send the message via email, if there is no email set or there is an error it returns false
    /// Not sure if a true is guaranteed message sent but at least we couldn't detect the error
    fn send_via_email(email: Option<&Email>, msg: &str) -> bool {
        match email {
            Some(email) => match email.send(msg) {
                Ok(()) => true,
                Err(e) => {
                    error!("failed to send message via email: {e:?}");
                    false
                }
            },
            None => {
                debug!("Email not set. Message not sent via email");
                false
            }
        }
    }

    pub(crate) fn start_keep_alive(&self) -> anyhow::Result<()> {
        let tx = self.tx_events.clone();
        let start = Instant::now();
        let keep_alive_time_of_day = self.config.keep_alive_time_of_day;

        tx.send(EventMessage::system_message(Event::Startup))
            .expect("failed to send startup event");

        if let Some(target_time) = keep_alive_time_of_day {
            warn!("Keep Alive time set for: {target_time:?}");
            thread::Builder::new()
                .name("KeepAlive".to_string())
                .spawn(move || loop {
                    let sleep_duration = seconds_to_time(target_time).expect(
                        "This should always be valid as we are only asking for at most 1 day",
                    );
                    thread::sleep(sleep_duration);
                    tx.send(EventMessage::system_message(Event::IAmAlive(
                        start.elapsed().as_secs().into(),
                    )))
                    .expect("failed to send keep alive event");
                })
                .context("failed to start keep alive thread")?;
        } else {
            warn!("Keep Alive notifications disabled");
        }
        Ok(())
    }

    pub(crate) fn log_events_output_folder(&self) -> anyhow::Result<()> {
        let event_output_folder = TargetHandler::BASE_FOLDER;
        let event_output_folder = canonicalize(event_output_folder)
            .context("canonicalization of event output folder failed")?;
        let msg = format!("Event logs are being stored at: {event_output_folder:?}");
        info!("{msg}");
        println!("{msg}");
        Ok(())
    }
}

fn seconds_to_time(target_time: NaiveTime) -> anyhow::Result<std::time::Duration> {
    let now = chrono::Local::now();
    let day_shift = if now.naive_local().time() > target_time {
        1
    } else {
        0
    };

    let new_date_time = (now + chrono::Duration::days(day_shift))
        .date_naive()
        .and_hms_opt(
            target_time.hour(),
            target_time.minute(),
            target_time.second(),
        )
        .expect("should always be valid built from NaiveTime");

    let result = new_date_time
        .signed_duration_since(now.naive_local())
        .to_std()?;
    Ok(result)
}
