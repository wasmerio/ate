use clap::Clap;

use super::*;

#[derive(Clap)]
#[clap()]
pub struct OptsToken {
    #[clap(subcommand)]
    pub action: TokenAction,
}

#[derive(Clap)]
pub enum TokenAction {
    /// Generate a token with normal permissions from the supplied username and password
    #[clap()]
    Generate(GenerateToken),
    /// Generate a token with extra permissions with elevated rights to modify groups and other higher risk actions
    #[clap()]
    Sudo(CreateTokenSudo),
    /// Gather the permissions needed to access a specific group into the token using either another supplied token or the prompted credentials
    #[clap()]
    Gather(GatherPermissions),
    /// Views the contents of the supplied token
    #[clap()]
    View(ViewToken),
}