use std::ops::Deref;
use std::sync::Arc;
use ate::prelude::*;
use ate::session::AteSessionType;
use chrono::NaiveDateTime;
use error_chain::bail;
#[allow(unused_imports)]
use tracing::{debug, error, info};
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;

use crate::error::*;
use crate::model::{INSTANCE_ROOT_ID, ServiceInstance, HistoricActivity, activities};
use crate::opt::*;
use crate::api::TokApi;

use super::*;

pub async fn main_opts_instance_list(api: &mut TokApi) -> Result<(), InstanceError> {
    println!("|-------name-------|-------created-------|-exports");
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
                let secs = instance.when_created() / 1000;
                let nsecs = (instance.when_created() % 1000) * 1000 * 1000;
                let when = NaiveDateTime::from_timestamp(secs as i64, nsecs as u32);
                let mut exports = String::new();
                for export in instance.exports.iter().await? {
                    if exports.len() > 0 { exports.push_str(","); }
                    exports.push_str(export.binary.as_str());
                    if export.distributed {
                        exports.push_str("*");
                    }
                }
                println!(
                    "- {:<16} - {:<16} - {}",
                    wallet_instance.name, when.format("%Y-%m-%d %H:%M:%S"), exports
                );
            }
            Err(err) => {
                debug!("error loading service chain - {}", err);
                println!(
                    "- {:<16} - {:<16} - {}",
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
    group: Option<String>,
    session: AteSessionType,
    db_url: url::Url,
) -> Result<(), InstanceError> {

    let name = match name {
        Some(a) => a,
        None => {
            ate::crypto::AteHash::generate().to_hex_string()[0..16].to_string()
        }
    };

    if let Err(err) = api.instance_create(name.clone(), group, session, db_url).await {
        bail!(err);
    };
    println!("Instance created ({})", name);

    Ok(())
}

pub async fn main_opts_instance_kill(
    api: &mut TokApi,
    name: &str,
) -> Result<(), InstanceError> {
    let (chain, instance) = api.instance_action(name).await?;

    let name = instance.name.clone();

    debug!("deleting all the roots in the chain");
    chain.delete_all_roots().await?;
    chain.commit().await?;
    drop(chain);

    // Now add the history
    if let Err(err) = api
        .record_activity(HistoricActivity::InstanceDestroyed(
            activities::InstanceDestroyed {
                when: chrono::offset::Utc::now(),
                by: api.user_identity(),
                alias: Some(name.clone()),
            },
        ))
        .await
    {
        error!("Error writing activity: {}", err);
    }

    debug!("deleting the instance from the user/group");
    let _ = instance.delete()?;
    api.dio.commit().await?;

    println!("Instance ({}) has been killed", name);
    Ok(())
}

pub async fn main_opts_instance(
    opts: OptsInstanceFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
    _sess_url: url::Url,
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
            main_opts_instance_create(&mut context.api, opts.name, purpose.group_name(), session, db_url).await?;
        }
        OptsInstanceAction::Kill(_opts_kill) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_kill(&mut context.api, name.as_str()).await?;
        }
        OptsInstanceAction::Shell(_opts_exec) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Export(_opts_export) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Deport(_opts_deport) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Clone(_opts_clone) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
        OptsInstanceAction::Mount(_opts_mount) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            bail!(InstanceErrorKind::Unsupported);
        }
    }

    Ok(())
}
