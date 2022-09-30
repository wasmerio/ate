#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use url::Url;
use std::io::Write;

#[cfg(unix)]
use std::env::temp_dir;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ::ate::crypto::EncryptKey;
use ::ate::prelude::*;

use crate::model::*;

pub(crate) fn compute_user_auth(user: &User) -> AteSessionUser {
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

pub(crate) fn compute_sudo_auth(sudo: &Sudo, session: AteSessionUser) -> AteSessionSudo {
    let mut role = AteGroupRole {
        purpose: AteRolePurpose::Owner,
        properties: Vec::new(),
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
        sudo: role,
    }
}

pub(crate) fn complete_group_auth(
    group: &Group,
    inner: AteSessionInner,
) -> Result<AteSessionGroup, LoadError> {
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
        let shared_keys = session
            .read_keys(AteSessionKeyCategory::AllKeys)
            .map(|a| a.clone())
            .collect::<Vec<_>>();
        let super_keys = session
            .private_read_keys(AteSessionKeyCategory::AllKeys)
            .map(|a| a.clone())
            .collect::<Vec<_>>();
        for role in roles.into_iter() {
            // Attempt to gain access to the role using the access rights of the super session
            let mut added = false;
            for read_key in super_keys.iter() {
                if let Some(a) = role.access.unwrap(&read_key)? {
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
                    if let Some(a) = role.access.unwrap_shared(&read_key)? {
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

pub async fn load_credentials(
    registry: &Registry,
    username: String,
    read_key: EncryptKey,
    _code: Option<String>,
    auth: Url,
) -> Result<AteSessionUser, AteError> {
    // Prepare for the load operation
    let key = PrimaryKey::from(username.clone());
    let mut session = AteSessionUser::new();
    session.user.add_read_key(&read_key);

    // Generate a chain key that matches this username on the authentication server
    let chain_key = chain_key_4hex(username.as_str(), Some("redo"));
    let chain = registry.open(&auth, &chain_key, true).await?;

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

pub fn save_token(
    token: String,
    token_path: String,
) -> Result<(), AteError> {
    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());

    // Create the folder structure
    let path = std::path::Path::new(&token_path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());

    // Create a random file that will hold the token
    #[cfg(unix)]
    let save_path = random_file();
    #[cfg(not(unix))]
    let save_path = token_path;

    {
        // Create the folder structure
        let path = std::path::Path::new(&save_path);
        let _ = std::fs::create_dir_all(path.parent().unwrap().clone());

        // Create the file
        let mut file = std::fs::File::create(save_path.clone())?;

        // Set the permissions so no one else can read it but the current user
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(save_path.clone())?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(save_path.clone(), perms)?;
        }

        // Write the token to it
        file.write_all(token.as_bytes())?;
    }

    // Update the token path so that it points to this temporary token
    #[cfg(unix)]
    symlink(save_path, token_path)?;
    Ok(())
}

#[cfg(unix)]
pub fn random_file() -> String {
    let mut tmp = temp_dir();

    let rnd = ate::prelude::PrimaryKey::default().as_hex_string();

    let file_name = format!("{}", rnd);
    tmp.push(file_name);

    let tmp_str = tmp.into_os_string().into_string().unwrap();
    let tmp_str = shellexpand::tilde(&tmp_str).to_string();

    tmp_str
}