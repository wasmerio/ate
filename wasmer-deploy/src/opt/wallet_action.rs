use clap::Parser;

use super::OptsBalance;
use super::OptsCreateWallet;
use super::OptsDeposit;
use super::OptsRemoveWallet;
use super::OptsTransactionHistory;
use super::OptsTransfer;
use super::OptsWithdraw;

#[derive(Parser, Clone)]
pub enum OptWalletAction {
    /// Creates this wallet with the supplied name
    #[clap()]
    Create(OptsCreateWallet),
    /// Removes this empty wallet
    #[clap()]
    Remove(OptsRemoveWallet),
    /// Displays the current balance
    #[clap()]
    Balance(OptsBalance),
    /// Displays the transaction history
    #[clap()]
    History(OptsTransactionHistory),
    /// Transfers a commodity (e.g. money) between two wallets
    #[clap()]
    Transfer(OptsTransfer),
    /// Deposit a wallet from an external source (e.g. PayPal Transfer)
    #[clap()]
    Deposit(OptsDeposit),
    /// Withdraws from the wallet to an external destination (e.g. PayPal Account)
    #[clap()]
    Withdraw(OptsWithdraw),
}
