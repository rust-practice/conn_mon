use std::{process::Command, sync::OnceLock};

use anyhow::{bail, Context};
use env_logger::Builder;
use log::{debug, warn, LevelFilter};
use regex::Regex;

fn main() -> anyhow::Result<()> {
    init_logging(LevelFilter::Debug)?;
    let targets = vec![
        Target::new("127.0.0.1".to_string(), None),
        Target::new("8.8.8.8".to_string(), None),
        Target::new("192.168.1.205".to_string(), None),
        Target::new("192.168.8.8".to_string(), None),
    ];

    for target in targets {
        let result = ping(&target);
        let _ = dbg!(result);
        println!("-----");
    }
    Ok(())
}

fn init_logging(level: LevelFilter) -> anyhow::Result<()> {
    Builder::new().filter(None, level).try_init()?;
    Ok(())
}

/// Finds the round trip time to the target if less than timeout
fn ping(target: &Target) -> anyhow::Result<PingResponse> {
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
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
struct Target {
    host: String,

    /// If supplied overrides the global default timeout for waiting for a response
    timeout: Option<Seconds>,
}

impl Target {
    fn new(host: String, timeout: Option<Seconds>) -> Self {
        Self { host, timeout }
    }
}

#[derive(Debug)]
struct Milliseconds(u16);

#[derive(Debug)]
struct Seconds(u8);

#[derive(Debug)]
enum PingResponse {
    Time(Milliseconds),
    Timeout,
    Error { msg: String },
}

impl From<u16> for Milliseconds {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<u8> for Seconds {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl TryFrom<(&str, &str)> for Milliseconds {
    type Error = anyhow::Error;

    fn try_from((ms, ms_frac): (&str, &str)) -> Result<Self, Self::Error> {
        let mut ms: u16 = ms.parse().context("Failed to parse ms in ping")?;
        let ms_frac: u16 = ms_frac.parse().context("Failed to parse ms in ping")?;
        if ms_frac >= 50 {
            ms += 1;
        }
        Ok(Self(ms))
    }
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
