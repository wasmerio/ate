use std::net::IpAddr;
use wasmer_term::wasmer_os;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

/// Runs a ssh server host
#[derive(Parser)]
pub struct OptsHost {
    /// IP address that the SSH server will isten on
    #[clap(short, long, default_value = "::")]
    pub listen: IpAddr,
    /// Port that the server will listen on for SSH requests
    #[clap(long, default_value = "22")]
    pub port: u16,
    /// Determines which compiler to use
    #[clap(short, long, default_value = "default")]
    pub compiler: wasmer_os::eval::Compiler,
    /// Location where cached compiled modules are stored
    #[clap(long, default_value = "~/wasmer/compiled")]
    pub compiler_cache_path: String,
    /// Location where webc files will be stored
    #[clap(long, default_value = "~/wasmer/webc")]
    pub webc_dir: String,
    /// Uses a local directory for native files rather than the published ate chain
    #[clap(long)]
    pub native_files_path: Option<String>,
}
