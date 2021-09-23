use clap::Clap;
use url::Url;

use super::*;

#[derive(Clap)]
#[clap(version = "1.5", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth: Url,
    /// Token used to access your encrypted file-system (if you do not supply a token then you will
    /// be prompted for a username and password)
    #[clap(short, long)]
    pub token: Option<String>,
    /// Token file to read that holds a previously created token to be used to access your encrypted
    /// file-system (if you do not supply a token then you will be prompted for a username and password)
    #[clap(long)]
    pub token_path: Option<String>,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Users are personal accounts and services that have an authentication context
    #[clap()]
    User(OptsUser),
    /// Groups are collections of users that share something together
    #[clap()]
    Group(OptsDomain),
    /// Tokens are stored authentication and authorization secrets used by other processes
    #[clap()]
    Token(OptsToken),
}