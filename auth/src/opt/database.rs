use clap::Clap;
use url::Url;

use super::*;

#[derive(Clap)]
#[clap()]
pub struct OptsDatabase {
    /// URL where the data is remotely stored on a distributed commit log.
    #[clap(short, long, default_value = "ws://tokera.com/db")]
    pub remote: Url,
    #[clap(subcommand)]
    pub action: DatabaseAction,
}

#[derive(Clap)]
pub enum DatabaseAction {
    /// Truncates an existing database by tombstoning all the events
    #[clap()]
    Truncate(DatabaseTruncate),
    /// Display the details about a particular database
    #[clap()]
    Details(DatabaseDetails),
}