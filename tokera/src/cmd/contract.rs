#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use crate::error::*;
use crate::opt::*;

use super::*;

pub async fn main_opts_contract(
    opts: OptsContractFor,
    token_path: String,
    auth_url: Url,
) -> Result<(), ContractError> {
    let mut context = PurposeContext::new(&opts, token_path.as_str(), &auth_url, true).await?;
    let identity = context.identity.clone();

    match context.action.clone() {
        OptsContractAction::List => {
            main_opts_contract_list(&mut context.api).await?;
        }
        OptsContractAction::Details(opts) => {
            main_opts_contract_details(opts, &mut context.api).await?;
        }
        OptsContractAction::Cancel(opts) => {
            main_opts_contract_cancel(opts, &mut context.api, &identity).await?;
        }
    }

    context.api.commit().await?;
    Ok(())
}
