#[allow(unused_imports)]
use tracing::{info, error, debug};
use error_chain::bail;
use std::time::Duration;
use ate_auth::helper::*;

use crate::error::*;
use crate::opt::*;
use crate::api::*;

use super::*;

pub async fn main_opts_service_list(auth_url: &url::Url) -> Result<(), CoreError>
{
    println!("|-----code-----|---provider---|---name---");
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.keep_alive(Duration::from_secs(10)).cement();
    let services = service_find_command(&registry, None, auth_url.clone()).await?;
    for service in services.services {
        println!("- {:12} - {:12} - {}", service.code, service.owner_identity, service.name);
    }
    Ok(())
}

pub async fn main_opts_service_details(opts: OptsServiceDetails, auth_url: &url::Url) -> Result<(), CoreError>
{
    let registry = ate::mesh::Registry::new(&conf_cmd()).await.keep_alive(Duration::from_secs(10)).cement();
    let services = service_find_command(&registry, None, auth_url.clone()).await?;

    let service_name = opts.service_name.to_lowercase();
    let service = match get_advertised_service(&services, &service_name)?
    {
        Some(a) => a,
        None => {
            eprintln!("The named service does not exist.");
            std::process::exit(1);
        }
    };

    println!("{}", service.description);
    for rate_card in service.rate_cards.iter() {
        println!("==================");
        println!("{}", serde_json::to_string_pretty(rate_card).unwrap());
    }
    
    Ok(())
}

pub async fn main_opts_service_subscribe(opts: OptsSubscribe, api: &mut TokApi) -> Result<(), ContractError>
{
    let services = service_find_command(&api.registry, None, api.auth.clone()).await?;

    let service_name = opts.service_name.to_lowercase();
    let service = match get_advertised_service(&services, &service_name)?
    {
        Some(a) => a,
        None => {
            eprintln!("The named service does not exist.");
            std::process::exit(1);
        }
    };

    // Find the rate card that matches this particular countries currency
    let rate_card = service.rate_cards.iter()
        .filter(|a| a.currency == api.wallet.gst_country.national_currency())
        .map(|a| a.clone())
        .next();
    let rate_card = match rate_card {
        Some(a) => a,
        None => {
            eprintln!("Unfortunately your national currency ({}) is not supported by this.", api.wallet.gst_country.national_currency());
            std::process::exit(1);
        }
    };

    // We need an agreement to the terms and conditions from the caller
    println!("{}", service.terms_and_conditions);
    println!("==================");
    println!("{}", service.description);
    println!("==================");
    println!("{}", serde_json::to_string_pretty(&rate_card).unwrap());
    println!("==================");
    println!("Please agree to contract and its terms and conditions above by typing the word 'agree'");
    
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).expect("Did not enter a valid response");
    let agreement = s.trim().to_string().to_lowercase();
    if agreement != "agree" {
        eprintln!("You may only create an contracts by specifically agreeing to the terms");
        eprintln!("which can only be confirmed if you specifically type the word 'agree'");
        std::process::exit(1);
    }

    let ret = match api.contract_create(service.clone()).await {
        Ok(a) => a,
        Err(ContractError(ContractErrorKind::AlreadyExists(msg), _)) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
        Err(err) => { bail!(err); }
    };
    println!("Contract created ({})", ret.contract_reference);

    Ok(())
}

pub async fn main_opts_service(opts: OptsServiceFor, token_path: String, auth_url: url::Url) -> Result<(), ContractError>
{
    // Determine what we need to do
    let purpose: &dyn OptsPurpose<OptsServiceAction> = &opts;
    match purpose.action() {
        OptsServiceAction::List => {
            main_opts_service_list(&auth_url).await?;
        },
        OptsServiceAction::Details(opts) => {
            main_opts_service_details(opts, &auth_url).await?;
        },
        OptsServiceAction::Subscribe(opts) => {
            let mut context = PurposeContext::new(purpose, token_path.as_str(), &auth_url, true).await?;
            main_opts_service_subscribe(opts, &mut context.api).await?;
            context.api.commit().await?;
        }
    }

    Ok(())
}