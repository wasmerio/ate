#![allow(unused_imports)]
use log::{info, warn, debug, error};
use url::Url;
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Clap;
use ate_auth::opts::*;

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    // Prepare the logging
    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::User(opts_user) => {
            ate_auth::main_opts_user(opts_user, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Group(opts_group) => {
            ate_auth::main_opts_group(opts_group, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Token(opts_token) => {
            ate_auth::main_opts_token(opts_token, opts.token, opts.token_path, opts.auth).await?;
        }
    }

    // We are done
    Ok(())
}