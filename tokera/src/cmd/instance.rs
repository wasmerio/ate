use ate_auth::helper::*;
use error_chain::bail;
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info};

use crate::error::*;
use crate::opt::*;

use super::*;

pub async fn main_opts_instance_list(auth_url: &url::Url) -> Result<(), CoreError> {
    println!("|------token-------|---status---|---wapm---");
    let registry = ate::mesh::Registry::new(&conf_cmd())
        .await
        .keep_alive(Duration::from_secs(10))
        .cement();
    let instances = instance_find_command(&registry, None, auth_url.clone()).await?;
    for instance in instances.instances {
        println!(
            "- {:16} - {:10} - {}",
            instance.token, instance.status, instance.wapm
        );
    }
    Ok(())
}

pub async fn main_opts_instance_details(
    opts: OptsInstanceDetails,
    auth_url: &url::Url,
) -> Result<(), CoreError> {
    let registry = ate::mesh::Registry::new(&conf_cmd())
        .await
        .keep_alive(Duration::from_secs(10))
        .cement();
    let instances = instance_find_command(&registry, None, auth_url.clone()).await?;

    let token = opts.token.to_lowercase();
    let instance = match get_service_instance(&instances, &token)? {
        Some(a) => a,
        None => {
            eprintln!("An instance does not exist for this token.");
            std::process::exit(1);
        }
    };

    println!("{}", serde_json::to_string_pretty(instance).unwrap());

    Ok(())
}

pub async fn main_opts_instance(
    opts: OptsInstanceFor,
    _token_path: String,
    auth_url: url::Url,
) -> Result<(), InstanceError> {
    // Determine what we need to do
    let purpose: &dyn OptsPurpose<OptsInstanceAction> = &opts;
    match purpose.action() {
        OptsInstanceAction::List => {
            main_opts_instance_list(&auth_url).await?;
        }
        OptsInstanceAction::Details(opts) => {
            main_opts_instance_details(opts, &auth_url).await?;
        }

        OptsInstanceAction::Start(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Stop(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Restart(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Backup(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Restore(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Upgrade(_opts) => {
            bail!(InstanceErrorKind::Unsupported);
        }
    }

    Ok(())
}
