use clap::Clap;

/// Creates a new group using the login credentials provided or prompted for
#[derive(Clap)]
pub struct CreateGroup {
    /// Name of the group to be created
    #[clap(index = 1)]
    pub group: String,
}