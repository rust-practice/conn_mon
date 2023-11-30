// Copied and edited based on https://github.com/estk/log4rs/pull/295

use anyhow::Context;
use log::LevelFilter;
use log4rs::Handle;
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};

pub fn init_logging(level: LevelFilter) -> anyhow::Result<Handle> {
    let file_path = "log/file.log";
    let archive_pattern = "log/file_{}.log";
    // Pattern: https://docs.rs/log4rs/*/log4rs/append/rolling_file/policy/compound/roll/fixed_window/struct.FixedWindowRollerBuilder.html#method.build

    // Build a stderr logger.
    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

    // Create a policy to use with the file logging
    let trigger = SizeTrigger::new(2_097_152); // 2mb (2 * 1024 * 1024)
    let roller = FixedWindowRoller::builder()
        .build(archive_pattern, 10) // Roll based on pattern and max 10 archive files
        .expect("Failed to create FixedWindowRoller");
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    // Logging to log file. (with rolling)
    let log_file = log4rs::append::rolling_file::RollingFileAppender::builder()
        // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} {l} - {m}\n",
        )))
        .build(file_path, Box::new(policy))
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("stderr", Box::new(stderr)),
        )
        .build(
            Root::builder()
                .appender("log_file")
                .appender("stderr")
                .build(level),
        )
        .context("Failed to configure logging")?;

    // Use this to change log levels at runtime.
    // This means you can change the default log level to trace
    // if you are trying to debug an issue and need more logs on then turn it off
    // once you are done.
    let handle = log4rs::init_config(config).context("Failed to init_config")?;

    Ok(handle)
}
