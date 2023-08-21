use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    path::Path,
    sync::mpsc::Receiver,
};

use anyhow::{bail, Context};
use chrono::Local;
use log::debug;

use crate::{
    ping::{PingResponse, Target},
    state_management::MonitorState,
    units::Seconds,
};

#[derive(Debug)]
/// Manages a target, tracking things like where to write the info to disk and what is pending being written
pub struct TargetHandler {
    file_identifier: String,
    pending_events: Vec<Event>,
    file_handle: File,
    time_sensitive_part_of_filename: String,
    state: MonitorState,
}

impl TargetHandler {
    fn new(target: &Target) -> anyhow::Result<Self> {
        debug!("Creating new TargetHandler for: {target}");
        let file_identifier = target.host.clone();
        let time_sensitive_part_of_filename = Self::create_time_part_for_filename();
        let file_handle =
            Self::create_file_handle(&file_identifier, &time_sensitive_part_of_filename)
                .context("Failed creating file handle during TargetInfo initialization")?;
        let result = Self {
            file_identifier,
            pending_events: Default::default(),
            file_handle,
            time_sensitive_part_of_filename,
            state: MonitorState::new(),
        };
        debug!("Succeeded in creating TargetHandler: {result:?}");
        Ok(result)
    }

    fn create_file_handle(
        host_identifier: &str,
        time_sensitive_part_of_filename: &str,
    ) -> anyhow::Result<File> {
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

        Ok(result)
    }

    fn create_time_part_for_filename() -> String {
        format!("{}", Local::now().format("%F"))
    }

    /// Updates the file handle when it needs to roll over
    fn update_file_handle(&mut self) -> anyhow::Result<()> {
        let new_time_part = Self::create_time_part_for_filename();
        if self.time_sensitive_part_of_filename != new_time_part {
            debug!("Updating file handle for: {}", self.file_identifier);
            let new_handle = Self::create_file_handle(&self.file_identifier, &new_time_part)
                .context("Creating new file handle for update failed")?;
            self.time_sensitive_part_of_filename = new_time_part;
            self.file_handle = new_handle;
        }
        Ok(())
    }

    fn receive_response(&mut self, response: PingResponse) {
        todo!("Save Response");
        todo!("Update State");
    }
}

#[derive(Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TargetID(usize);

impl TargetID {
    fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug)]
pub enum Event {
    ConnectionFailed(String),
    ConnectionStillDown(Seconds),
    ConnectionRestoredAfter(Seconds),
    Error(String),
}

pub struct ResponseMessage {
    id: TargetID,
    response: PingResponse,
}

impl ResponseMessage {
    pub fn new(id: TargetID, response: PingResponse) -> Self {
        Self { id, response }
    }
}

/// Handles all incoming events and sends them to the right handler based on the ID in the message
pub struct ResponseManager {
    rx: Receiver<ResponseMessage>,
    target_map: HashMap<TargetID, TargetHandler>,
    next_id: TargetID,
}

impl ResponseManager {
    pub fn new(rx: Receiver<ResponseMessage>) -> Self {
        debug!("New event manager being created");
        Self {
            rx,
            target_map: Default::default(),
            next_id: Default::default(),
        }
    }

    pub fn register_target(&mut self, target: &Target) -> anyhow::Result<TargetID> {
        debug_assert!(!self.target_map.contains_key(&self.next_id));
        let result = self.next_id;
        self.target_map.insert(result, TargetHandler::new(target)?);
        self.next_id = result.next(); // Update ID for next call
        Ok(result)
    }

    /// Blocks forever receiving messages from ping threads
    pub fn start_receive_loop(&mut self) {
        todo!()
    }
}
