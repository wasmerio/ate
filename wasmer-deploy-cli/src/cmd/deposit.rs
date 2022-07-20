use error_chain::*;
use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};
use url::Url;

use ate::prelude::*;

use crate::api::*;
use crate::error::*;
use crate::model::*;
use crate::opt::*;
use crate::request::*;

#[allow(dead_code)]
pub async fn deposit_command(
    registry: &Arc<Registry>,
    amount: Decimal,
    currency: NationalCurrency,
    session: &'_ dyn AteSession,
    auth: Url,
) -> Result<DepositResponse, WalletError> {
    // Open a command chain
    let chain = registry.open_cmd(&auth).await?;

    // Get the email and build the proof
    let email = session.user().identity().to_string();
    let sign_key = match session.write_keys(AteSessionKeyCategory::SudoKeys).next() {
        Some(a) => a.clone(),
        None => {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::MissingTokenKey));
        }
    };

    // Create the login command
    let deposit = DepositRequest {
        proof: CoinProof {
            inner: SignedProtectedData::new(
                &sign_key,
                CoinProofInner {
                    amount,
                    currency,
                    email,
                },
            )?,
        },
    };

    // Attempt the login request with a 10 second timeout
    let response: Result<DepositResponse, DepositFailed> = chain.invoke(deposit).await?;
    let result = response?;
    Ok(result)
}

#[allow(unreachable_code)]
pub async fn main_opts_deposit_new(
    opts: OptsDepositNew,
    api: &mut DeployApi,
) -> Result<(), WalletError> {
    let ret = api.deposit_create(opts.currency, opts.amount).await?;

    println!("Deposit invoice created (id={})", ret.invoice_id);
    println!("");

    // Display the QR code
    println!("Below is a URL you can use to pay the invoice via PayPal:");
    println!("{}", ret.pay_url);
    println!("");
    println!("Alternatively below is an QR code - scan it on your phone to pay");
    println!("");
    println!("{}", ret.qr_code);

    // Done
    Ok(())
}

#[allow(unreachable_code)]
pub async fn main_opts_deposit_pending(
    _opts: OptsDepositPending,
    api: &mut DeployApi,
) -> Result<(), WalletError> {
    let query = api.deposit_query().await?;

    println!("Id               Status Value");
    for c in query.pending_deposits {
        println!(
            "{:16} UNPAID {} {} - {}",
            c.invoice_number, c.reserve, c.currency, c.pay_url
        );
    }

    Ok(())
}

#[allow(unreachable_code)]
pub async fn main_opts_deposit_cancel(
    opts: OptsDepositCancel,
    api: &mut DeployApi,
) -> Result<(), WalletError> {
    match api.deposit_cancel(opts.id.as_str()).await {
        Ok(a) => a,
        Err(WalletError(WalletErrorKind::InvalidReference(invoice_number), _)) => {
            eprintln!(
                "Wallet has no deposit request with this ID ({}).",
                invoice_number
            );
            std::process::exit(1);
        }
        Err(WalletError(WalletErrorKind::InvoiceAlreadyPaid(invoice_number), _)) => {
            eprintln!(
                "Can not cancel a deposit that is already paid ({}).",
                invoice_number
            );
            std::process::exit(1);
        }
        Err(err) => return Err(err),
    };

    println!("{} has been cancelled.", opts.id);
    Ok(())
}

#[allow(unreachable_code)]
pub async fn main_opts_deposit(opts: OptsDeposit, api: &mut DeployApi) -> Result<(), WalletError> {
    match opts.action {
        OptsDepositAction::Pending(opts) => {
            main_opts_deposit_pending(opts, api).await?;
        }
        OptsDepositAction::New(opts) => {
            main_opts_deposit_new(opts, api).await?;
        }
        OptsDepositAction::Cancel(opts) => {
            main_opts_deposit_cancel(opts, api).await?;
        }
    }
    Ok(())
}
