use clap::Parser;
use url::Url;

use super::*;

#[derive(Parser)]
#[clap()]
pub struct OptsDatabase {
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://wasmer.sh/db).
    #[clap(short, long)]
    pub remote: Option<Url>,
    #[clap(subcommand)]
    pub action: DatabaseAction,
}

#[derive(Parser)]
pub enum DatabaseAction {
    /// Truncates an existing database by tombstoning all the events
    #[clap()]
    Truncate(DatabaseTruncate),
    /// Display the details about a particular database
    #[clap()]
    Details(DatabaseDetails),
}
