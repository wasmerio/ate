use clap::Parser;

/// Removes a previously created database with a specific name
#[derive(Parser)]
pub struct DatabaseTruncate {
    /// Name of the database to be removed
    #[clap(index = 1)]
    pub name: String,
}
