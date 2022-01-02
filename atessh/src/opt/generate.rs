use clap::Parser;

/// Generates the SSH server key that helps protect from man-in-the-middle attacks
#[derive(Parser)]
pub struct OptsGenerate {
    /// Path to the secret server key
    #[clap(index = 1, default_value = "~/ate/ssh.server.key")]
    pub key_path: String,
}
