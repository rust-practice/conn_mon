use std::sync::mpsc::Sender;

use crate::{
    event_recorder::{EventMessage, TargetID},
    ping::PingResponse,
};

pub struct EventPublisher {
    target_id: TargetID,
    tx: Sender<EventMessage>,
    state: MonitorState,
}

impl EventPublisher {
    pub fn new(target_id: TargetID, tx: Sender<EventMessage>) -> Self {
        Self {
            target_id,
            tx,
            state: MonitorState::new(),
        }
    }

    pub fn process_response(&mut self, response: PingResponse) {
        todo!()
    }
}

enum MonitorState {
    Start,
    Up,
}

impl MonitorState {
    fn new() -> Self {
        Self::Start
    }
}
