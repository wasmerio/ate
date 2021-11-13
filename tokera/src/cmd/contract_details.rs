#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};

use crate::error::*;
use crate::opt::*;
use crate::api::*;

pub async fn main_opts_contract_details(opts: OptsContractDetails, api: &mut TokApi) -> Result<(), ContractError>
{
    let contract = match api.contract_get(opts.reference_number.as_str()).await {
        Ok(a) => a,
        Err(ContractError(ContractErrorKind::InvalidReference(reference_number), _)) => {
            eprintln!("No contract exists with this ID ({}).", reference_number);
            std::process::exit(1);
        },
        Err(err) => return Err(err),
    };
    let service = contract.service;

    println!("{}", service.description);
    for rate_card in service.rate_cards {
        println!("==================");
        println!("{}", serde_json::to_string_pretty(&rate_card).unwrap());
    }
    println!("==================");
    println!("{}", serde_json::to_string_pretty(&contract.metrics).unwrap());
    println!("==================");
    println!("status: {}", contract.status);
    
    Ok(())
}