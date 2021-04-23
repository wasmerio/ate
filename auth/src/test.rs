#![allow(unused_imports)]
use ate::prelude::*;
use crate::prelude::*;
use url::Url;

#[tokio::main]
#[test]
pub async fn test_create_user_and_group() -> Result<(), AteError>
{
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

    // Login to the user
    let session = crate::main_login(Some(username), Some(password), None, auth.clone()).await?;

    // Gather in a specific group
    let _session = crate::main_gather(Some(group), session, auth.clone()).await?;

    Ok(())
}

#[test]
pub fn test_create_group()
{

}