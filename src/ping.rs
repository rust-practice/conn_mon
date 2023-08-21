use anyhow::bail;
use log::{debug, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, process::Command, sync::OnceLock};

use crate::{Milliseconds, Seconds};

/// Finds the round trip time to the target if less than timeout
pub fn ping(target: &Target, default_timeout: &Seconds) -> PingResponse {
    let mut cmd = Command::new("ping");
    cmd.arg("-c").arg("1");

    // Set timeout
    cmd.arg("-W");
    match &target.timeout {
        Some(duration) => {
            cmd.arg(duration.to_string());
        }
        None => {
            cmd.arg(default_timeout.to_string());
        }
    }

    let output = match cmd.arg(&target.host).output() {
        Ok(out) => out,
        Err(e) => {
            return PingResponse::ErrorOS {
                msg: format!("Failed to execute ping: {e}"),
            }
        }
    };
    let stdout = match std::str::from_utf8(&output.stdout) {
        Ok(out) => out,
        Err(e) => {
            return PingResponse::ErrorOS {
                msg: format!("Failed to convert stdout to ut8: {e}"),
            }
        }
    };

    // Check if stderr is not empty
    if !output.stderr.is_empty() {
        let stderr = match std::str::from_utf8(&output.stderr) {
            Ok(out) => out,
            Err(e) => {
                return PingResponse::ErrorOS {
                    msg: format!("Failed to convert stdout to ut8: {e}"),
                }
            }
        };
        warn!("Pinging {target:?} stderr not empty: {stderr:?}");
    }

    match stdout.try_into() {
        Ok(result) => result,
        Err(e) => PingResponse::ErrorInternal {
            msg: format!("{e}"),
        },
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Target {
    /// The argument to be used when sending the ping request
    pub host: String,

    /// Value to be used when referring to this host in a user facing context
    pub display_name: Option<String>,

    /// If supplied overrides the global default timeout for waiting for a response
    pub timeout: Option<Seconds>,
}

impl From<&str> for Target {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<String> for Target {
    fn from(host: String) -> Self {
        Self {
            host,
            display_name: None,
            timeout: None,
        }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name.as_ref().unwrap_or(&self.host))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PingResponse {
    Time(Milliseconds),
    Timeout,
    ErrorPing { msg: String },
    ErrorOS { msg: String },
    ErrorInternal { msg: String },
}

impl TryFrom<&str> for PingResponse {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        static CELL_PASS: OnceLock<Regex> = OnceLock::new();
        static CELL_FAIL: OnceLock<Regex> = OnceLock::new();
        let re_pass = CELL_PASS.get_or_init(|| {
            debug!("Compile regex for parsing ping responses");
            Regex::new(r"icmp_seq=\d+ ttl=\d+ time=(\d+)\.?(\d+)? ms")
                .expect("Failed to compile regex")
        });
        let re_fail = CELL_FAIL.get_or_init(|| {
            Regex::new(r"bytes of data.\n(?:(.*)\n)?\n---.*\n1 packets transmitted, 0 received")
                .expect("Failed to compile regex")
        });

        if let Some(captures) = re_pass.captures(value) {
            // Regex matched and can only match if both capture groups are found as they are not optional
            let ms = captures.get(1).unwrap(); // Required for match
            let ms_frac = captures.get(2); // May not be present if value is 0

            Ok(PingResponse::Time(Milliseconds::try_from((
                ms.as_str(),
                if let Some(ms_frac) = ms_frac {
                    ms_frac.as_str()
                } else {
                    "0"
                },
            ))?))
        } else if let Some(captures) = re_fail.captures(value) {
            match captures.get(1) {
                Some(error_msg) => Ok(PingResponse::ErrorPing {
                    msg: error_msg.as_str().to_owned(),
                }),
                None => Ok(PingResponse::Timeout),
            }
        } else {
            bail!("Failed to convert value into PingResponse. Did not match pass nor fail. Value: {value:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_response_time() {
        // Arrange
        let expected = PingResponse::Time(5.into());
        let input = "PING 8.8.8.8 (8.8.8.8) 56(84) bytes of data.
64 bytes from 8.8.8.8: icmp_seq=1 ttl=117 time=5.32 ms

--- 8.8.8.8 ping statistics ---
1 packets transmitted, 1 received, 0% packet loss, time 0ms
rtt min/avg/max/mdev = 5.315/5.315/5.315/0.000 ms";

        // Act
        let actual: PingResponse = input.try_into().unwrap();

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn ping_response_time_no_frac_ms() {
        // Arrange
        let expected = PingResponse::Time(5.into());
        let input = "PING 8.8.8.8 (8.8.8.8) 56(84) bytes of data.
64 bytes from 8.8.8.8: icmp_seq=1 ttl=117 time=5 ms

--- 8.8.8.8 ping statistics ---
1 packets transmitted, 1 received, 0% packet loss, time 0ms
rtt min/avg/max/mdev = 5.315/5.315/5.315/0.000 ms";

        // Act
        let actual: PingResponse = input.try_into().unwrap();

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn ping_response_timeout() {
        // Arrange
        let expected = PingResponse::Timeout;
        let input = "PING 192.8.8.8 (192.8.8.8) 56(84) bytes of data.

--- 192.8.8.8 ping statistics ---
1 packets transmitted, 0 received, 100% packet loss, time 0ms";

        // Act
        let actual: PingResponse = input.try_into().unwrap();

        // Assert
        assert_eq!(actual, expected);
    }

    #[test]
    fn ping_response_error() {
        // Arrange
        let expected = PingResponse::ErrorPing {
            msg: "From 192.168.1.2 icmp_seq=1 Destination Host Unreachable".into(),
        };
        let input = "PING 192.168.1.205 (192.168.1.205) 56(84) bytes of data.
From 192.168.1.2 icmp_seq=1 Destination Host Unreachable

--- 192.168.1.205 ping statistics ---
1 packets transmitted, 0 received, +1 errors, 100% packet loss, time 0ms";

        // Act
        let actual: PingResponse = input.try_into().unwrap();

        // Assert
        assert_eq!(actual, expected);
    }
}
