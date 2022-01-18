use ate_auth::helper::*;
use error_chain::bail;
use std::time::Duration;
#[allow(unused_imports)]
use tracing::{debug, error, info};

use crate::error::*;
use crate::opt::*;
use crate::api::TokApi;
use crate::request::*;

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

pub async fn main_opts_instance_create(
    api: &mut TokApi,
    wapm: String,
    stateful: bool,
) -> Result<(), InstanceError> {
    let identity = api.session_identity();
    let ret = match api.instance_create(wapm, stateful, identity).await {
        Ok(a) => a,
        Err(err) => {
            bail!(err);
        }
    };
    println!("Instance created ({})", ret.token);

    Ok(())
}

pub async fn main_opts_instance_action(
    api: &mut TokApi,
    token: &str,
    identity: &str,
    action: InstanceAction,
) -> Result<(), InstanceError> {
    match api
        .instance_action(token, identity, identity, action)
        .await
    {
        Ok(a) => a,
        Err(InstanceError(InstanceErrorKind::InvalidInstance, _)) => {
            eprintln!("No instance exists with this token.");
            std::process::exit(1);
        }
        Err(err) => return Err(err),
    };

    Ok(())
}

pub async fn main_opts_instance(
    opts: OptsInstanceFor,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), InstanceError>
{
    // Check if sudo is needed
    let needs_sudo = if opts.action().needs_sudo() {
        true
    } else {
        false
    };

    // Perform the action
    let token = opts.action().token();
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, needs_sudo).await?;
    let identity = context.identity.clone();    

    // Determine what we need to do
    let purpose: &dyn OptsPurpose<OptsInstanceAction> = &opts;
    match purpose.action() {
        OptsInstanceAction::List => {
            main_opts_instance_list(&auth_url).await?;
        }
        OptsInstanceAction::Details(opts) => {
            main_opts_instance_details(opts, &auth_url).await?;
        }
        OptsInstanceAction::Create(opts) => {
            main_opts_instance_create(&mut context.api, opts.wapm.clone(), opts.stateful).await?;
        }
        OptsInstanceAction::Start(_opts_start) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Start).await?;
        }
        OptsInstanceAction::Stop(_opts_stop) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Stop).await?;
        }
        OptsInstanceAction::Kill(_opts_kill) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Kill).await?;
        }
        OptsInstanceAction::Restart(_opts_restart) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Restart).await?;
        }
        OptsInstanceAction::Clone(_opts_clone) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Clone).await?;
        }
        OptsInstanceAction::Backup(opts_backup) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Backup {
                chain: opts_backup.chain,
                path: opts_backup.path
            }).await?;
        }
        OptsInstanceAction::Restore(opts_restore) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Restore {
                chain: opts_restore.chain,
                path: opts_restore.path
            }).await?;
        }
        OptsInstanceAction::Upgrade(_opts) => {
            if token.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = token.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), identity.as_str(), InstanceAction::Upgrade).await?;
        }
    }

    Ok(())
}
