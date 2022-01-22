use std::ops::Deref;
use std::sync::Arc;
use ate::prelude::*;
use ate::session::AteSessionType;
use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info};
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;

use crate::error::*;
use crate::model::{INSTANCE_ROOT_ID, ServiceInstance, InstanceAction};
use crate::opt::*;
use crate::api::TokApi;

use super::*;

pub async fn main_opts_instance_list(api: &mut TokApi) -> Result<(), InstanceError> {
    println!("|-------name-------|---status---|--action--|---wapm---");
    let instances = api.instances().await;

    let instances = instances.iter().await?;
    let instances_ext = {
        let api = api.clone();
        stream! {
            for instance in instances {
                yield api.instance_chain(instance.name.as_str())
                    .await
                    .map(|chain| (instance, chain));
            }
        }
    };
    pin_mut!(instances_ext);

    let session = api.session();
    while let Some(res) = instances_ext.next().await {
        let (wallet_instance, chain) = res?;
        let dio: Arc<Dio> = chain.dio(session.deref()).await;
        match dio.load::<ServiceInstance>(&PrimaryKey::from(INSTANCE_ROOT_ID)).await {
            Ok(instance) => {
                let action = instance.action.clone().map(|a| a.to_string()).unwrap_or_else(|| "".to_string());
                println!(
                    "- {:<16} - {:<10} - {:<8} - {}",
                    wallet_instance.name, instance.status.to_string(), action, wallet_instance.wapm
                );
            }
            Err(err) => {
                debug!("error loading service chain - {}", err);
                println!(
                    "- {:<16} - {:<10} - {}",
                    wallet_instance.name, "error", err
                );
            }
        }
    }
    Ok(())
}

pub async fn main_opts_instance_details(
    api: &mut TokApi,
    opts: OptsInstanceDetails,
) -> Result<(), InstanceError> {
    let instance = api.instance_find(opts.name.as_str())
        .await;
    let instance = match instance {
        Ok(a) => a,
        Err(InstanceError(InstanceErrorKind::InvalidInstance, _)) => {
            eprintln!("An instance does not exist for this token.");
            std::process::exit(1);
        }
        Err(err) => {
            bail!(err);
        }
    };

    println!("{}", serde_json::to_string_pretty(instance.deref()).unwrap());

    let _chain = api.instance_chain(instance.name.as_str()).await?;
    //println!("{}", serde_json::to_string_pretty(instance.deref()).unwrap());

    Ok(())
}

pub async fn main_opts_instance_create(
    api: &mut TokApi,
    name: Option<String>,
    wapm: String,
    group: Option<String>,
    stateful: bool,
    session: AteSessionType,
    db_url: url::Url,
) -> Result<(), InstanceError> {

    let name = match name {
        Some(a) => a,
        None => {
            ate::crypto::AteHash::generate().to_hex_string()[0..16].to_string()
        }
    };

    if let Err(err) = api.instance_create(name.clone(), wapm.clone(), stateful, group, session, db_url).await {
        bail!(err);
    };
    println!("Instance created ({} with alias {})", wapm, name);

    Ok(())
}

pub async fn main_opts_instance_action(
    api: &mut TokApi,
    name: &str,
    action: InstanceAction,
) -> Result<(), InstanceError> {
    match api
        .instance_action(name, action)
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
    db_url: url::Url,
) -> Result<(), InstanceError>
{
    // Check if sudo is needed
    let needs_sudo = if opts.action().needs_sudo() {
        true
    } else {
        false
    };

    // Perform the action
    let name = opts.action().name();
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, Some(&db_url), needs_sudo).await?;
    
    // Determine what we need to do
    let purpose: &dyn OptsPurpose<OptsInstanceAction> = &opts;
    match purpose.action() {
        OptsInstanceAction::List => {
            main_opts_instance_list(&mut context.api).await?;
        }
        OptsInstanceAction::Details(opts) => {
            main_opts_instance_details(&mut context.api, opts).await?;
        }
        OptsInstanceAction::Create(opts) => {
            let session = context.session.clone();
            main_opts_instance_create(&mut context.api, opts.name, opts.wapm.clone(), purpose.group_name(), opts.stateful, session, db_url).await?;
        }
        OptsInstanceAction::Start(_opts_start) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = name.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), InstanceAction::Start).await?;
        }
        OptsInstanceAction::Stop(_opts_stop) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = name.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), InstanceAction::Stop).await?;
        }
        OptsInstanceAction::Kill(_opts_kill) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = name.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), InstanceAction::Kill).await?;
        }
        OptsInstanceAction::Restart(_opts_restart) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = name.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), InstanceAction::Restart).await?;
        }
        OptsInstanceAction::Clone(_opts_clone) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Exec(_opts_exec) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Stdio(_opts_stdio) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Mount(_opts_mount) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Backup(_opts_backup) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Restore(_opts_restore) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Upgrade(_opts) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let token = name.unwrap();
            main_opts_instance_action(&mut context.api, token.as_str(), InstanceAction::Upgrade).await?;
        }
    }

    Ok(())
}
