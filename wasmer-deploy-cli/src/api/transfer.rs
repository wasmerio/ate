use error_chain::*;
use std::ops::Deref;
use std::sync::Arc;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::model::*;
use crate::opt::*;
use crate::request::*;
use ate::prelude::*;

use super::*;

impl DeployApi {
    pub async fn deposit_coins(
        &mut self,
        coins: BagOfCoins,
        notify: Option<CoinRotateNotification>,
    ) -> std::result::Result<CoinRotateResponse, WalletError> {
        // Add the new ownership coins to the destination wallet
        let new_token = EncryptKey::generate(KeySize::Bit192);
        let new_owners = coins.clone().to_ownerships().await;
        for mut new_owner in new_owners.into_iter() {
            new_owner.token = new_token.clone();
            self.wallet.inbox.push_with_dio(&self.dio, new_owner)?;
        }
        self.dio.commit().await?;

        // Now rotate ownerships
        let session = self.dio.session().clone_session();
        let ret = coin_rotate_command(
            &self.registry,
            coins.coins,
            new_token,
            session.deref(),
            self.auth.clone(),
            notify,
        )
        .await?;

        // Lastly we need to reconcile so that the coins get put into chests
        self.reconcile().await?;
        Ok(ret)
    }

    pub async fn transfer<A, B>(
        &mut self,
        amount: Decimal,
        currency: NationalCurrency,
        destination: &dyn OptsPurpose<A>,
        source: &dyn OptsPurpose<B>,
        should_notify: bool,
    ) -> std::result::Result<CoinRotateResponse, WalletError>
    where
        A: Clone,
        B: Clone,
    {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to transfer from the wallet when there are uncommitted transactions on the DIO"))));
        }

        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }
        let ret = self
            .__transfer::<A, B>(amount, currency, destination, source, should_notify)
            .await;

        // Commit unlock and return the result
        self.dio.commit().await?;
        self.wallet.unlock().await?;
        ret
    }

    pub(super) async fn __transfer<A, B>(
        &mut self,
        amount: Decimal,
        currency: NationalCurrency,
        destination: &dyn OptsPurpose<A>,
        source: &dyn OptsPurpose<B>,
        should_notify: bool,
    ) -> std::result::Result<CoinRotateResponse, WalletError>
    where
        A: Clone,
        B: Clone,
    {
        // Get the session and identity
        let session = self.dio.session().clone_session();
        let identity = get_identity(destination, session.deref()).await?;

        // Open the chain
        let registry = Arc::clone(&self.registry);
        let chain_key = chain_key_4hex(&identity, Some("redo"));
        debug!("chain_url={}", self.auth);
        debug!("chain_key={}", chain_key);
        let chain = registry.open(&self.auth, &chain_key, true).await?;

        // Load the API for the destination
        // Open the DIO and load the wallet
        debug!("getting destination wallet");
        let mut api_destination = {
            let session = session.clone_inner();
            let session: AteSessionType = if let Purpose::<A>::Domain {
                domain_name: group_name,
                wallet_name: _,
                action: _,
            } = destination.purpose()
            {
                gather_command(
                    &self.registry,
                    group_name.clone(),
                    session,
                    self.auth.clone(),
                )
                .await?
                .into()
            } else {
                session.into()
            };
            let mut dio = chain.dio_trans(&session, TransactionScope::Full).await;
            let wallet = get_wallet(destination, &mut dio, &identity).await?;
            build_api_accessor(&dio, wallet, self.auth.clone(), self.db_url.clone(), &registry).await
        };

        // Carve out the coins from the wallet
        debug!("carving bag");
        let carved = self.__carve_bag(currency, amount, true).await?;

        // Lets build the notify
        let email = session.user().identity().to_string();
        let notify = if should_notify {
            let notify = CoinRotateNotification {
                operator: email.clone(),
                receipt_number: AteHash::generate().to_hex_string().to_uppercase(),
                from: source.fullname(email.as_str()),
                to: destination.fullname(email.as_str()),
            };
            Some(notify)
        } else {
            None
        };

        // Deposit the coins
        debug!("depositing coins");
        let ret = api_destination.deposit_coins(carved, notify).await?;

        // Now add the history into both wallets
        {
            let activity = activities::FundsTransferred {
                when: chrono::offset::Utc::now(),
                by: self.user_identity(),
                amount,
                currency,
                from: source.fullname(email.as_str()),
                to: destination.fullname(email.as_str()),
            };
            if let Err(err) = self
                .record_activity(HistoricActivity::TransferOut(activity.clone()))
                .await
            {
                error!("Error writing activity: {}", err);
            }

            match api_destination
                .record_activity(HistoricActivity::TransferIn(activity.clone()))
                .await
            {
                Ok(_) => {
                    if let Err(err) = api_destination.commit().await {
                        error!("Error writing activity: {}", err);
                    }
                }
                Err(err) => error!("Error writing activity: {}", err),
            }
        }

        // Success
        Ok(ret)
    }
}
