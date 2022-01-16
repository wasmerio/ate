#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug};
use ate::prelude::*;
use clap::Parser;

/// Generates the secret key that helps protect session
#[derive(Parser)]
pub struct Generate {
    /// Path to the secret key
    #[clap(index = 1, default_value = "~/ate/session.key")]
    pub key_path: String,
    /// Strength of the key that will be generated
    #[clap(short, long, default_value = "192")]
    pub strength: KeySize,
}