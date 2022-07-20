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
    pub async fn deposit_create(
        &mut self,
        currency: NationalCurrency,
        amount: Decimal,
    ) -> Result<DepositResponse, WalletError> {
        // Invoke the command
        let session = self.dio.session().clone_session();
        let ret = deposit_command(
            &self.registry,
            amount,
            currency,
            session.deref(),
            self.auth.clone(),
        )
        .await?;

        // Load the new set of coins into the wallet
        self.wallet
            .inbox
            .push_with_dio(&self.dio, ret.coin_ancestor.clone())?;

        // Now add the history
        if let Err(err) = self
            .record_activity(HistoricActivity::DepositCreated(
                activities::DepositCreated {
                    when: chrono::offset::Utc::now(),
                    by: self.user_identity(),
                    invoice_number: ret.invoice_number.clone(),
                    invoice_id: ret.invoice_id.clone(),
                    amount,
                    currency,
                    pay_url: ret.pay_url.clone(),
                },
            ))
            .await
        {
            error!("Error writing activity: {}", err);
        }

        self.dio.commit().await?;

        Ok(ret)
    }

    pub async fn deposit_query(&mut self) -> Result<CoinCollectResponse, WalletError> {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to query despoits on the wallet when there are uncommitted transactions on the DIO"))));
        }

        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }
        let ret = self.__deposit_query().await;

        // Unlock and return the result
        self.dio.commit().await?;
        self.wallet.unlock().await?;
        ret
    }

    pub(super) async fn __deposit_query(&mut self) -> Result<CoinCollectResponse, WalletError> {
        let things = self
            .wallet
            .inbox
            .iter()
            .await?
            .filter(|a| {
                if let CommodityCategory::NationalCurrency(_) = a.kind.category() {
                    true
                } else {
                    false
                }
            })
            .map(|a| a.take())
            .collect::<Vec<_>>();

        let query = self.__coin_collect_internal(things).await?;
        Ok(query)
    }

    pub async fn deposit_cancel(&mut self, invoice_number: &str) -> Result<(), WalletError> {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to cancel deposit for the wallet when there are uncommitted transactions on the DIO"))));
        }

        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }
        let ret = self.__deposit_cancel(invoice_number).await;

        // Unlock and return the result
        self.dio.commit().await?;
        self.wallet.unlock().await?;
        ret
    }

    pub(super) async fn __deposit_cancel(
        &mut self,
        invoice_number: &str,
    ) -> Result<(), WalletError> {
        let coins = self
            .wallet
            .inbox
            .iter()
            .await?
            .filter(|a| {
                if let CommodityCategory::NationalCurrency(_) = a.kind.category() {
                    true
                } else {
                    false
                }
            })
            .map(|a| a.take())
            .collect::<Vec<_>>();

        let query = self.__coin_collect_internal(coins).await?;

        let ancestor = query
            .pending_deposits
            .iter()
            .filter(|a| a.invoice_number.as_str() == invoice_number)
            .next();

        let ancestor = match ancestor {
            Some(a) => a,
            None => {
                bail!(WalletErrorKind::CoinError(CoinErrorKind::InvalidReference(
                    invoice_number.to_string()
                )));
            }
        };
        let to_delete = self
            .wallet
            .as_mut()
            .inbox
            .iter_mut()
            .await?
            .filter(|a| ancestor.key.eq(&a.what))
            .next()
            .unwrap();

        let _ = cancel_deposit_command(&self.registry, to_delete.clone().take(), self.auth.clone())
            .await?;

        to_delete.delete()?;
        self.dio.commit().await?;
        Ok(())
    }
}
