use clap::Parser;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsRemoveWallet {
    /// Forces the wallet to be destroyed even if it has commodities in it
    #[clap(short, long)]
    pub force: bool,
}