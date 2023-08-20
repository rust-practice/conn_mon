use crate::ping::PingResponse;

pub struct EventPublisher {}

impl EventPublisher {
    pub fn new() -> Self {
        todo!();
        Self {}
    }

    pub fn process_response(&mut self, response: anyhow::Result<PingResponse>) {
        todo!()
    }
}

enum MonitorState {}
