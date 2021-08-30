use clap::Clap;

use super::*;

#[derive(Clap)]
#[clap()]
pub struct OptsUser {
    #[clap(subcommand)]
    pub action: UserAction,
}

#[derive(Clap)]
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