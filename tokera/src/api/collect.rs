#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};
use ate::prelude::*;
use fxhash::FxHashSet;
use std::ops::Deref;

use crate::model::*;
use crate::error::*;
use crate::cmd::*;
use crate::request::*;

use super::*;

impl TokApi
{
    pub(crate) async fn __coin_collect_internal(&mut self, coin_ancestors: Vec<Ownership>) -> Result<CoinCollectResponse, CoinError>
    {
        // Make the request to carve the coins (if we have a local tok service then use it)
        let query = coin_collect_command(&self.registry, coin_ancestors, self.auth.clone()).await?;

        // Now add the history
        for confirm in query.confirmations.iter()
        {
            if let Err(err) = self.record_activity(HistoricActivity::DepositCompleted(activities::DepositCompleted {
                when: confirm.when.clone(),
                by: self.user_identity(),
                invoice_number: confirm.invoice_number.clone(),
                amount: confirm.amount.clone(),
                currency: confirm.currency.clone(),
                invoice_url: confirm.invoice_url.clone(),
            })).await {
                error!("Error writing activity: {}", err);
            }
        }

        return Ok(query);
    }

    pub(super) async fn __collect_coins(&mut self) -> Result<(), WalletError>
    {
        // Determine what currencies are currently stored within the wallet
        let mut currencies = self.wallet.inbox.iter().await?
            .filter_map(|a| {
                if let CommodityCategory::NationalCurrency(b) = a.kind.category() {
                    Some(b)
                } else {
                    None
                }    
            })
            .collect::<Vec<_>>();
        currencies.sort();
        currencies.dedup();

        for currency in currencies
        {
            let mut ancestors = self.wallet.inbox.iter_ext(true, true).await?
                .filter(|a| {
                    if let CommodityCategory::NationalCurrency(b) = a.kind.category() {
                        currency == b
                    } else {
                        false
                    }    
                })
                .collect::<Vec<Dao<Ownership>>>();

            // We process in batches of 50 so that we dont have timeouts
            // while processing wallets that have huge numbers of deposits
            while ancestors.len() > 0
            {
                // Build a list of all the coin ancestors
                let ancestors = {
                    let mut a = Vec::new();
                    while let Some(b) = ancestors.pop() {
                        a.push(b);
                        if a.len() > 50 {
                            break;
                        }
                    }
                    a
                };

                // Query the currency
                let query = {
                    let query_ancestors = ancestors.iter()
                        .map(|a| a.deref().clone())
                        .collect::<Vec<_>>();
                    self.__coin_collect_internal(query_ancestors).await?
                };

                // Build a list of all the coin ancestors that would be destroyed by this operation
                let mut pending_deposits = FxHashSet::default();
                query.pending_deposits.iter().for_each(|o| { pending_deposits.insert(o.owner.clone()); } );
                
                // If the query returned any bags of coins we need to add them to
                // the wallet in the right chests
                for coin in query.cleared_coins {
                    self.__add_coin_to_wallet(coin).await?;
                }

                // Delete any ancestors that completed (not in a pending state)
                // (they will be added to the chests instead)
                let delete_me = ancestors.iter()
                    .filter(|a| pending_deposits.contains(a.deref()) == false)
                    .map(|a| a.key().clone())
                    .collect::<Vec<_>>();
                for d in delete_me {
                    self.dio.delete(&d).await?;
                }

                // Commit the transaction
                self.dio.commit().await?;
            }
        }
        Ok(())
    }
}