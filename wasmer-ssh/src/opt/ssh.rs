#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use clap::Parser;

use super::generate::*;
use super::host::*;


#[derive(Parser)]
pub struct OptsSsh {
    #[clap(subcommand)]
    pub action: OptsSshAction,
}


#[derive(Parser)]
pub enum OptsSshAction {
    /// Starts a ssh host
    #[clap()]
    Host(OptsHost),
    /// Generates the SSH serve side keys
    #[clap()]
    Generate(OptsGenerate),
}
