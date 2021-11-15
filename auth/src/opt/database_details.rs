use clap::Parser;

/// Display the details about a particular database
#[derive(Parser)]
pub struct DatabaseDetails {
    /// Name of the database to query
    #[clap(index = 1)]
    pub name: String,
}
