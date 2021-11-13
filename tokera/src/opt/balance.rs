use clap::Parser;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsBalance {
    /// Show the individual coins that make up the balance
    #[clap(short, long)]
    pub coins: bool,
    /// When reading the balance the wallet is first reconciled - to prevent this happening then set this flag
    #[clap(long)]
    pub no_reconcile: bool,
}