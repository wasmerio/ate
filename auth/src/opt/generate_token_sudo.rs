use clap::Parser;

/// Logs into the authentication server using the supplied credentials and 2nd factor authentication
#[derive(Parser)]
pub struct CreateTokenSudo {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
    /// Authenticator code from your google authenticator
    #[clap(index = 3)]
    pub code: Option<String>,
}