use clap::Parser;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsTransactionHistory {
    /// Returns the history only for a particular year
    #[clap(long)]
    pub year: Option<i32>,
    /// Returns the history only for a particular month
    #[clap(long)]
    pub month: Option<u32>,
    /// Returns the history only for a particular day
    #[clap(long)]
    pub day: Option<u32>,
    /// Indicates if the details of each event should be displayed
    #[clap(short, long)]
    pub details: bool,
    /// Also show the current balance of the account
    #[clap(short, long)]
    pub balance: bool,
    /// When reading the balance the wallet is first reconciled - to prevent this happening then set this flag
    #[clap(long)]
    pub no_reconcile: bool,
}
