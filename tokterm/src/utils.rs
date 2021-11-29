#![allow(unused_imports)]
use tracing::metadata::LevelFilter;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::EnvFilter;

pub fn log_init(verbose: i32, debug: bool) {
    let mut log_level = match verbose {
        0 => None,
        1 => Some(LevelFilter::WARN),
        2 => Some(LevelFilter::INFO),
        3 => Some(LevelFilter::DEBUG),
        4 => Some(LevelFilter::TRACE),
        _ => None,
    };
    if debug {
        log_level = Some(LevelFilter::DEBUG);
    }

    if let Some(log_level) = log_level {
        SubscriberBuilder::default()
            .with_max_level(log_level)
            .init();
    } else {
        SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }
}
