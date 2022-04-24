use ate::prelude::*;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::{ServiceInstance, IpCidr};
use crate::opt::*;

pub async fn main_opts_cidr_list(
    instance: DaoMut<ServiceInstance>,
) -> Result<(), InstanceError> {
    println!("|-------cidr-------|");
    for cidr in instance.subnet.cidrs.iter() {
        println!("{}/{}", cidr.ip, cidr.prefix);
    }
    
    Ok(())
}

pub async fn main_opts_cidr_add(
    mut instance: DaoMut<ServiceInstance>,
    opts: OptsCidrAdd
) -> Result<(), InstanceError> {
    let dio = instance.dio_mut();

    {
        let mut instance = instance.as_mut();
        instance.subnet.cidrs.push(
            IpCidr {
                ip: opts.ip,
                prefix: opts.prefix,
            }
        )
    }
    dio.commit().await?;

    Ok(())
}

pub async fn main_opts_cidr_remove(
    mut instance: DaoMut<ServiceInstance>,
    opts: OptsCidrRemove
) -> Result<(), InstanceError> {

    let dio = instance.dio_mut();

    {
        let mut instance = instance.as_mut();
        instance.subnet.cidrs.retain(|cidr| cidr.ip != opts.ip);
    }
    dio.commit().await?;

    Ok(())
}

pub async fn main_opts_cidr(
    instance: DaoMut<ServiceInstance>,
    action: OptsCidrAction,
) -> Result<(), InstanceError>
{
    // Determine what we need to do
    match action {
        OptsCidrAction::List => {
            main_opts_cidr_list(instance).await?;
        }
        OptsCidrAction::Add(add) => {
            main_opts_cidr_add(instance, add).await?;
        }
        OptsCidrAction::Remove(remove) => {
            main_opts_cidr_remove(instance, remove).await?;
        }
    }

    Ok(())
}
