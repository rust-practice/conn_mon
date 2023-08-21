use std::fs;

use anyhow::Context;
use log::warn;

pub struct Email {}
impl Email {
    pub fn new() -> anyhow::Result<Self> {
        let filename = "e.data";
        let credentials = fs::read_to_string(filename)
            .with_context(|| format!("Failed to read email credentials from {filename:?}"))?;
        todo!();
        Ok(Self {})
    }

    pub fn send(&self, msg: &str) -> anyhow::Result<()> {
        warn!("EMAIL MESSAGE: {msg}");
        todo!();
        Ok(())
    }
}
