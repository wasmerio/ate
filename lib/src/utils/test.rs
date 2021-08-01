#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};
use std::sync::Once;

static INIT: Once = Once::new();

pub fn bootstrap_env() {
    INIT.call_once(|| {
        env_logger::init();
    });
}