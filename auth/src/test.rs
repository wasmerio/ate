#![allow(unused_imports)]
use log::{info, error, debug};
use ate::prelude::*;
use crate::prelude::*;
use url::Url;

#[tokio::main]
#[test]
pub async fn test_create_user_and_group() -> Result<(), AteError>
{
    ate::utils::bootstrap_env();

    // Create the configuration
    let mut cfg_ate = crate::conf_auth();
    cfg_ate.log_path = format!("/tmp/ate/test/{}", fastrand::u64(..));

    // Build a session for service
    let root_read_key = EncryptKey::generate(KeySize::Bit256);
    let root_write_key = PrivateSignKey::generate(KeySize::Bit256);
    let mut session = AteSession::new(&cfg_ate);
    session.user.add_read_key(&root_read_key);
    session.user.add_write_key(&root_write_key);

    // Create the chain flow and generate configuration
    let port_offset = fastrand::u16(..1000);
    let flow = ChainFlow::new(&cfg_ate, root_write_key, session);

    // Create the server and listen on port 5000
    let port = 5000 + port_offset;
    let cfg_mesh = ConfMesh::solo("127.0.0.1", port);
    let _server = create_server(&cfg_ate, &cfg_mesh, Box::new(flow)).await;

    // Create the user
    let username = "joe.blogs@nowhere.com".to_string();
    let password = "letmein".to_string();
    let auth = Url::parse(format!("tcp://127.0.0.1:{}/auth", port).as_str()).unwrap();
    let session = crate::main_create_user(
        Some(username.clone()),
        Some(password.clone()),
        auth.clone()).await?;

    // Get the read key for the user
    let _read_key = session.read_keys().next().unwrap().clone();

    // Create the group
    let group = "mygroup".to_string();
    let _session = crate::main_create_group(Some(group.clone()), auth.clone(), &session).await?;

    // Login to the main user and gather the rights to the group
    let session = crate::main_login(Some(username.clone()), Some(password.clone()), None, auth.clone()).await?;
    let session = crate::main_gather(Some(group.clone()), session, auth.clone()).await?;

    // Make sure its got the permission
    let _group_read = session.get_group_role(&group, &AteRolePurpose::Owner)
        .expect("Should have the owner role")
        .private_read_keys()
        .next()
        .expect("Should have a private key for the owner role");

    // Create a friend and add it to the new group we just added
    let friend_username = "myfriend@nowhere.come".to_string();
    let friend = crate::main_create_user(Some(friend_username.clone()), Some(password.clone()), auth.clone()).await?;

    crate::main_group_add(Some(group.clone()), Some(friend_username.clone()), Some(AteRolePurpose::Contributor), auth.clone(), &session).await?;

    // Gather the extra rights for the friend
    let friend = crate::main_gather(Some(group.clone()), friend, auth.clone()).await?;

    // Make sure its got the permission
    let _group_read = friend.get_group_role(&group, &AteRolePurpose::Contributor)
        .expect("Should have the contributor role")
        .private_read_keys()
        .next()
        .expect("Should have a private key for the owner role");
    
    Ok(())
}