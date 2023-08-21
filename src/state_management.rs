use std::time::Instant;

use crate::{event_recorder::TimestampedResponse, units::Seconds};

#[derive(Debug)]
pub struct MonitorState(State);

#[derive(Debug)]
enum State {
    Start,
    Up,
    Down(Instant),
}

impl MonitorState {
    pub fn new() -> Self {
        Self(State::Start)
    }

    /// Updates the state and fires and returns an event if applicable
    pub fn process_response(&self, response: &TimestampedResponse) -> Option<Event> {
        // TODO Handle state updates
        None
    }
}

#[derive(Debug)]
pub enum Event {
    ConnectionFailed(String),
    ConnectionStillDown(Seconds),
    ConnectionRestoredAfter(Seconds),
    Error(String),
}
