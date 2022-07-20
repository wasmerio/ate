#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::api::*;
use crate::error::*;
use crate::opt::*;

pub async fn main_opts_contract_cancel(
    opts: OptsContractCancel,
    api: &mut DeployApi,
    identity: &str,
) -> Result<(), ContractError> {
    match api
        .contract_cancel(opts.reference_number.as_str(), identity)
        .await
    {
        Ok(a) => a,
        Err(ContractError(ContractErrorKind::InvalidReference(reference_number), _)) => {
            eprintln!("No contract exists with this ID ({}).", reference_number);
            std::process::exit(1);
        }
        Err(err) => return Err(err),
    };

    Ok(())
}
