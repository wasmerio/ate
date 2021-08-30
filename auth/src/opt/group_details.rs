use clap::Clap;

/// Display the details about a particular group
#[derive(Clap)]
pub struct GroupDetails {
    /// Name of the group to query
    #[clap(index = 1)]
    pub group: String,
    /// Determines if sudo permissions should be sought
    #[clap(long)]
    pub sudo: bool
}