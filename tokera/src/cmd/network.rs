#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use {
    std::process::Command,
    std::process::Stdio,
    std::net::Ipv4Addr,
    tokio::io::AsyncReadExt,
    tokio::io::AsyncWriteExt,
    tokio_tun::TunBuilder,
};
use ate::prelude::*;

use crate::error::*;
use crate::model::NetworkToken;
#[allow(unused_imports)]
use crate::model::HardwareAddress;
use crate::opt::*;
use crate::mio::Port;

use super::*;

pub async fn main_opts_network(
    opts: OptsNetwork,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), InstanceError>
{
    match opts.action
    {
        NetworkAction::List(opts) => {
            let db_url = ate_auth::prelude::origin_url(&opts.db_url, "db");
            main_opts_network_list(opts.purpose, token_path, auth_url, db_url).await
        },
        NetworkAction::Connect(opts) => {
            let db_url = ate_auth::prelude::origin_url(&opts.db_url, "db");
            let net_url = ate_auth::prelude::origin_url(&opts.net_url, "net");
            main_opts_network_connect(opts.purpose, token_path, auth_url, db_url, net_url, opts.export).await
        },
        NetworkAction::Reconnect(opts) => {
            main_opts_network_reconnect(opts.token, token_path).await
        }
        NetworkAction::Disconnect => {
            main_opts_network_disconnect(token_path).await;
            Ok(())
        },
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        NetworkAction::Bridge(opts) => {
            let net_url = opts.net_url.clone().map(|net_url| {
                ate_auth::prelude::origin_url(&Some(net_url), "net")
            });
            main_opts_network_bridge(opts, token_path, net_url).await
        }
    }
}

pub async fn main_opts_network_list(
    opts: OptsNetworkListFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
) -> Result<(), InstanceError>
{
    let context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, Some(&db_url), true).await?;
    let api = context.api;
    
    println!("|-------name-------|");
    let instances = api.instances().await;

    let instances = instances.iter_ext(true, true).await?;
    let instances_ext = {
        let api = api.clone();
        stream! {
            for instance in instances {
                let name = instance.name.clone();
                yield
                (
                    api.instance_chain(instance.name.as_str())
                        .await
                        .map(|chain| (instance, chain)),
                    name,
                )
            }
        }
    };
    pin_mut!(instances_ext);

    while let Some((res, name)) = instances_ext.next().await {
        let (wallet_instance, _) = match res {
            Ok(a) => a,
            Err(err) => {
                debug!("error loading wallet instance - {} - {}", name, err);
                println!(
                    "- {:<16} - {:<19} - {}",
                    name, "error", err
                );
                continue;
            }
        };
        println!(
            "- {:<16}",
            wallet_instance.name
        );
    }
    Ok(())
}

pub async fn main_opts_network_connect(
    opts: OptsNetworkConnectFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
    net_url: url::Url,
    export: bool,
) -> Result<(), InstanceError>
{
    // Get the specifics around the network we will be connecting too
    let network_name = opts.network_name().to_string();
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, Some(&db_url), true).await?;
    let (instance, _) = context.api.instance_action(network_name.as_str()).await?;
    let instance = instance?;
    let chain = instance.chain.clone();
    let access_token = instance.subnet.network_token.clone();

    // Build the access token
    let token = NetworkToken {
        chain: ChainKey::from(chain.clone()),
        network_url: net_url,
        access_token: access_token.clone(),
    };

    // If we are exporting then just throw it out to STDOUT
    if export {
        let token = encode_access_token(&token)?;
        println!("{}", token);
        return Ok(());
    }

    // Save the token
    save_access_token(token_path, &token).await?;
    Ok(())            
}

pub async fn main_opts_network_reconnect(
    token: String,
    token_path: String,
) -> Result<(), InstanceError>
{
    // Decode the token
    let token = decode_access_token(token)?;

    // Save the token
    save_access_token(token_path, &token).await?;
    Ok(())            
}

pub async fn main_opts_network_disconnect(token_path: String)
{
    clear_access_token(token_path).await;
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub async fn main_opts_network_bridge(
    bridge: OptsNetworkBridge,
    token_path: String,
    net_url: Option<url::Url>,
) -> Result<(), InstanceError>
{
    let token_path = if let Ok(t) = std::env::var("NETWORK_TOKEN_PATH") {
        t
    } else {
        shellexpand::tilde(token_path.as_str()).to_string()
    };

    std::env::set_var("NETWORK_TOKEN_PATH", token_path.as_str());
    ::sudo::with_env(&(vec!("NETWORK_TOKEN_PATH")[..])).unwrap();

    if bridge.daemon {
        if let Ok(fork::Fork::Parent(_)) = fork::daemon(true, true) {
            return Ok(())
        }
    }

    let port = load_port(token_path, net_url).await?;
    
    let hw = port.hardware_address()
        .await
        .ok_or_else(|| {
            error!("the hardware address (MAC) on the port has not been set");
            InstanceErrorKind::InternalError(0)
        })?;
    let hw: [u8; 6] = hw.into();
    
    let (ip, netmask) = port.dhcp_acquire()
        .await
        .map_err(|err| {
            let err = format!("failed to acquire IP address - {}", err);
            error!("{}", err);
            InstanceErrorKind::InternalError(0)
        })?;

    let mut socket = port.bind_raw()
        .await
        .map_err(|err| {
            let err = format!("failed to open raw socket - {}", err);
            error!("{}", err);
            InstanceErrorKind::InternalError(0)
        })?;

    let name_id = fastrand::u64(..);
    let name = format!("ate{}", hex::encode(name_id.to_ne_bytes()).to_uppercase());
    let name = &name[..15];

    let tap = TunBuilder::new()
        .name(name)
        .tap(true)
        .packet_info(false)
        .mtu(1500)
        .mac(hw.clone())
        //.up()
        .address(ip)
        .netmask(netmask)
        .broadcast(Ipv4Addr::BROADCAST)
        .try_build()
        .map_err(|err| {
            let err = format!("failed to build tun/tap device - {}", err);
            error!("{}", err);
            InstanceErrorKind::InternalError(0)
        })?;

    let (mut reader, mut writer) = tokio::io::split(tap);

    cmd("ip", &["link", "set", "dev", name, "down"]);
    if bridge.promiscuous {
        cmd("ip", &["link", "set", "dev", name, "promisc", "on"]);    
    }
    let hw = hex::encode(hw.as_slice());
    let hw = format!("{}:{}:{}:{}:{}:{}", &hw[0..2], &hw[2..4], &hw[4..6], &hw[6..8], &hw[8..10], &hw[10..12]);
    let _ = cmd("ip", &["link", "set", "dev", name, "address", hw.as_str()]);
    let _ = cmd("ip", &["link", "set", "dev", name, "up"]);

    loop {
        let mut buf = [0u8; 2048];
        tokio::select! {
            n = reader.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        let buf = (&buf[..n]).to_vec();
                        socket.send(buf).await?;
                    }
                    Err(err) => {
                        error!("TAP device closed - {}", err);
                        break;
                    }
                }
            },
            data = socket.recv() => {
                let data = data?;
                writer.write(&data[..]).await?;
            }
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cmd_inner(cmd: &str, args: &[&str], stderr: Stdio, stdout: Stdio) -> Result<std::process::ExitStatus, std::io::Error> {
    Command::new(cmd)
        .args(args)
        .stderr(stderr)
        .stdout(stdout)
        .spawn()
        .unwrap()
        .wait()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cmd(cmd: &str, args: &[&str]) {
    let ecode = cmd_inner(cmd, args, Stdio::inherit(), Stdio::inherit()).unwrap();
    assert!(ecode.success(), "Failed to execte {}", cmd);
}

pub async fn load_port(token_path: String, net_url: Option<url::Url>) -> Result<Port, InstanceError> {
    let token = load_access_token(token_path).await?;
    let token = match token {
        Some(a) => a,
        None => { return Err(InstanceErrorKind::InvalidAccessToken.into()); }
    };

    let net_url = match net_url {
        Some(n) => n,
        None => token.network_url
    };
    
    let port = Port::new(net_url, token.chain, token.access_token)
        .await
        .map_err(|err| {
            error!("failed to create port - {}", err);
            InstanceErrorKind::InternalError(0)
        })?;

    Ok(port)
}

fn decode_access_token(token: String) -> Result<NetworkToken, SerializationError> {
    let val = token.trim().to_string();
    let bytes = base64::decode(val).unwrap();
    Ok(SerializationFormat::MessagePack.deserialize(&bytes)?)
}

async fn load_access_token(token_path: String) -> Result<Option<NetworkToken>, SerializationError> {
    let token_path = format!("{}.network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();
    if let Ok(token) = tokio::fs::read_to_string(token_path).await {
        Ok(Some(decode_access_token(token)?))
    } else {
        Ok(None)
    }
}

fn encode_access_token(token: &NetworkToken) -> Result<String, SerializationError> {
    let bytes = SerializationFormat::MessagePack.serialize(&token)?;
    let bytes = base64::encode(bytes);
    Ok(bytes)
}

async fn save_access_token(token_path: String, token: &NetworkToken) -> Result<(), SerializationError> {
    let bytes = encode_access_token(token)?;
    
    let token_path = format!("{}.network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();

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
    let save_path = ate_auth::helper::random_file();
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
        file.write_all(bytes.as_bytes())?;
    }

    // Update the token path so that it points to this temporary token
    #[cfg(unix)]
    symlink(save_path, token_path)?;
    Ok(())
}

async fn clear_access_token(token_path: String) {
    let token_path = format!("{}/network", token_path);
    let token_path = shellexpand::tilde(token_path.as_str()).to_string();

    // Remove any old paths
    if let Ok(old) = std::fs::canonicalize(token_path.clone()) {
        let _ = std::fs::remove_file(old);
    }
    let _ = std::fs::remove_file(token_path.clone());
}