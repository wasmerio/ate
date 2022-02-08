use clap::Parser;

/// Gathers the permissions needed to access a specific group into the token using either another supplied token or the prompted credentials
#[derive(Parser)]
pub struct GatherPermissions {
    /// Name of the group to gather the permissions for
    #[clap(index = 1)]
    pub group: String,
    /// Determines if sudo permissions should be sought
    #[clap(long)]
    pub sudo: bool,
    /// Display the token in human readable format
    #[clap(long)]
    pub human: bool,
}
