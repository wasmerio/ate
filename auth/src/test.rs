#![allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use ate::prelude::*;
use crate::prelude::*;
use ate::time::TimeKeeper;
use url::Url;
use std::time::Duration;

use crate::helper::conf_cmd;
use crate::login::login_command;
use crate::login::handle_login_response;

#[tokio::main(flavor = "current_thread")]
#[test]
pub async fn test_create_user_and_group()
{
    ate::utils::bootstrap_test_env();

    // Create the configuration
    #[allow(unused_mut)]
    let mut cfg_ate = crate::conf_auth();
    #[cfg(feature = "enable_local_fs")]
    {
        cfg_ate.log_path = Some(format!("/tmp/ate/test/{}", fastrand::u64(..)));
    }

    // Create the certificate
    let cert = PrivateEncryptKey::generate(KeySize::Bit192);
    Registry::add_global_certificate(&cert.hash());
    
    // Build a session for service
    info!("building session for service");
    let root_read_key = EncryptKey::generate(KeySize::Bit192);
    let root_write_key = PrivateSignKey::generate(KeySize::Bit192);
    let mut session = AteSessionUser::new();
    session.user.add_read_key(&root_read_key);
    session.user.add_write_key(&root_write_key);

    // Create the chain flow and generate configuration
    info!("generating random config");
    let port_offset = fastrand::u16(..1000);
    let port = 5000 + port_offset;
    let auth = Url::parse(format!("ws://localhost:{}/auth", port).as_str()).unwrap();
    let flow = ChainFlow::new(&cfg_ate, root_write_key, session, &auth);

    // Create the server and listen on port 5000
    info!("creating server and listening on ports with routes");
    let mut cfg_mesh = ConfMesh::solo_from_url(&cfg_ate, &auth, &IpAddr::from_str("::1").unwrap(), None).await.unwrap();
    cfg_mesh.wire_protocol = StreamProtocol::WebSocket;
    cfg_mesh.listen_certificate = Some(cert);
    let server = create_server(&cfg_mesh).await.unwrap();
    server.add_route(Box::new(flow), &cfg_ate).await.unwrap();

    // Create the user
    info!("creating user joe.blogs");
    let username = "joe.blogs@nowhere.com".to_string();
    let password = "letmein".to_string();
    let response = crate::main_create_user(
        Some(username.clone()),
        Some(password.clone()),
        auth.clone()).await.unwrap();
    let session = response.authority;

    // Get the read key for the user
    info!("checking we have a read key");
    let _read_key = session.read_keys(AteSessionKeyCategory::AllKeys).next().unwrap().clone();

    // Create the group
    info!("creating group 'mygroup'");
    let group = "mygroup".to_string();
    let _session = crate::main_create_group(Some(group.clone()), auth.clone(), Some(username.clone()), "Group").await.unwrap();

    // Compute the code using the returned QR secret
    info!("computing login code");
    let timer = TimeKeeper::new(&cfg_ate, 30000).await.unwrap();
    let google_auth = google_authenticator::GoogleAuthenticator::new();
    timer.wait_for_high_accuracy().await;
    let code = google_auth.get_code(response.qr_secret.as_str(), timer.current_timestamp_as_duration().unwrap().as_secs() / 30).unwrap();

    // Login lots of times to hammer it
    {
        let registry = ate::mesh::Registry::new( &conf_cmd()).await
            .keep_alive(Duration::from_secs(30))
            .cement();
        for n in 0..10 {
            info!("login request for joe.blogs [n={}]", n);
            let response = login_command(&registry, username.clone(), password.clone(), None, auth.clone(), true).await;
            info!("login completed for joe.blogs [n={}]", n);
            let _ = handle_login_response(&registry, response, username.clone(), password.clone(), auth.clone()).await.unwrap();
        }
    }
            

    // Login to the main user and gather the rights to the group (full sudo rights)
    info!("sudo login for 'joe.blogs'");
    let session = crate::main_login(Some(username.clone()), Some(password.clone()), auth.clone()).await.unwrap();
    let session = crate::main_sudo(session, Some(code), auth.clone()).await.unwrap();
    info!("gather permissions for group 'mygroup'");
    let session = crate::main_gather(Some(group.clone()), session.into(), auth.clone(), "Group").await.unwrap();

    // Make sure its got the permission
    info!("test we have group roles");
    let _group_read = session.get_group_role(&AteRolePurpose::Owner)
        .expect("Should have the owner role")
        .private_read_keys().next().expect("Should have a private key for the owner role");
    let _group_read = session.get_group_role(&AteRolePurpose::Delegate)
        .expect("Should have the delegate role")
        .private_read_keys().next().expect("Should have a private key for the delegate role");

    // Login to the main user and gather the rights to the group (we do not have sudo rights)
    info!("login without sudo 'joe.blogs'");
    let session = crate::main_login(Some(username.clone()), Some(password.clone()), auth.clone()).await.unwrap();
    info!("gather permissions for group 'mygroup'");
    let session = crate::main_gather(Some(group.clone()), session.into(), auth.clone(), "Group").await.unwrap();

    // Make sure its got the permission
    info!("test we at have delegate and not owner");
    let _group_read = session.get_group_role(&AteRolePurpose::Delegate)
        .expect("Should have the delegate role")
        .private_read_keys().next().expect("Should have a private key for the delegate role");
    assert!(session.get_group_role(&AteRolePurpose::Owner).is_none(), "The user should have had this role");

    // Create a friend and add it to the new group we just added
    info!("create a friend account 'myfriend'");
    let friend_username = "myfriend@nowhere.come".to_string();
    let friend = crate::main_create_user(Some(friend_username.clone()), Some(password.clone()), auth.clone()).await.unwrap();
    let friend_session = friend.authority;

    info!("add friend to the group 'mygroup'");
    crate::main_group_user_add(Some(AteRolePurpose::Contributor), Some(friend_username.clone()), auth.clone(), &session, "Group").await.unwrap();

    // Gather the extra rights for the friend
    info!("gather extra rights for friend");
    let friend = crate::main_gather(Some(group.clone()), friend_session.clone().into(), auth.clone(), "Group").await.unwrap();

    // Make sure its got the permission
    info!("test the friend got the 'contributor' role");
    let _group_read = friend.get_group_role(&AteRolePurpose::Contributor)
        .expect("Should have the contributor role")
        .private_read_keys().next().expect("Should have a private key for the owner role");
    assert!(friend.get_group_role(&AteRolePurpose::Owner).is_none(), "The user should have had this role");
    assert!(friend.get_group_role(&AteRolePurpose::Delegate).is_none(), "The user should have had this role");

    // Load the details of the group
    info!("get the group details");
    crate::main_group_details(Some(group.clone()), auth.clone(), Some(&session), "Group").await.unwrap();

    // Remove user the role
    info!("remove the 'friend' from the group");
    crate::main_group_user_remove(Some(AteRolePurpose::Contributor), Some(friend_username.clone()), auth.clone(), &session, "Group").await.unwrap();

    // Make sure its got the permission
    info!("gather permissions and make sure they dont have contributor anymore");
    let friend = crate::main_gather(Some(group.clone()), friend_session.clone().into(), auth.clone(), "Group").await.unwrap();
    assert!(friend.get_group_role(&AteRolePurpose::Contributor).is_none(), "The user should have had this role removed");
}