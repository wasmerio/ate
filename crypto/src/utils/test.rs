#![allow(unused_imports)]
use std::sync::Once;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

static INIT: Once = Once::new();

pub fn bootstrap_test_env() {
    INIT.call_once(|| {
        super::log_init(0, false);
    });
}
