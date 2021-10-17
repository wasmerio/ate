use ate::prelude::*;
use clap::Parser;

/// Adds a particular user to a role within a group
#[derive(Parser)]
pub struct GroupAddUser {
    /// Name of the group that the user will be added to
    #[clap(index = 1)]
    pub group: String,
    /// Role within the group that the user will be added to, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    pub role: AteRolePurpose,
    /// Username that will be added to the group role
    #[clap(index = 3)]
    pub username: String,
}