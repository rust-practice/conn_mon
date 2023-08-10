use anyhow::{bail, Context};
use log::{debug, warn};
use regex::Regex;
use std::{process::Command, sync::OnceLock};

use crate::{Milliseconds, Seconds};

/// Finds the round trip time to the target if less than timeout
pub fn ping(target: &Target) -> anyhow::Result<PingResponse> {
    let mut cmd = Command::new("ping");
    cmd.arg("-c").arg("1");

    match &target.timeout {
        Some(duration) => {
            cmd.arg("-W").arg(duration.to_string());
        }
        None => (), // TODO Once we have global settings we need to check those for a value
    }

    let output = cmd
        .arg(&target.host)
        .output()
        .context("Failed to execute ping")?;
    let stdout = std::str::from_utf8(&output.stdout).context("Failed to convert stdout to ut8")?;

    // Check if stderr is not empty
    if !output.stderr.is_empty() {
        let stderr =
            std::str::from_utf8(&output.stderr).context("Failed to convert stdout to ut8")?;
        warn!("Pinging {target:?} stderr not empty: {stderr:?}");
    }

    stdout.try_into()
}

#[derive(Debug)]
pub struct Target {
    /// The argument to be used when sending the ping request
    pub host: String,

    /// Value to be used when referring to this host in a user facing context
    pub display_name: Option<String>,

    /// If supplied overrides the global default timeout for waiting for a response
    pub timeout: Option<Seconds>,
}

impl Target {
    pub fn new(host: String) -> Self {
        Self {
            host,
            display_name: None,
            timeout: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PingResponse {
    Time(Milliseconds),
    Timeout,
    Error { msg: String },
}

impl TryFrom<&str> for PingResponse {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        static CELL_PASS: OnceLock<Regex> = OnceLock::new();
        static CELL_FAIL: OnceLock<Regex> = OnceLock::new();
        let re_pass = CELL_PASS.get_or_init(|| {
            debug!("Compile regex for passing ping responses");
            Regex::new(r"icmp_seq=\d+ ttl=\d+ time=(\d+)\.(\d+) ms")
                .expect("Failed to compile regex")
        });
        let re_fail = CELL_FAIL.get_or_init(|| {
            Regex::new(r"bytes of data.\n(?:(.*)\n)?\n---.*\n1 packets transmitted, 0 received")
                .expect("Failed to compile regex")
        });

        if let Some(captures) = re_pass.captures(value) {
            // Regex matched and can only match if both capture groups are found as they are not optional
            let ms = captures.get(1).unwrap();
            let ms_frac = captures.get(2).unwrap();

            Ok(PingResponse::Time(Milliseconds::try_from((
                ms.as_str(),
                ms_frac.as_str(),
            ))?))
        } else if let Some(captures) = re_fail.captures(value) {
            match captures.get(1) {
                Some(error_msg) => Ok(PingResponse::Error {
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
        let expected = PingResponse::Error {
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