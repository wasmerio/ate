#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use url::Url;
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Clap;
use ate_auth::opts::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    ate::log_init(opts.verbose, opts.debug);

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::User(opts_user) => {
            ate_auth::main_opts_user(opts_user, opts.token, opts.token_path, opts.auth).await?;
        },
        SubCommand::Group(opts_group) => {
            ate_auth::main_opts_group(opts_group, opts.token, opts.token_path, opts.auth, "Group").await?;
        },
        SubCommand::Token(opts_token) => {
            ate_auth::main_opts_token(opts_token, opts.token, opts.token_path, opts.auth).await?;
        }
    }

    // We are done
    Ok(())
}