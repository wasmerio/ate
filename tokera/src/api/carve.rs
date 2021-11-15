use error_chain::bail;
use num_traits::*;
use std::collections::BTreeMap;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::helper::*;
use crate::model::*;

use super::*;

impl TokApi {
    pub async fn carve_bag(
        &mut self,
        currency: NationalCurrency,
        needed_total_amount: Decimal,
        auto_recover_coins: bool,
    ) -> Result<BagOfCoins, WalletError> {
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
            .__carve_bag(currency, needed_total_amount, auto_recover_coins)
            .await;

        // Commit unlock and return the result
        self.dio.commit().await?;
        self.wallet.unlock().await?;
        ret
    }

    pub(crate) async fn __carve_bag(
        &mut self,
        currency: NationalCurrency,
        needed_total_amount: Decimal,
        auto_recover_coins: bool,
    ) -> Result<BagOfCoins, WalletError> {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to cave a bag of coins when there are uncommitted transactions on the DIO"))));
        }

        // Loop a limited number of times
        for n in 1..=50 {
            trace!("carve: attempt={}", n);

            // First reconcile the wallet to recover any coins or deposits
            // (this reconcile is also needed to collect any coins that have been carved up)
            trace!("carve: collect coins");
            self.__collect_coins().await?;

            // Now do a quick check to make sure we actually have enough coins of this currency
            let currency_summary = self.__wallet_currency_summary(currency).await?;
            if needed_total_amount > currency_summary.total {
                trace!(
                    "carve: insufficient coins-needed={} available={}",
                    needed_total_amount,
                    currency_summary.total
                );
                bail!(WalletErrorKind::InsufficientCoins);
            }

            // Grab a reference to all the chests that will be used to carve out this bag of coins
            let bags = self
                .wallet
                .as_mut()
                .bags
                .iter_mut()
                .await?
                .filter(|(k, _)| k.currency == currency)
                .collect::<Vec<_>>();
            trace!("carve: found {} bags of coins", bags.len());
            let mut bags = {
                let mut r = BTreeMap::default();
                bags.into_iter().for_each(|(k, v)| {
                    r.insert(k.value, v);
                });
                r
            };

            // Prepare a bag of coins
            let mut ret_bag = BagOfCoins::default();
            let mut ret_bag_total = Decimal::zero();

            // Build the bag of coins for these denomination brackets to make a certain total amount
            let needed_coins = carve_denominations(needed_total_amount, currency);
            trace!(
                "needed_coins: {}",
                serde_json::to_string_pretty(&needed_coins).unwrap()
            );
            for (needed_denomination, needed_denomination_count) in needed_coins.iter().rev() {
                let needed_denomination = needed_denomination.clone();
                let needed_denomination_count = needed_denomination_count.clone();
                let needed_amount = needed_denomination * Decimal::from(needed_denomination_count);

                // Add all the coins we need to make up this amount
                // (until we run out of coins to add)
                let mut added = Decimal::zero();
                for (denomination, bag) in
                    bags.range_mut(Decimal::zero()..=needed_denomination).rev()
                {
                    let total_bag_coins = bag.coins.len();
                    trace!(
                        "processing chest (denomination={}, total_coins={}",
                        denomination,
                        total_bag_coins
                    );
                    while added < needed_amount {
                        if added + *denomination > needed_amount {
                            break;
                        }
                        match self
                            .__remove_coin_from_wallet(Denomination {
                                value: denomination.clone(),
                                currency,
                            })
                            .await?
                        {
                            Some(coin) => {
                                trace!("carve: adding coin of value {}", coin.value);
                                added += coin.value;
                                ret_bag_total += coin.value;
                                ret_bag.coins.push(coin);
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }

                // If we have enough then move onto the next denomination
                if added == needed_amount {
                    continue;
                }

                // We don't have enough coins of this denomination so lets go and make some more after
                // all the chests are reset
                trace!("carve: we dont have enough coins - lets unzip some bags");
                self.dio.cancel();

                trace!("carve: time to carve up some of the bigger coins");

                // Grab a carved coin from one of the higher denominations
                // Grab a reference to all the chests that will be used to carve out this bag of coins
                let mut bigger_bags = self
                    .wallet
                    .bags
                    .iter_mut_with_dio(&self.dio)
                    .await?
                    .filter(|(k, _)| k.value > needed_denomination)
                    .collect::<Vec<_>>();
                bigger_bags.sort_by(|(k1, _), (k2, _)| k1.value.cmp(&k2.value));

                for (bigger_denomination, _) in bigger_bags {
                    if let Some(bigger_coin) =
                        self.__remove_coin_from_wallet(bigger_denomination).await?
                    {
                        trace!("carve: splitting coin of value {}", bigger_coin.value);

                        // Before we attempt to carve up the coin we add it back into the inbox
                        // so that if something goes wrong the parts of the coin can be recovered
                        // on the next reconcile. Otherwise on success the this will pick up
                        // all the carved coins anyway
                        self.wallet
                            .inbox
                            .push_with_dio(&self.dio, bigger_coin.owner.clone())?;
                        self.dio.commit().await?;

                        // Now we carve the coin and collect the results
                        coin_carve_command(
                            &self.registry,
                            bigger_coin.owner.clone(),
                            bigger_coin.coin,
                            needed_denomination,
                            bigger_coin.owner.token,
                            self.auth.clone(),
                        )
                        .await?;
                        break;
                    }
                }
                break;
            }

            // Make sure we have the right amount in the bag (if not we have to try again)
            if ret_bag_total != needed_total_amount {
                trace!(
                    "carve: attempt failed - not enough change - found={} needed={}",
                    ret_bag_total,
                    needed_total_amount
                );
                continue;
            }

            // We need to add all the coins back into the inbox processor so that if
            // they are lost in the future rotation or transfer operations that the
            // original wallet can claim them back
            if auto_recover_coins {
                let mut wallet = self.wallet.as_mut();
                let ancestors = ret_bag.to_ownerships().await;
                for ancestor in ancestors {
                    wallet.inbox.push(ancestor)?;
                }
            }

            // Now we commit the transaction which gets us ready for the rotates
            self.dio.commit().await?;

            // Return the bag of coins
            return Ok(ret_bag);
        }

        // We gave it a good go but for some reason we just can't
        // seem to build a bag of coins to represent this amount
        debug!("insufficient coins: loop count exceeded");
        bail!(WalletErrorKind::InsufficientCoins);
    }
}
