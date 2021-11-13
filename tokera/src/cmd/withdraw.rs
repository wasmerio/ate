#[allow(unused_imports)]
use tracing::{info, error, debug, trace, warn};
use error_chain::bail;
use std::sync::Arc;
use url::Url;

use ate::prelude::*;

use crate::opt::*;
use crate::error::*;
use crate::model::*;
use crate::api::*;
use crate::request::*;

use super::*;

pub async fn withdraw_command(registry: &Arc<Registry>, coins: Vec<CarvedCoin>, session: &'_ dyn AteSession, auth: Url, wallet_name: String) -> Result<WithdrawResponse, WalletError>
{
    // The signature key needs to be present to send the notification
    let sign_key = match session.user().user.write_keys().next() {
        Some(a) => a,
        None => {
            bail!(CoreError::from_kind(CoreErrorKind::NoMasterKey));
        }
    };

    // Get the receiver email address
    let receiver = session.user().identity().to_string();
    
    // Create the login command
    let email = session.user().identity().to_string();
    let query = WithdrawRequest {
        coins,
        params: SignedProtectedData::new(sign_key, WithdrawRequestParams {
            sender: email.clone(),
            receiver: receiver.clone(),
            wallet: wallet_name,
        })?,
    };

    // Attempt the login request with a 10 second timeout
    let chain = registry.open_cmd(&auth).await?;
    let response: Result<WithdrawResponse, WithdrawFailed> = chain.invoke(query).await?;
    let result = response?;
    Ok(result)
}

pub async fn main_opts_withdraw<A>(opts: OptsWithdraw, source: &dyn OptsPurpose<A>, api: &mut TokApi)
-> Result<(), WalletError>
where A: Clone
{
    let identity = api.dio.session().user().identity().to_string();

    api.withdraw(opts.currency, opts.amount, source.fullname(identity.as_str())).await?;
    println!("Successfully withdrawn {} {}", opts.amount, opts.currency);

    // Show the new balances
    println!("");
    main_opts_balance(OptsBalance {
        coins: false,
        no_reconcile: false,
    }, api).await?;
    
    // Done
    Ok(())
}