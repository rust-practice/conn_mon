use std::{fmt::Display, time::Instant};

use crate::{
    config::Config, event_recorder::TimestampedResponse, ping::PingResponse, units::Seconds,
};

#[derive(Debug)]
pub struct MonitorState {
    state: State,
    notify_remind_interval: Seconds,
    min_time_before_first_down_notification: Seconds,
}

#[derive(Debug)]
enum State {
    Start,
    Up,
    Down {
        start: Instant,
        last_notify: Option<Instant>,
    },
    SystemError {
        start: Instant,
        last_notify: Instant,
    },
}
impl State {
    fn down_now() -> Self {
        Self::Down {
            start: Instant::now(),
            last_notify: None,
        }
    }

    fn error_now() -> Self {
        Self::SystemError {
            start: Instant::now(),
            last_notify: Instant::now(),
        }
    }
}

impl MonitorState {
    pub fn new(config: &Config) -> Self {
        Self {
            state: State::Start,
            notify_remind_interval: config.notify_remind_interval,
            min_time_before_first_down_notification: config.min_time_before_first_down_notification,
        }
    }

    /// Updates the state and returns an event if one occurred as a result of the transition applicable
    pub fn process_response(
        &mut self,
        timestamped_response: &TimestampedResponse,
    ) -> Option<Event> {
        let ping_response = &timestamped_response.response;
        let result;
        (result, self.state) = match self.state {
            State::Start | State::Up => match ping_response {
                PingResponse::Time(_ms) => (None, State::Up),
                PingResponse::Timeout | PingResponse::ErrorPing { .. } => {
                    if self.min_time_before_first_down_notification == 0.into() {
                        (
                            Some(Event::ConnectionFailed(0.into())),
                            State::Down {
                                start: Instant::now(),
                                last_notify: Some(Instant::now()),
                            },
                        )
                    } else {
                        (None, State::down_now())
                    }
                }
                PingResponse::ErrorOS { msg } | PingResponse::ErrorProgramming { msg } => {
                    Self::new_system_error(msg)
                }
            },
            State::Down { start, last_notify } => match ping_response {
                PingResponse::Time(_ms) => {
                    let notification = if last_notify.is_some() {
                        Some(Event::ConnectionRestoredAfter(
                            start.elapsed().as_secs().into(),
                        ))
                    } else {
                        None
                    };
                    (notification, State::Up)
                }
                PingResponse::Timeout => {
                    let notification = if self.should_notify() {
                        if last_notify.is_none() {
                            Some(Event::ConnectionFailed(start.elapsed().as_secs().into()))
                        } else {
                            Some(Event::ConnectionStillDown(start.elapsed().as_secs().into()))
                        }
                    } else {
                        None
                    };
                    let last_notify = if notification.is_some() {
                        Some(Instant::now())
                    } else {
                        last_notify
                    };
                    (notification, State::Down { start, last_notify })
                }
                PingResponse::ErrorPing { msg } => {
                    let notification = if self.should_notify() {
                        if last_notify.is_none() {
                            Some(Event::ConnectionError(
                                start.elapsed().as_secs().into(),
                                msg.to_string(),
                            ))
                        } else {
                            Some(Event::ConnectionStillDown(start.elapsed().as_secs().into()))
                        }
                    } else {
                        None
                    };
                    let last_notify = if notification.is_some() {
                        Some(Instant::now())
                    } else {
                        last_notify
                    };
                    (notification, State::Down { start, last_notify })
                }
                PingResponse::ErrorOS { msg } | PingResponse::ErrorProgramming { msg } => {
                    (Some(Event::SystemError(msg.clone())), State::error_now())
                }
            },
            State::SystemError { start, last_notify } => match ping_response {
                PingResponse::Time(_ms) => (
                    Some(Event::ConnectionRestoredAfter(
                        start.elapsed().as_secs().into(),
                    )),
                    State::Up,
                ),
                PingResponse::Timeout | PingResponse::ErrorPing { .. } => (None, State::down_now()),
                PingResponse::ErrorOS { .. } | PingResponse::ErrorProgramming { .. } => {
                    let notification = if self.should_notify() {
                        Some(Event::StillSystemError(start.elapsed().as_secs().into()))
                    } else {
                        None
                    };
                    let last_notify = if notification.is_some() {
                        Instant::now()
                    } else {
                        last_notify
                    };
                    (notification, State::SystemError { start, last_notify })
                }
            },
        };
        result
    }

    fn new_system_error(msg: &str) -> (Option<Event>, State) {
        (
            Some(Event::SystemError(msg.to_string())),
            State::error_now(),
        )
    }

    /// Meant for Down and SystemError only but couldn't find easy way to make function only compile if in one of those states
    /// Others just always return true as this function is not meant for them
    fn should_notify(&self) -> bool {
        let last_notify = match self.state {
            State::Start | State::Up => return true,
            State::Down { start, last_notify } => match last_notify {
                Some(last) => last,
                None => {
                    return start.elapsed().as_secs()
                        >= self.min_time_before_first_down_notification.into()
                }
            },
            State::SystemError { last_notify, .. } => last_notify,
        };

        last_notify.elapsed().as_secs() >= self.notify_remind_interval.into()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Startup,
    IAmAlive(Seconds),
    ConnectionFailed(Seconds),
    ConnectionError(Seconds, String),
    ConnectionStillDown(Seconds),
    ConnectionRestoredAfter(Seconds),
    SystemError(String),
    StillSystemError(Seconds),
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            Event::Startup => "Monitoring Tool Started Up".to_string(),
            Event::IAmAlive(uptime) => format!("I'm still alive. Uptime: {uptime}"),
            Event::ConnectionFailed(duration) => {
                format!("NEW Down. Outage duration IS {duration}")
            }
            Event::ConnectionError(duration, err_msg) => {
                format!("Error connecting with message {err_msg:?}. Outage duration IS {duration}")
            }
            Event::ConnectionStillDown(duration) => {
                format!("STILL down. Outage duration IS {duration}")
            }
            Event::StillSystemError(duration) => {
                format!("System Error persists. Error duration IS {duration}")
            }
            Event::ConnectionRestoredAfter(duration) => {
                format!("Connection back UP. Outage duration WAS {duration}")
            }
            Event::SystemError(err_msg) => {
                format!("System error with message {err_msg:?}")
            }
        };
        write!(f, "{result}")
    }
}
