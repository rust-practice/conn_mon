use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Instant,
};

use anyhow::{bail, Context};
use chrono::Local;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::Config,
    ping::{PingResponse, Target},
    state_management::{Event, MonitorState},
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
    fn new(target: &Target, config: &'a Config) -> anyhow::Result<Self> {
        debug!("Creating new TargetHandler for: {target}");
        let host_disp_name = format!("{target}");
        let time_sensitive_part_of_filename = Self::create_time_part_for_filename();
        let (file_path, file_handle) =
            Self::create_file_handle(&host_disp_name, &time_sensitive_part_of_filename)
                .context("Failed creating file handle during TargetInfo initialization")?;
        let result = Self {
            host_disp_name,
            pending_for_file: Default::default(),
            file_handle,
            file_path,
            time_sensitive_part_of_filename,
            state: MonitorState::new(config.notify_remind_interval),
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
        let base_folder = "events";
        let new_filename = format!(
            "{} {} events.log",
            time_sensitive_part_of_filename, host_identifier
        );
        debug!("Creating new file handle for {new_filename:?}");

        let path = Path::new(base_folder);
        create_dir_all(path).context("Failed to create base directory for events")?;

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
                    .context("Creating new file handle for update failed")?;
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
            Some(EventMessage {
                host_disp_name: self.host_disp_name.to_string(),
                event,
            })
        } else {
            None
        };
        self.pending_for_file.push(response);
        self.update_file_handle()
            .context("Failed to update FileHandle")?;
        self.write_to_file().context("Failed to write to file")?;
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
        trace!(
            "{} has {} pending messages being written to disk at {:?}",
            self.host_disp_name,
            self.pending_for_file.len(),
            self.file_path
        );

        // Write all messages to disk
        for response in self.pending_for_file.drain(..) {
            writeln!(self.file_handle, "{}", &json!(response).to_string())
                .with_context(|| format!("Failed to write to file: {:?}", self.file_path))?;
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
    event: Event,
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
        trace!("Main Receive loop started for ping responses");
        loop {
            let msg = self.rx_ping_response.recv().expect("No Senders found");
            let handler = self
                .target_map
                .get_mut(&msg.id)
                .expect("Failed to get handler for ID");
            if let Some(event_msg) = handler
                .receive_response(msg.into_response())
                .expect("Failed to handle response")
            {
                self.tx_events
                    .send(event_msg)
                    .expect("Failed to send event");
            }
        }
    }

    fn start_event_thread(rx: Receiver<EventMessage>) -> anyhow::Result<()> {
        thread::Builder::new()
            .name("EventDispatch".to_string())
            .spawn(move || loop {
                let msg = rx.recv().expect("Failed to receive event message");
                dbg!(&msg);
                let name = msg.host_disp_name;
                let notification_message = match msg.event {
                    Event::ConnectionFailed => "Connection Failed".to_string(),
                    Event::ConnectionError(err_msg) => {
                        format!("Error connecting with message {err_msg:?}")
                    }
                    Event::ConnectionStillDown(duration) => {
                        format!("Connection still down. Outage duration {duration}")
                    }
                    Event::ConnectionStillError(duration) => {
                        format!("Connection still in error. Outage duration {duration}")
                    }
                    Event::ConnectionRestoredAfter(duration) => {
                        format!("Connection Restored. Outage duration was {duration}")
                    }
                    Event::SystemError(err_msg) => {
                        format!("System error with message {err_msg:?}")
                    }
                };
                let notification_message = format!("{name:?} - {notification_message}");
                // TODO send message
                println!("{notification_message}");
            })
            .context("Failed to start event loop thread")?;
        Ok(())
    }
}
