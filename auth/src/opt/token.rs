use clap::Parser;

use super::*;

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsToken {
    #[clap(subcommand)]
    pub action: TokenAction,
}

#[derive(Parser)]
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