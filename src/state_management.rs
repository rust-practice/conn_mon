use std::time::Instant;

use crate::{event_recorder::TimestampedResponse, ping::PingResponse, units::Seconds};

#[derive(Debug)]
pub struct MonitorState(State);

#[derive(Debug)]
enum State {
    Start,
    Up,
    Down {
        start: Instant,
        last_notify: Instant,
    },
    SysError {
        start: Instant,
        last_notify: Instant,
    },
}
impl State {
    fn down_now() -> Self {
        Self::Down {
            start: Instant::now(),
            last_notify: Instant::now(),
        }
    }

    fn error_now() -> Self {
        Self::SysError {
            start: Instant::now(),
            last_notify: Instant::now(),
        }
    }
}

impl MonitorState {
    pub fn new() -> Self {
        Self(State::Start)
    }

    /// Updates the state and fires and returns an event if applicable
    pub fn process_response(
        &mut self,
        timestamped_response: &TimestampedResponse,
    ) -> Option<Event> {
        let ping_response = &timestamped_response.response;
        let result;
        (result, self.0) = match self.0 {
            State::Start | State::Up => match ping_response {
                PingResponse::Time(_ms) => (None, State::Up),
                PingResponse::Timeout => Self::new_down(),
                PingResponse::ErrorPing { msg } => Self::new_ping_error(msg),
                PingResponse::ErrorOS { msg } | PingResponse::ErrorInternal { msg } => {
                    Self::new_os_error(msg)
                }
            },
            State::Down { start, last_notify } => match ping_response {
                PingResponse::Time(_ms) => (
                    Some(Event::ConnectionRestoredAfter(
                        start.elapsed().as_secs().into(),
                    )),
                    State::Up,
                ),
                PingResponse::Timeout | PingResponse::ErrorPing { .. } => {
                    let notification = if self.should_notify(last_notify) {
                        Some(Event::ConnectionStillDown(start.elapsed().as_secs().into()))
                    } else {
                        None
                    };
                    let last_notify = if notification.is_some() {
                        Instant::now()
                    } else {
                        last_notify
                    };
                    (notification, State::Down { start, last_notify })
                }
                PingResponse::ErrorOS { msg } | PingResponse::ErrorInternal { msg } => {
                    (Some(Event::OsError(msg.clone())), State::error_now())
                }
            },
            State::SysError { start, last_notify } => match ping_response {
                PingResponse::Time(_ms) => (
                    Some(Event::ConnectionRestoredAfter(
                        start.elapsed().as_secs().into(),
                    )),
                    State::Up,
                ),
                PingResponse::Timeout => Self::new_down(),
                PingResponse::ErrorPing { msg } => Self::new_ping_error(msg),
                PingResponse::ErrorOS { .. } | PingResponse::ErrorInternal { .. } => {
                    let notification = if self.should_notify(last_notify) {
                        Some(Event::ConnectionStillError(
                            start.elapsed().as_secs().into(),
                        ))
                    } else {
                        None
                    };
                    let last_notify = if notification.is_some() {
                        Instant::now()
                    } else {
                        last_notify
                    };
                    (notification, State::SysError { start, last_notify })
                }
            },
        };
        result
    }

    fn should_notify(&self, last_notify: Instant) -> bool {
        todo!()
    }

    fn new_down() -> (Option<Event>, State) {
        (Some(Event::ConnectionFailed), State::down_now())
    }

    fn new_ping_error(msg: &str) -> (Option<Event>, State) {
        (
            Some(Event::ConnectionError(msg.to_string())),
            State::down_now(),
        )
    }

    fn new_os_error(msg: &str) -> (Option<Event>, State) {
        (Some(Event::OsError(msg.to_string())), State::error_now())
    }
}

#[derive(Debug)]
pub enum Event {
    ConnectionFailed,
    ConnectionError(String),
    ConnectionStillDown(Seconds),
    ConnectionStillError(Seconds),
    ConnectionRestoredAfter(Seconds),
    OsError(String),
}
