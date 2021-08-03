#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use std::sync::Once;

static INIT: Once = Once::new();

pub fn bootstrap_test_env() {
    INIT.call_once(|| {
        super::log_init(0, false);
    });
}