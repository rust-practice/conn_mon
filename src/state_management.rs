use std::sync::mpsc::Sender;

use crate::{
    event_recorder::{EventMessage, TargetID},
    ping::PingResponse,
};

pub struct EventPublisher {}

impl EventPublisher {
    pub fn new(target_id: TargetID, tx: Sender<EventMessage>) -> Self {
        todo!();
        Self {}
    }

    pub fn process_response(&mut self, response: PingResponse) {
        todo!()
    }
}

enum MonitorState {}
