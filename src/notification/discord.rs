use std::fs;

use anyhow::Context;
use log::warn;
use serenity::{http::Http, model::webhook::Webhook};
use tokio::runtime::Runtime;

pub struct Discord {
    rt: Runtime,
    http: Http,
    webhook: Webhook,
}
impl Discord {
    pub fn new() -> anyhow::Result<Self> {
        let filename = "d.data";
        let url_suffix = fs::read_to_string(filename).with_context(|| {
            format!("Failed to read discord webhook url suffix from {filename:?}")
        })?;
        let url = format!("https://discord.com/api/webhooks/{url_suffix}");
        let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
        let http = Http::new("");
        let webhook = rt
            .block_on(Webhook::from_url(&http, &url))
            .context("Failed to build webhook")?;
        Ok(Self { rt, http, webhook })
    }

    pub fn send(&self, msg: &str) -> anyhow::Result<()> {
        warn!("DISCORD MESSAGE: {msg}");
        self.rt
            .block_on(self.webhook.execute(&self.http, true, |w| w.content(msg)))?;
        Ok(())
    }
}
