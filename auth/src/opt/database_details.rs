use clap::Clap;

/// Display the details about a particular database
#[derive(Clap)]
pub struct DatabaseDetails {
    /// Name of the database to query
    #[clap(index = 1)]
    pub name: String,
}