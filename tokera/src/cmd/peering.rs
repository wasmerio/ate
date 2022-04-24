use ate::prelude::*;
use crate::api::TokApi;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::{ServiceInstance, InstancePeering, WalletInstance};
use crate::opt::*;

pub async fn main_opts_peering_list(
    instance: DaoMut<ServiceInstance>,
) -> Result<(), InstanceError> {
    println!("|---network peerings---|");
    for peer in instance.subnet.peerings.iter() {
        println!("{}", peer.name);
    }
    
    Ok(())
}

pub async fn main_opts_peering_add(
    api: &mut TokApi,
    mut instance: DaoMut<ServiceInstance>,
    wallet_instance: DaoMut<WalletInstance>,
    opts: OptsPeeringAdd
) -> Result<(), InstanceError> {
    let (peer, peer_wallet) = api.instance_action(opts.peer.as_str()).await?;
    let mut peer = peer?;

    {
        let dio = instance.dio_mut();
        if instance.subnet.peerings.iter().any(|p| p.chain.name == peer.chain) == false {
            let mut instance = instance.as_mut();
            instance.subnet.peerings.push(
                InstancePeering {
                    id: peer.id,
                    name: peer_wallet.name.clone(),
                    chain: ChainKey::from(peer.chain.clone()),
                    access_token: peer.subnet.network_token.clone(),
                }
            );
        }
        dio.commit().await?;
    }
    {
        let dio = peer.dio_mut();
        if peer.subnet.peerings.iter().any(|p| p.chain.name == instance.chain) == false {
            let mut peer = peer.as_mut();
            peer.subnet.peerings.push(
                InstancePeering {
                    id: instance.id,
                    name: wallet_instance.name.clone(),
                    chain: ChainKey::from(instance.chain.clone()),
                    access_token: instance.subnet.network_token.clone(),
                }
            );
        }
        dio.commit().await?;
    }
    

    Ok(())
}

pub async fn main_opts_peering_remove(
    api: &mut TokApi,
    mut instance: DaoMut<ServiceInstance>,
    opts: OptsPeeringRemove
) -> Result<(), InstanceError> {
    let (peer, _) = api.instance_action(opts.peer.as_str()).await?;
    let mut peer = peer?;

    {
        let dio = instance.dio_mut();
        let mut instance = instance.as_mut();
        instance.subnet.peerings.retain(|p| p.chain.name != peer.chain);
        drop(instance);
        dio.commit().await?;
    }
    {
        let dio = peer.dio_mut();
        let mut peer = peer.as_mut();
        peer.subnet.peerings.retain(|p| p.chain.name != instance.chain);
        drop(peer);
        dio.commit().await?;
    }

    Ok(())
}

pub async fn main_opts_peering(
    api: &mut TokApi,
    instance: DaoMut<ServiceInstance>,
    wallet_instance: DaoMut<WalletInstance>,
    action: OptsPeeringAction,
) -> Result<(), InstanceError>
{
    // Determine what we need to do
    match action {
        OptsPeeringAction::List => {
            main_opts_peering_list(instance).await?;
        }
        OptsPeeringAction::Add(add) => {
            main_opts_peering_add(api, instance, wallet_instance, add).await?;
        }
        OptsPeeringAction::Remove(remove) => {
            main_opts_peering_remove(api, instance, remove).await?;
        }
    }

    Ok(())
}
