#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};

use crate::error::*;
use crate::api::*;

pub async fn main_opts_contract_list(api: &mut TokApi) -> Result<(), ContractError>
{
    let result = api.contract_summary().await?;

    println!("|-----code-----|-------------reference------------|---status---");
    for contract in result {
        println!("- {:12} - {:16} - {}", contract.service.code, contract.reference_number, contract.status);
    }

    Ok(())
}