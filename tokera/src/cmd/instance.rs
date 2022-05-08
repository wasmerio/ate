use std::ops::Deref;
use std::io::Read;
use ate::prelude::*;
use chrono::NaiveDateTime;
use error_chain::bail;
use async_stream::stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use ate_comms::StreamSecurity;

use crate::error::*;
use crate::model::{HistoricActivity, activities, InstanceHello, InstanceCommand, InstanceExport, InstanceCall};
use crate::opt::*;
use crate::api::{TokApi, InstanceClient};

use super::*;

pub async fn main_opts_instance_list(api: &mut TokApi) -> Result<(), InstanceError> {
    println!("|-------name-------|-------created-------|-exports");
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
        match api.instance_load(&wallet_instance).await {
            Ok(instance) => {
                let secs = instance.when_created() / 1000;
                let nsecs = (instance.when_created() % 1000) * 1000 * 1000;
                let when = NaiveDateTime::from_timestamp(secs as i64, nsecs as u32);
                let mut exports = String::new();
                for export in instance.exports.iter().await? {
                    if exports.len() > 0 { exports.push_str(","); }
                    exports.push_str(export.binary.as_str());
                    if export.distributed == false {
                        exports.push_str("*");
                    }
                }
                println!(
                    "- {:<16} - {:<19} - {}",
                    wallet_instance.name, when.format("%Y-%m-%d %H:%M:%S"), exports
                );
            }
            Err(err) => {
                debug!("error loading service chain - {}", err);
                println!(
                    "- {:<16} - {:<19} - {}",
                    wallet_instance.name, "error", err
                );
            }
        }
    }
    Ok(())
}

pub async fn main_opts_instance_details(
    api: &mut TokApi,
    inst_url: url::Url,
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

    println!("Instance");
    println!("{}", serde_json::to_string_pretty(instance.deref()).unwrap());

    if let Ok(service_instance) = api.instance_load(instance.deref()).await {
        println!("{}", serde_json::to_string_pretty(&service_instance.subnet).unwrap());

        if service_instance.exports.len().await? > 0 {
            let id = service_instance.id_str();
            let chain = ChainKey::from(service_instance.chain.clone());
            println!("ID: {}", id);
            println!("");
            println!("Exports");
            for export in service_instance.exports.iter().await? {
                let url = compute_export_url(&inst_url, &chain, export.binary.as_str());
                println!("POST {}", url);
                println!("{}", serde_json::to_string_pretty(export.deref()).unwrap());
            }
        }
    }

    Ok(())
}

pub async fn main_opts_instance_create(
    api: &mut TokApi,
    name: Option<String>,
    group: Option<String>,
    db_url: url::Url,
    instance_authority: String,
    force: bool,
) -> Result<(), InstanceError> {

    let name = match name {
        Some(a) => a,
        None => {
            ate::crypto::AteHash::generate().to_hex_string()[0..16].to_string()
        }
    };

    if let Err(err) = api.instance_create(name.clone(), group, db_url, instance_authority, force).await {
        bail!(err);
    };

    println!("Instance created ({})", name);
    Ok(())
}

pub async fn main_opts_instance_kill(
    api: &mut TokApi,
    name: &str,
    force: bool,
) -> Result<(), InstanceError> {
    let (service_instance, wallet_instance) = api.instance_action(name).await?;

    let name = match service_instance {
        Ok(service_instance) => {
            let dio = service_instance.dio_mut();
            let name = service_instance.id_str();
            debug!("deleting all the roots in the chain");
            dio.delete_all_roots().await?;
            dio.commit().await?;
            drop(dio);
            name
        }
        Err(err) if force => {
            warn!("failed to read service instance data - forcing through - {}", err);
            name.to_string()
        }
        Err(err) => {
            bail!(err);
        }
    };

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
    let _ = wallet_instance.delete()?;
    api.dio.commit().await?;

    println!("Instance ({}) has been killed", name);
    Ok(())
}

pub async fn main_opts_instance_shell(
    api: &mut TokApi,
    inst_url: url::Url,
    name: &str,
    security: StreamSecurity
) -> Result<(), InstanceError> {
    let (instance, _) = api.instance_action(name).await?;
    let instance = instance?;
    let mut client = InstanceClient::new_ext(inst_url, InstanceClient::PATH_INST, security).await
        .unwrap();

    client.send_hello(InstanceHello {
        access_token: instance.admin_token.clone(),
        chain: ChainKey::from(instance.chain.clone()),
    }).await.unwrap();

    client.send_cmd(InstanceCommand::Shell)
        .await.unwrap();

    client.run_shell()
        .await
        .map_err(|err| {
            InstanceErrorKind::InternalError(ate::utils::obscure_error_str(err.to_string().as_str()))
        })?;
        
    Ok(())
}

pub async fn main_opts_instance_call(
    api: &mut TokApi,
    inst_url: url::Url,
    name: &str,
    binary: &str,
    topic: &str,
    security: StreamSecurity
) -> Result<(), InstanceError>
{
    // Read the object into stdin
    let mut request = Vec::new();
    std::io::stdin()
        .lock()
        .read_to_end(&mut request)
        .map_err(|_| InstanceErrorKind::NoInput)?;

    let (instance, _) = api.instance_action(name).await?;
    let instance = instance?;
    let mut client = InstanceClient::new_ext(inst_url, InstanceClient::PATH_INST, security).await
        .unwrap();

    // Search for an export that matches this binary
    let export = instance.exports
        .iter()
        .await?
        .filter(|e| e.binary.eq_ignore_ascii_case(binary))
        .next()
        .ok_or_else(|| InstanceErrorKind::NotExported)?;

    client.send_hello(InstanceHello {
        access_token: export.access_token.clone(),
        chain: ChainKey::from(instance.chain.clone()),
    }).await.unwrap();

    client.send_cmd(InstanceCommand::Call(InstanceCall {
        parent: None,
        handle: fastrand::u32(..),
        binary: binary.to_string(),
        topic: topic.to_string(),
        keepalive: false,
    })).await.unwrap();

    client.send_data(request).await.unwrap();

    client.run_read()
        .await
        .map_err(|err| {
            InstanceErrorKind::InternalError(ate::utils::obscure_error_str(err.to_string().as_str()))
        })?;
        
    Ok(())
}

pub async fn main_opts_instance_export(
    api: &mut TokApi,
    inst_url: url::Url,
    name: &str,
    binary: &str,
    pinned: bool,
    no_http: bool,
    no_https: bool,
    no_bus: bool,
) -> Result<(), InstanceError> {
    let (service_instance, _wallet_instance) = api.instance_action(name).await?;

    let access_token = AteHash::generate().to_hex_string();
    let (chain, id_str) = match service_instance {
        Ok(mut service_instance) => {
            let dio = service_instance.dio_mut();
            let chain = service_instance.chain.clone();
            let id_str = service_instance.id_str();
            service_instance.as_mut().exports.push(InstanceExport {
                access_token: access_token.clone(),
                binary: binary.to_string(),
                distributed: pinned == false,
                http: no_http == false,
                https: no_https == false,
                bus: no_bus == false,
                pinned: None,
            })?;
            dio.commit().await?;
            drop(dio);
            (chain, id_str)
        }
        Err(err) => {
            bail!(err);
        }
    };
    let chain = ChainKey::from(chain);

    // Build the URL that can be used to access this binary
    let url = compute_export_url(&inst_url, &chain, binary);

    // Now add the history
    if let Err(err) = api
        .record_activity(HistoricActivity::InstanceExported(
            activities::InstanceExported {
                when: chrono::offset::Utc::now(),
                by: api.user_identity(),
                alias: Some(id_str.to_string()),
                binary: binary.to_string(),
            },
        ))
        .await
    {
        error!("Error writing activity: {}", err);
    }
    api.dio.commit().await?;

    println!("Instance ({}) has exported binary ({})", id_str, binary);
    println!("Authorization: {}", access_token);
    println!("POST: {}arg0/arg1/...", url);
    println!("PUT: {}[request]", url);
    Ok(())
}

fn compute_export_url(inst_url: &url::Url, chain: &ChainKey, binary: &str) -> String
{
    // Build the URL that can be used to access this binary
    let domain = inst_url.domain().unwrap_or_else(|| "localhost");
    let url = format!("https://{}{}/{}/{}/", domain, inst_url.path(), chain.to_string(), binary);
    url
}

pub async fn main_opts_instance_deport(
    api: &mut TokApi,
    name: &str,
    access_token: &str,
) -> Result<(), InstanceError> {

    let (service_instance, _wallet_instance) = api.instance_action(name).await?;

    let (id, binary) = match service_instance {
        Ok(mut service_instance) => {
            let dio = service_instance.dio_mut();
            let id = service_instance.id_str();

            let export = service_instance.as_mut().exports.iter_mut().await?
                .filter(|e| e.access_token.eq_ignore_ascii_case(access_token))
                .next()
                .ok_or(InstanceErrorKind::InvalidAccessToken)?;

            let binary = export.binary.clone();

            export.delete()?;                
            dio.commit().await?;
            drop(dio);
            (id, binary)
        }
        Err(err) => {
            bail!(err);
        }
    };

    // Now add the history
    if let Err(err) = api
        .record_activity(HistoricActivity::InstanceDeported(
            activities::InstanceDeported {
                when: chrono::offset::Utc::now(),
                by: api.user_identity(),
                alias: Some(id.clone()),
                binary: binary.to_string(),
            },
        ))
        .await
    {
        error!("Error writing activity: {}", err);
    }
    api.dio.commit().await?;

    println!("Instance ({}) has deported binary ({})", id, binary);
    Ok(())
}

pub async fn main_opts_instance_clone(
    _api: &mut TokApi,
    _name: &str,
) -> Result<(), InstanceError> {

    Err(InstanceErrorKind::Unsupported.into())
}

pub async fn main_opts_instance_mount(
    _api: &mut TokApi,
    _name: &str,
) -> Result<(), InstanceError> {

    Err(InstanceErrorKind::Unsupported.into())
}

pub async fn main_opts_instance_cidr(
    api: &mut TokApi,
    name: &str,
    action: OptsCidrAction,
) -> Result<(), InstanceError> {
    let (instance, _) = api.instance_action(name).await?;
    let instance = instance?;
    
    main_opts_cidr(instance, action).await?;

    Ok(())
}

pub async fn main_opts_instance_peering(
    api: &mut TokApi,
    name: &str,
    action: OptsPeeringAction,
) -> Result<(), InstanceError> {
    let (instance, wallet_instance) = api.instance_action(name).await?;
    let instance = instance?;
    
    main_opts_peering(api, instance, wallet_instance, action).await?;

    Ok(())
}

pub async fn main_opts_instance_reset(
    api: &mut TokApi,
    name: &str,
) -> Result<(), InstanceError> {
    let (instance, _) = api.instance_action(name).await?;
    let mut instance = instance?;
    
    let dio = instance.dio_mut();
    {
        let mut instance = instance.as_mut();
        let _ = instance.mesh_nodes.clear().await;
    }
    dio.commit().await?;

    Ok(())
}

pub async fn main_opts_instance(
    opts: OptsInstanceFor,
    token_path: String,
    auth_url: url::Url,
    db_url: url::Url,
    inst_url: url::Url,
    security: StreamSecurity
) -> Result<(), InstanceError>
{
    // Check if sudo is needed
    let needs_sudo = if opts.action().needs_sudo() {
        true
    } else {
        false
    };

    // Determine the instance authority from the session URL
    let mut instance_authority = inst_url.domain()
        .map(|a| a.to_string())
        .unwrap_or_else(|| "tokera.sh".to_string());
    if instance_authority == "localhost" {
        instance_authority = "tokera.sh".to_string();
    }

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
            main_opts_instance_details(&mut context.api, inst_url, opts).await?;
        }
        OptsInstanceAction::Create(opts) => {
            main_opts_instance_create(&mut context.api, opts.name, purpose.group_name(), db_url, instance_authority, opts.force).await?;
        }
        OptsInstanceAction::Kill(opts_kill) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_kill(&mut context.api, name.as_str(), opts_kill.force).await?;
        }
        OptsInstanceAction::Shell(_opts_exec) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_shell(&mut context.api, inst_url, name.as_str(), security).await?;
        }
        OptsInstanceAction::Call(opts_call) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_call(&mut context.api, inst_url, name.as_str(), opts_call.binary.as_str(), opts_call.topic.as_str(), security).await?;
        }
        OptsInstanceAction::Export(opts_export) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_export(&mut context.api, inst_url, name.as_str(), opts_export.binary.as_str(), opts_export.pinned, opts_export.no_http, opts_export.no_https, opts_export.no_bus).await?;
        }
        OptsInstanceAction::Deport(opts_deport) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_deport(&mut context.api, name.as_str(), opts_deport.token.as_str()).await?;
        }
        OptsInstanceAction::Clone(_opts_clone) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_clone(&mut context.api, name.as_str()).await?;
        }
        OptsInstanceAction::Mount(_opts_mount) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_mount(&mut context.api, name.as_str()).await?;
        }
        OptsInstanceAction::Cidr(opts_cidr) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_cidr(&mut context.api, name.as_str(), opts_cidr.action).await?;
        }
        OptsInstanceAction::Peering(opts_peering) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_peering(&mut context.api, name.as_str(), opts_peering.action).await?;
        }
        OptsInstanceAction::Reset(_opts_reset) => {
            if name.is_none() { bail!(InstanceErrorKind::InvalidInstance); }
            let name = name.unwrap();
            main_opts_instance_reset(&mut context.api, name.as_str()).await?;
        }
    }

    Ok(())
}
