use error_chain::*;
use std::ops::Deref;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::model::*;
use crate::request::*;

use super::*;

impl DeployApi {
    pub async fn withdraw(
        &mut self,
        currency: NationalCurrency,
        amount: Decimal,
        wallet_name: String,
    ) -> Result<WithdrawResponse, WalletError> {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to withdraw from the wallet when there are uncommitted transactions on the DIO"))));
        }

        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }
        let ret = self.__withdraw(currency, amount, wallet_name).await;

        // Commit, unlock and return the result
        self.dio.commit().await?;
        self.wallet.unlock().await?;
        ret
    }

    pub async fn __withdraw(
        &mut self,
        currency: NationalCurrency,
        amount: Decimal,
        wallet_name: String,
    ) -> Result<WithdrawResponse, WalletError> {
        // Make the session
        let session = self.dio.session().clone_session();

        // First carve out the coins we want
        let carved = self.__carve_bag(currency, amount, true).await?;

        // Now withdraw the amount specified
        let ret = withdraw_command(
            &self.registry,
            carved.coins,
            session.deref(),
            self.auth.clone(),
            wallet_name,
        )
        .await?;

        // Now add the history
        if let Err(err) = self
            .record_activity(HistoricActivity::FundsWithdrawn(
                activities::FundsWithdrawn {
                    when: chrono::offset::Utc::now(),
                    by: self.user_identity(),
                    receipt_number: ret.receipt_number.clone(),
                    amount_less_fees: ret.amount_less_fees,
                    fees: ret.fees,
                    currency,
                },
            ))
            .await
        {
            error!("Error writing activity: {}", err);
        }

        Ok(ret)
    }
}
