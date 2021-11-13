use clap::Parser;

use super::source::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsWallet {
    /// Wallet to perform the action on
    #[clap(subcommand)]
    pub source: OptsWalletSource,
}