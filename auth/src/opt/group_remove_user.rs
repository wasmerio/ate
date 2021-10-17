use ate::prelude::*;
use clap::Parser;

/// Removes a particular user from a role within a group
#[derive(Parser)]
pub struct GroupRemoveUser {
    /// Name of the group that the user will be removed from
    #[clap(index = 1)]
    pub group: String,
    /// Role within the group that the user will be removed from, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    pub role: AteRolePurpose,
    /// Username that will be removed to the group role
    #[clap(index = 3)]
    pub username: String,
}