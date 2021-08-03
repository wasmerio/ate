#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::prelude::*;
use crate::prelude::*;
use ate::time::TimeKeeper;
use url::Url;

#[tokio::main(flavor = "current_thread")]
#[test]
pub async fn test_create_user_and_group() -> Result<(), AteError>
{
    ate::utils::bootstrap_test_env();

    // Create the configuration
    #[allow(unused_mut)]
    let mut cfg_ate = crate::conf_auth();
    #[cfg(feature = "enable_local_fs")]
    {
        cfg_ate.log_path = Some(format!("/tmp/ate/test/{}", fastrand::u64(..)));
    }
    
    // Build a session for service
    let root_read_key = EncryptKey::generate(KeySize::Bit256);
    let root_write_key = PrivateSignKey::generate(KeySize::Bit256);
    let mut session = AteSession::new(&cfg_ate);
    session.user.add_read_key(&root_read_key);
    session.user.add_write_key(&root_write_key);

    // Create the chain flow and generate configuration
    let port_offset = fastrand::u16(..1000);
    let port = 5000 + port_offset;
    let auth = Url::parse(format!("ws://localhost:{}/auth", port).as_str()).unwrap();
    let flow = ChainFlow::new(&cfg_ate, root_write_key, session, &auth);

    // Create the server and listen on port 5000
    let cfg_mesh = ConfMesh::solo_from_url(&cfg_ate, &auth, &IpAddr::from_str("::1").unwrap(), None).await?;
    let server = create_server(&cfg_mesh).await?;
    server.add_route(Box::new(flow), &cfg_ate).await?;

    // Create the user
    let username = "joe.blogs@nowhere.com".to_string();
    let password = "letmein".to_string();
    let response = crate::main_create_user(
        Some(username.clone()),
        Some(password.clone()),
        auth.clone()).await?;
    let session = response.authority;

    // Get the read key for the user
    let _read_key = session.read_keys().next().unwrap().clone();

    // Create the group
    let group = "mygroup".to_string();
    let _session = crate::main_create_group(Some(group.clone()), auth.clone(), Some(username.clone())).await?;

    // Compute the code using the returned QR secret
    let timer = TimeKeeper::new(&cfg_ate, 30000).await?;
    let google_auth = google_authenticator::GoogleAuthenticator::new();
    let code = google_auth.get_code(response.qr_secret.as_str(), timer.current_timestamp_as_duration()?.as_secs() / 30).unwrap();

    // Login to the main user and gather the rights to the group (full sudo rights)
    let session = crate::main_sudo(Some(username.clone()), Some(password.clone()), Some(code), auth.clone()).await?;
    let session = crate::main_gather(Some(group.clone()), session, auth.clone()).await?;

    // Make sure its got the permission
    let _group_read = session.get_group_role(&group, &AteRolePurpose::Owner)
        .expect("Should have the owner role")
        .private_read_keys().next().expect("Should have a private key for the owner role");
    let _group_read = session.get_group_role(&group, &AteRolePurpose::Delegate)
        .expect("Should have the delegate role")
        .private_read_keys().next().expect("Should have a private key for the delegate role");

    // Login to the main user and gather the rights to the group (we do not have sudo rights)
    let session = crate::main_login(Some(username.clone()), Some(password.clone()), auth.clone()).await?;
    let session = crate::main_gather(Some(group.clone()), session, auth.clone()).await?;

    // Make sure its got the permission
    let _group_read = session.get_group_role(&group, &AteRolePurpose::Delegate)
        .expect("Should have the delegate role")
        .private_read_keys().next().expect("Should have a private key for the delegate role");

    // Create a friend and add it to the new group we just added
    let friend_username = "myfriend@nowhere.come".to_string();
    let friend = crate::main_create_user(Some(friend_username.clone()), Some(password.clone()), auth.clone()).await?;
    let friend_session = friend.authority;

    crate::main_group_user_add(Some(group.clone()), Some(AteRolePurpose::Contributor), Some(friend_username.clone()), auth.clone(), &session).await?;

    // Gather the extra rights for the friend
    let friend = crate::main_gather(Some(group.clone()), friend_session.clone(), auth.clone()).await?;

    // Make sure its got the permission
    let _group_read = friend.get_group_role(&group, &AteRolePurpose::Contributor)
        .expect("Should have the contributor role")
        .private_read_keys().next().expect("Should have a private key for the owner role");

    // Load the details of the group
    crate::main_group_details(Some(group.clone()), auth.clone(), Some(&session)).await?;

    // Remove user the role
    crate::main_group_user_remove(Some(group.clone()), Some(AteRolePurpose::Contributor), Some(friend_username.clone()), auth.clone(), &session).await?;

    // Make sure its got the permission
    let friend = crate::main_gather(Some(group.clone()), friend_session.clone(), auth.clone()).await?;
    assert!(friend.get_group_role(&group, &AteRolePurpose::Contributor).is_none(), "The user should have had this role removed");
    
    Ok(())
}