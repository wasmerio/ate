use clap::Parser;

/// Removes a particular  group
#[derive(Parser)]
pub struct GroupRemove {
    /// Name of the group to be removed
    #[clap(index = 1)]
    pub group: String,
}