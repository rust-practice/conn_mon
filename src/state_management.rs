use std::time::Instant;

#[derive(Debug)]
pub struct MonitorState(State);

#[derive(Debug)]
enum State {
    Start,
    Up,
    Down(Instant),
}

impl MonitorState {
    // TODO Implement notifications
    pub fn new() -> Self {
        Self(State::Start)
    }
}
