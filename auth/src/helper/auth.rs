#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use url::Url;

use ::ate::prelude::*;
use ::ate::crypto::EncryptKey;

use crate::model::*;

pub(crate) fn compute_user_auth(user: &User) -> AteSessionUser
{
    let mut session = AteSessionUser::default();
    for auth in user.access.iter() {
        session.user.add_read_key(&auth.read);
        session.user.add_private_read_key(&auth.private_read);
        session.user.add_write_key(&auth.write);
    }
    session.user.add_uid(user.uid);
    session.identity = user.email.clone();
    session.broker_read = Some(user.broker_read.clone());
    session.broker_write = Some(user.broker_write.clone());

    session
}

pub(crate) fn compute_sudo_auth(sudo: &Sudo, session: AteSessionUser) -> AteSessionSudo
{
    let mut role = AteGroupRole {
        purpose: AteRolePurpose::Owner,
        properties: Vec::new()
    };
    for auth in sudo.access.iter() {
        role.add_read_key(&auth.read);
        role.add_private_read_key(&auth.private_read);
        role.add_write_key(&auth.write);
    }
    role.add_read_key(&sudo.contract_read_key);
    role.add_uid(sudo.uid);

    AteSessionSudo {
        inner: session,
        sudo: role
    }
}

pub(crate) fn complete_group_auth(group: &Group, inner: AteSessionInner)
    -> Result<AteSessionGroup, LoadError>
{
    // Create the session that we will return to the call
    let mut session = AteSessionGroup::new(inner, group.name.clone());

    // Add the broker keys and contract read key
    session.group.broker_read = Some(group.broker_read.clone());
    session.group.broker_write = Some(group.broker_write.clone());
    
    // Enter a recursive loop that will expand its authorizations of the roles until
    // it expands no more or all the roles are gained.
    let mut roles = group.roles.iter().collect::<Vec<_>>();
    while roles.len() > 0 {
        let start = roles.len();
        let mut next = Vec::new();

        // Process all the roles
        let shared_keys = session.read_keys(AteSessionKeyCategory::AllKeys).map(|a| a.clone()).collect::<Vec<_>>();
        let super_keys = session.private_read_keys(AteSessionKeyCategory::AllKeys).map(|a| a.clone()).collect::<Vec<_>>();
        for role in roles.into_iter()
        {
            // Attempt to gain access to the role using the access rights of the super session
            let mut added = false;
            for read_key in super_keys.iter() {
                if let Some(a) = role.access.unwrap(&read_key)?
                {
                    // Add access rights to the session                    
                    let b = session.get_or_create_group_role(&role.purpose);
                    b.add_read_key(&a.read);
                    b.add_private_read_key(&a.private_read);
                    b.add_write_key(&a.write);
                    b.add_gid(group.gid);
                    added = true;
                    break;
                }
            }
            if added == false {
                for read_key in shared_keys.iter() {
                    if let Some(a) = role.access.unwrap_shared(&read_key)?
                    {
                        // Add access rights to the session                    
                        let b = session.get_or_create_group_role(&role.purpose);
                        b.add_read_key(&a.read);
                        b.add_private_read_key(&a.private_read);
                        b.add_write_key(&a.write);
                        b.add_gid(group.gid);
                        added = true;
                        break;
                    }
                }
            }

            // If we have no successfully gained access to the role then add
            // it to the try again list.
            if added == false {
                next.push(role);
            }
        }

        // If we made no more progress (no more access was granted) then its
        // time to give up
        if next.len() >= start {
            break;
        }
        roles = next;
    }

    Ok(session)
}

pub async fn load_credentials(registry: &Registry, username: String, read_key: EncryptKey, _code: Option<String>, auth: Url) -> Result<AteSessionUser, AteError>
{
    // Prepare for the load operation
    let key = PrimaryKey::from(username.clone());
    let mut session = AteSessionUser::new();
    session.user.add_read_key(&read_key);

    // Generate a chain key that matches this username on the authentication server
    let chain_key = chain_key_4hex(username.as_str(), Some("redo"));
    let chain = registry.open(&auth, &chain_key).await?;

    // Load the user
    let dio = chain.dio(&session).await;
    let user = dio.load::<User>(&key).await?;

    // Build a new session
    let mut session = AteSessionUser::new();
    for access in user.access.iter() {
        session.user.add_read_key(&access.read);
        session.user.add_write_key(&access.write);
    }
    Ok(session)
}