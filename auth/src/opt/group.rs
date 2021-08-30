use clap::Clap;

use super::*;

#[derive(Clap)]
#[clap()]
pub struct OptsGroup {
    #[clap(subcommand)]
    pub action: GroupAction,
}

#[derive(Clap)]
pub enum GroupAction {
    /// Creates a new group
    #[clap()]
    Create(CreateGroup),
    /// Adds another user to an existing group
    #[clap()]
    AddUser(GroupAddUser),
    /// Removes a user from an existing group
    #[clap()]
    RemoveUser(GroupRemoveUser),
    /// Display the details about a particular group (token is required to see role membership)
    #[clap()]
    Details(GroupDetails),
}