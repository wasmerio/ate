#![allow(unused_imports)]
use serde::*;
use std::fs::File;
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

pub fn try_load_key<T>(key_path: String) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("loading key: {}", path);
    let path = std::path::Path::new(&path);
    File::open(path)
        .ok()
        .map(|file| bincode::deserialize_from(&file).unwrap())
}

pub fn load_key<T>(key_path: String) -> T
where
    T: serde::de::DeserializeOwned,
{
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("loading key: {}", path);
    let path = std::path::Path::new(&path);
    let file = File::open(path).expect(format!("failed to load key at {}", key_path).as_str());
    bincode::deserialize_from(&file).unwrap()
}

pub fn save_key<T>(key_path: String, key: T)
where
    T: Serialize,
{
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("saving key: {}", path);
    let path = std::path::Path::new(&path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
    let mut file = File::create(path).unwrap();
    bincode::serialize_into(&mut file, &key).unwrap();
}
