use clap::Parser;

/// Creates a new user and login credentials on the authentication server
#[derive(Parser)]
pub struct CreateUser {
    /// Email address of the user to be created
    #[clap(index = 1)]
    pub email: Option<String>,
    /// New password to be associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
}