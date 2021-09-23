use clap::Clap;

/// Removes a particular  group
#[derive(Clap)]
pub struct GroupRemove {
    /// Name of the group to be removed
    #[clap(index = 1)]
    pub group: String,
}