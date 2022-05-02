#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::ops::Deref;
use error_chain::bail;
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
#[cfg(feature = "enable_bridge")]
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
use crate::api::TokApi;

use super::*;

pub async fn main_opts_network(
    opts: OptsNetwork,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), InstanceError>
{
    let no_inner_encryption = opts.double_encrypt == false; // the messages are still encrypted inside the stream
    let db_url = ate_auth::prelude::origin_url(&opts.db_url, "db");
    match opts.cmd
    {
        OptsNetworkCommand::For(opts) => {
            let purpose: &dyn OptsPurpose<OptsNetworkAction> = &opts.purpose;
            let mut context = PurposeContext::new(purpose, token_path.as_str(), &auth_url, Some(&db_url), true).await?;
            match context.action.clone() {
                OptsNetworkAction::List => {
                    main_opts_network_list(&mut context.api).await
                },
                OptsNetworkAction::Details(opts) => {
                    main_opts_network_details(&mut context.api, opts.name.as_str()).await
                },
                OptsNetworkAction::Cidr(opts) => {
                    main_opts_network_cidr(&mut context.api, opts.name.as_str(), opts.action).await
                },
                OptsNetworkAction::Peering(opts) => {
                    main_opts_network_peering(&mut context.api, opts.name.as_str(), opts.action).await
                },
                OptsNetworkAction::Reset(opts) => {
                    main_opts_network_reset(&mut context.api, opts.name.as_str()).await
                },
                OptsNetworkAction::Connect(opts) => {
                    let net_url = ate_auth::prelude::origin_url(&opts.net_url, "net");
                    main_opts_network_connect(&mut context.api, opts.name.as_str(), net_url, token_path, opts.export).await
                },
                OptsNetworkAction::Create(opts) => {
                    let mut instance_authority = db_url.domain()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "tokera.sh".to_string());
                    if instance_authority == "localhost" {
                        instance_authority = "tokera.sh".to_string();
                    }
                    main_opts_network_create(&mut context.api, opts.name, purpose.group_name(), db_url, instance_authority, opts.force).await
                },
                OptsNetworkAction::Kill(opts) => {
                    main_opts_network_kill(&mut context.api, opts.name.as_str(), opts.force).await
                },
            }
        },
        OptsNetworkCommand::Reconnect(opts) => {
            main_opts_network_reconnect(opts.token, token_path).await
        },
        OptsNetworkCommand::Disconnect => {
            main_opts_network_disconnect(token_path).await;
            Ok(())
        },
        #[cfg(feature = "enable_bridge")]
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        OptsNetworkCommand::Bridge(opts) => {
            let net_url = opts.net_url.clone().map(|net_url| {
                ate_auth::prelude::origin_url(&Some(net_url), "net")
            });
            main_opts_network_bridge(opts, token_path, net_url, no_inner_encryption).await
        }
    }
}

pub async fn main_opts_network_list(
    api: &mut TokApi,
) -> Result<(), InstanceError>
{
    println!("|-------name-------|-peerings");
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
        let mut peerings = String::new();
        if let Ok(service_instance) = api.instance_load(wallet_instance.deref()).await {
            for peer in service_instance.subnet.peerings.iter() {
                if peerings.len() > 0 { peerings.push_str(","); }
                peerings.push_str(peer.name.as_str());
            }
        }
        println!(
            "- {:<16} - {}",
            wallet_instance.name,
            peerings
        );
    }
    Ok(())
}

pub async fn main_opts_network_details(
    api: &mut TokApi,
    network_name: &str,
) -> Result<(), InstanceError>
{
    let network = api.instance_find(network_name)
        .await;
    let network = match network {
        Ok(a) => a,
        Err(InstanceError(InstanceErrorKind::InvalidInstance, _)) => {
            eprintln!("An network does not exist for this token.");
            std::process::exit(1);
        }
        Err(err) => {
            bail!(err);
        }
    };

    println!("Network");
    println!("{}", serde_json::to_string_pretty(network.deref()).unwrap());

    if let Ok(service_instance) = api.instance_load(network.deref()).await {
        println!("{}", serde_json::to_string_pretty(&service_instance.subnet).unwrap());

        for node in service_instance.mesh_nodes.iter().await? {
            println!("");
            println!("Mesh Node");
            println!("Key: {}", node.key());
            println!("Address: {}", node.node_addr);
            
            if node.switch_ports.len() > 0 {
                println!("Switch Ports:");
                for switch_port in node.switch_ports.iter() {
                    println!("- {}", switch_port);
                }
            }
            if node.dhcp_reservation.len() > 0 {
                println!("DHCP");
                for (mac, ip) in node.dhcp_reservation.iter() {
                    println!("- {} - {},", mac, ip.addr4);
                }
            }
        }
    }

    Ok(())
}

pub async fn main_opts_network_cidr(
    api: &mut TokApi,
    network_name: &str,
    action: OptsCidrAction,
) -> Result<(), InstanceError> {
    let (instance, _) = api.instance_action(network_name).await?;
    let instance = instance?;
    
    main_opts_cidr(instance, action).await?;

    Ok(())
}

pub async fn main_opts_network_peering(
    api: &mut TokApi,
    network_name: &str,
    action: OptsPeeringAction,
) -> Result<(), InstanceError> {
    let (instance, wallet_instance) = api.instance_action(network_name).await?;
    let instance = instance?;
    
    main_opts_peering(api, instance, wallet_instance, action).await?;

    Ok(())
}

pub async fn main_opts_network_reset(
    api: &mut TokApi,
    network_name: &str,
) -> Result<(), InstanceError> {
    main_opts_instance_reset(api, network_name).await
}

pub async fn main_opts_network_connect(
    api: &mut TokApi,
    network_name: &str,
    net_url: url::Url,
    token_path: String,
    export: bool,
) -> Result<(), InstanceError>
{
    // Get the specifics around the network we will be connecting too
    let (instance, _) = api.instance_action(network_name).await?;
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

pub async fn main_opts_network_create(
    api: &mut TokApi,
    network_name: Option<String>,
    group: Option<String>,
    db_url: url::Url,
    instance_authority: String,
    force: bool,
) -> Result<(), InstanceError> {
    main_opts_instance_create(api, network_name, group, db_url, instance_authority, force).await
}

pub async fn main_opts_network_kill(
    api: &mut TokApi,
    network_name: &str,
    force: bool,
) -> Result<(), InstanceError> {
    main_opts_instance_kill(api, network_name, force).await
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

#[cfg(feature = "enable_bridge")]
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub async fn main_opts_network_bridge(
    bridge: OptsNetworkBridge,
    token_path: String,
    net_url: Option<url::Url>,
    no_inner_encryption: bool
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

    let port = load_port(token_path, net_url, no_inner_encryption).await?;
    
    let hw = port.hardware_address()
        .await
        .ok_or_else(|| {
            error!("the hardware address (MAC) on the port has not been set");
            InstanceErrorKind::InternalError(0)
        })?;
    let hw: [u8; 6] = hw.into();
    
    let mtu = bridge.mtu.unwrap_or(1500);

    // The IP address is either staticly defined or we use DHCP
    let (ip4, netmask4) = {
        if let Some(ip4) = bridge.ip4 {
            if let Some(netmask4) = bridge.netmask4 {
                (ip4, netmask4)
            } else {
                (ip4, Ipv4Addr::new(255, 255, 255, 0))
            }
        } else {
            port.dhcp_acquire()
                .await
                .map_err(|err| {
                    let err = format!("failed to acquire IP address - {}", err);
                    error!("{}", err);
                    InstanceErrorKind::InternalError(0)
                })?
        }
    };

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
        .mtu(mtu as i32)
        .mac(hw.clone())
        //.up()
        .address(ip4)
        .netmask(netmask4)
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

#[cfg(feature = "enable_bridge")]
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

#[cfg(feature = "enable_bridge")]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn cmd(cmd: &str, args: &[&str]) {
    let ecode = cmd_inner(cmd, args, Stdio::inherit(), Stdio::inherit()).unwrap();
    assert!(ecode.success(), "Failed to execte {}", cmd);
}

pub async fn load_port(token_path: String, net_url: Option<url::Url>, no_inner_encryption: bool) -> Result<Port, InstanceError> {
    let token = load_access_token(token_path).await?;
    let token = match token {
        Some(a) => a,
        None => { return Err(InstanceErrorKind::InvalidAccessToken.into()); }
    };

    let net_url = match net_url {
        Some(n) => n,
        None => token.network_url
    };
    
    let port = Port::new_ext(net_url, token.chain, token.access_token, no_inner_encryption)
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
    if let Ok(token) = std::fs::read_to_string(token_path) {
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