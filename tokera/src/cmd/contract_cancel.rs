#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};

use crate::error::*;
use crate::opt::*;
use crate::api::*;

pub async fn main_opts_contract_cancel(opts: OptsContractCancel, api: &mut TokApi, identity: &str) -> Result<(), ContractError>
{
    match api.contract_cancel(opts.reference_number.as_str(), identity).await {
        Ok(a) => a,
        Err(ContractError(ContractErrorKind::InvalidReference(reference_number), _)) => {
            eprintln!("No contract exists with this ID ({}).", reference_number);
            std::process::exit(1);
        },
        Err(err) => return Err(err),
    };

    Ok(())
}