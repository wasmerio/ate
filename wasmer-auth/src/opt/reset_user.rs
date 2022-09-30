use clap::Parser;

/// Recovers a lost account using the recovery code that you stored somewhere safely when you created your account
#[derive(Parser)]
pub struct ResetUser {
    /// Email address of the user to be recovered
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Recovery code that you stored somewhere safely when you created your account
    #[clap(index = 2)]
    pub recovery_code: Option<String>,
    /// New password for the user
    #[clap(index = 3)]
    pub new_password: Option<String>,
    /// The authenticator code from your mobile authenticator
    #[clap(index = 4)]
    pub auth_code: Option<String>,
    /// The next authenticator code from your mobile authenticator
    #[clap(index = 5)]
    pub next_auth_code: Option<String>,
}
