use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info};

use ate::prelude::*;

use crate::error::*;

use crate::api::*;
use crate::cmd::*;
use crate::model::*;
use crate::opt::*;

use crate::model::WALLET_COLLECTION_ID;

use super::core::*;

pub(super) async fn get_or_create_wallet(
    purpose: &dyn OptsPurpose<OptWalletAction>,
    dio: &Arc<DioMut>,
    auth: &url::Url,
    registry: &Arc<Registry>,
    identity: &String,
) -> Result<DaoMut<Wallet>, AteError> {
    let action = purpose.action();

    // Make sure the parent exists
    let parent_key = PrimaryKey::from(identity.clone());
    debug!("parent_key={}", parent_key);
    if dio.exists(&parent_key).await == false {
        eprintln!("The parent user or group does not exist in the chain-or-trust.");
        std::process::exit(1);
    }

    // Grab a reference to the wallet
    let wallet_name = get_wallet_name(purpose)?;
    let mut wallet_vec = DaoVec::<Wallet>::new_orphaned_mut(dio, parent_key, WALLET_COLLECTION_ID);
    let wallet = wallet_vec
        .iter_mut()
        .await?
        .into_iter()
        .filter(|a| a.name.eq_ignore_ascii_case(wallet_name.as_str()))
        .next();

    // Create the wallet if it does not exist (otherwise make sure it was loaded)
    let wallet = match action {
        OptWalletAction::Create(opts) => {
            if wallet.is_some() {
                eprintln!("Wallet ({}) already exists (with same name).", wallet_name);
                std::process::exit(1);
            }

            create_wallet(
                dio,
                auth,
                registry,
                &identity,
                &wallet_name,
                &parent_key,
                opts.country,
            )
            .await?
        }
        _ => match wallet {
            Some(a) => a,
            None => {
                eprintln!("Wallet ({}) does not exist - you must first 'create' the wallet before using it.", wallet_name);
                std::process::exit(1);
            }
        },
    };

    Ok(wallet)
}

#[allow(unreachable_code)]
pub async fn main_opts_remove(opts: OptsRemoveWallet, api: TokApi) -> Result<(), WalletError> {
    match api.delete_wallet(opts.force).await {
        Ok(_) => {}
        Err(WalletError(WalletErrorKind::WalletNotEmpty, _)) => {
            eprintln!("The wallet is not empty and thus can not be removed.");
            std::process::exit(1);
        }
        Err(err) => return Err(err),
    };
    Ok(())
}

pub async fn main_opts_wallet(
    opts_wallet: OptsWalletSource,
    token_path: String,
    auth_url: url::Url,
) -> Result<(), WalletError> {
    let sudo = match opts_wallet.action() {
        OptWalletAction::Balance(_) => true,
        OptWalletAction::History(_) => true,
        OptWalletAction::Create(_) => true,
        OptWalletAction::Remove(_) => true,
        OptWalletAction::Deposit(_) => true,
        OptWalletAction::Transfer(_) => true,
        OptWalletAction::Withdraw(_) => true,
        #[allow(unreachable_patterns)]
        _ => false,
    };

    // Create the API to the wallet
    let inner =
        PurposeContextPrelude::new(&opts_wallet, token_path.as_str(), &auth_url, sudo).await?;
    let wallet = get_or_create_wallet(
        &opts_wallet,
        &inner.dio,
        &auth_url,
        &inner.registry,
        &inner.identity,
    )
    .await?;
    let api = crate::api::build_api_accessor(&inner.dio, wallet, auth_url, None, &inner.registry).await;

    let mut context = PurposeContext::<OptWalletAction> { inner, api };

    // Determine what we need to do
    match context.inner.action {
        OptWalletAction::Create(_) => {
            context.api.commit().await?;

            // This was handled earlier
            eprintln!("Wallet successfully created.")
        }
        OptWalletAction::Remove(opts_remove_wallet) => {
            main_opts_remove(opts_remove_wallet, context.api).await?;
            return Ok(());
        }
        OptWalletAction::Balance(opts_balance) => {
            main_opts_balance(opts_balance, &mut context.api).await?;
        }
        OptWalletAction::History(opts_history) => {
            main_opts_transaction_history(opts_history, &mut context.api).await?;
        }
        OptWalletAction::Deposit(opts_deposit) => {
            main_opts_deposit(opts_deposit, &mut context.api).await?;
        }
        OptWalletAction::Transfer(opts_transfer) => {
            main_opts_transfer(opts_transfer, &opts_wallet, &mut context.api).await?;
        }
        OptWalletAction::Withdraw(opts_withdraw) => {
            main_opts_withdraw(opts_withdraw, &opts_wallet, &mut context.api).await?;
        }
    }

    context.api.commit().await?;
    Ok(())
}
