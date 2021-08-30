use clap::Clap;

/// Logs into the authentication server using the supplied credentials
#[derive(Clap)]
pub struct GenerateToken {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
}