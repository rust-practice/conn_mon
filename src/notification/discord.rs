use std::fs;

use anyhow::{bail, Context};
use log::{error, warn};
use serenity::{builder::ExecuteWebhook, http::Http, model::webhook::Webhook};
use tokio::runtime::Runtime;

use crate::Seconds;

pub struct Discord {
    rt: Runtime,
    http: Http,
    url: String,
}

impl Discord {
    // TODO 2: Move this into a setting just hard coding as this is needed quickly
    const RETRY_ATTEMPTS: u8 = 3;
    const INTERVAL_BETWEEN_RETRY: Seconds = Seconds::new(15);

    pub fn new() -> anyhow::Result<Self> {
        let filename = "d.data";
        let url_suffix = fs::read_to_string(filename).with_context(|| {
            format!("failed to read discord webhook url suffix from {filename:?}")
        })?;
        let url = format!("https://discord.com/api/webhooks/{url_suffix}");
        let rt = tokio::runtime::Runtime::new().context("failed to create async runtime")?;
        let http = Http::new("");
        Ok(Self { rt, http, url })
    }

    pub fn send(&self, msg: &str) -> anyhow::Result<()> {
        warn!("DISCORD MESSAGE: {msg}");
        for i in 0..Self::RETRY_ATTEMPTS {
            // Wait before trying again
            if i > 0 {
                std::thread::sleep(Self::INTERVAL_BETWEEN_RETRY.into());
            }

            match self
                .rt
                .block_on(self.do_send(msg))
                .context("failed to send ")
            {
                Ok(()) => return Ok(()),
                Err(e) => error!(
                    "attempt #{} failed to send via discord. Error: {e:?}",
                    i + 1
                ),
            }
        }
        bail!(
            "failed to send via discord after {} attempts",
            Self::RETRY_ATTEMPTS
        )
    }

    async fn do_send(&self, msg: &str) -> anyhow::Result<()> {
        let webhook = Webhook::from_url(&self.http, &self.url)
            .await
            .context("failed to build webhook")?;
        let builder = ExecuteWebhook::new().content(msg);
        webhook
            .execute(&self.http, true, builder)
            .await
            .context("failed to send msg via discord using webhook")?;
        Ok(())
    }
}
