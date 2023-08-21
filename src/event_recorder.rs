use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
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
    state_management::MonitorState,
    utils::make_single_line,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampedResponse {
    timestamp: Timestamp,
    response: PingResponse,
}

#[derive(Debug)]
/// Manages a target, tracking things like where to write the info to disk and what is pending being written
pub struct TargetHandler<'a> {
    file_identifier: String,
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
        let file_identifier = target.host.clone();
        let time_sensitive_part_of_filename = Self::create_time_part_for_filename();
        let (file_path, file_handle) =
            Self::create_file_handle(&file_identifier, &time_sensitive_part_of_filename)
                .context("Failed creating file handle during TargetInfo initialization")?;
        let result = Self {
            file_identifier,
            pending_for_file: Default::default(),
            file_handle,
            file_path,
            time_sensitive_part_of_filename,
            state: MonitorState::new(),
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
            debug!("Updating file handle for: {}", self.file_identifier);
            let (new_path, new_handle) =
                Self::create_file_handle(&self.file_identifier, &new_time_part)
                    .context("Creating new file handle for update failed")?;
            self.time_sensitive_part_of_filename = new_time_part;
            self.file_handle = new_handle;
            self.file_path = new_path;
        }
        Ok(())
    }

    fn receive_response(&mut self, response: TimestampedResponse) -> anyhow::Result<()> {
        let event = self.state.process_response(&response);
        if let Some(event) = event {
            todo!("Send event to another thread that handles sending out notifications")
        }
        self.pending_for_file.push(response);
        self.update_file_handle()
            .context("Failed to update FileHandle")?;
        self.write_to_file().context("Failed to write to file")
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
            self.file_identifier,
            self.pending_for_file.len(),
            self.file_path
        );

        // Write all messages to disk
        for response in self.pending_for_file.drain(..) {
            writeln!(
                self.file_handle,
                "{}",
                make_single_line(&json!(response).to_string())
            )
            .with_context(|| format!("Failed to write to file: {}", self.file_identifier))?;
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

/// Handles all incoming events and sends them to the right handler based on the ID in the message
pub struct ResponseManager<'a> {
    rx: Receiver<ResponseMessage>,
    target_map: HashMap<TargetID, TargetHandler<'a>>,
    next_id: TargetID,
    config: &'a Config,
}

impl<'a> ResponseManager<'a> {
    pub fn new(rx: Receiver<ResponseMessage>, config: &'a Config) -> Self {
        debug!("New event manager being created");
        Self {
            rx,
            target_map: Default::default(),
            next_id: Default::default(),
            config,
        }
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
            let msg = self.rx.recv().expect("No Senders found");
            let handler = self
                .target_map
                .get_mut(&msg.id)
                .expect("Failed to get handler for ID");
            handler
                .receive_response(msg.into_response())
                .expect("Failed to handle response");
        }
    }
}
