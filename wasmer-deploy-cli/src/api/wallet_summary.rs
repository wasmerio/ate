use num_traits::*;
use std::collections::BTreeMap;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;
use crate::model::*;

use super::*;

#[derive(Debug, Clone)]
pub struct DenominationSummary {
    pub denomination: Decimal,
    pub total: Decimal,
    pub cnt: usize,
}

#[derive(Debug, Clone)]
pub struct CurrencySummary {
    pub currency: NationalCurrency,
    pub total: Decimal,
    pub denominations: BTreeMap<Decimal, DenominationSummary>,
}

#[derive(Debug, Clone, Default)]
pub struct WalletSummary {
    pub currencies: BTreeMap<NationalCurrency, CurrencySummary>,
}

impl DeployApi {
    pub async fn wallet_summary(&mut self) -> Result<WalletSummary, WalletError> {
        // Determine what currencies are currently stored within the wallet
        let mut currencies = self
            .wallet
            .bags
            .iter()
            .await?
            .map(|(a, _)| a.currency)
            .collect::<Vec<_>>();
        currencies.sort();
        currencies.dedup();

        let mut ret = WalletSummary::default();
        for currency in currencies {
            let currency_summary = self.__wallet_currency_summary(currency).await?;
            if currency_summary.total <= Decimal::zero() {
                continue;
            }

            ret.currencies.insert(currency, currency_summary);
        }

        Ok(ret)
    }

    pub async fn wallet_currency_summary(
        &mut self,
        currency: NationalCurrency,
    ) -> Result<CurrencySummary, WalletError> {
        // First we reconcile any deposits that have been made
        self.reconcile().await?;

        // Now make the currency summary
        self.__wallet_currency_summary(currency).await
    }

    pub(super) async fn __wallet_currency_summary(
        &mut self,
        currency: NationalCurrency,
    ) -> Result<CurrencySummary, WalletError> {
        // Compute the total for all chests of this currency
        let bags = self
            .wallet
            .bags
            .iter()
            .await?
            .filter(|(a, _)| a.currency == currency)
            .collect::<Vec<_>>();

        trace!("{} bags", bags.len());
        for (denomination, _) in bags.iter() {
            trace!("bag= {} {:?}", denomination.to_string(), denomination);
        }

        let mut total = Decimal::zero();
        for (_, bag) in bags.iter() {
            let sub_total: Decimal = bag.coins.iter().map(|a| a.value).sum();
            total += sub_total;
        }

        let mut coin_denominations = bags.iter().map(|(a, _)| a.value).collect::<Vec<_>>();
        coin_denominations.sort();
        coin_denominations.dedup();
        coin_denominations.reverse();

        let mut denominations = BTreeMap::default();
        for coin_denomination in coin_denominations {
            let mut cnt = 0usize;
            for (denomination, bag) in bags.iter() {
                if denomination.value == coin_denomination {
                    cnt += bag.coins.len();
                }
            }

            let total = coin_denomination * Decimal::from(cnt);
            if total <= Decimal::zero() {
                continue;
            }

            denominations.insert(
                coin_denomination,
                DenominationSummary {
                    denomination: coin_denomination,
                    total,
                    cnt: cnt,
                },
            );
        }

        Ok(CurrencySummary {
            currency,
            total,
            denominations,
        })
    }
}
