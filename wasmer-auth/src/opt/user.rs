use clap::Parser;

use super::*;

#[derive(Parser)]
#[clap()]
pub struct OptsUser {
    #[clap(subcommand)]
    pub action: UserAction,
}

#[derive(Parser)]
pub enum UserAction {
    /// Creates a new user and generates login credentials
    #[clap()]
    Create(CreateUser),
    /// Returns all the details about a specific user
    #[clap()]
    Details,
    /// Recovers a lost account using your recovery code
    #[clap()]
    Recover(ResetUser),
}
