use clap::Parser;

#[allow(dead_code)]
#[derive(Parser)]
#[clap(version = "1.5", author = "Wasmer Inc <info@wasmer.io>")]
pub struct OptsLogin {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
    /// Flag that indicates if you will login as SUDO which is a high priv session
    /// that has access to make changes to the wallet without MFA challenges
    #[clap(long)]
    pub sudo: bool,
    /// Supplies an encoded token as a login context
    #[clap(long)]
    pub token: Option<String>,
}
