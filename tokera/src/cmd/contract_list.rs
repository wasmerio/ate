#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;
use crate::error::*;

pub async fn main_opts_contract_list(api: &mut TokApi) -> Result<(), ContractError> {
    let result = api.contract_summary().await?;

    println!("|-----code-----|-------------reference------------|---status---");
    for contract in result {
        println!(
            "- {:12} - {:16} - {}",
            contract.service.code, contract.reference_number, contract.status
        );
    }

    Ok(())
}
