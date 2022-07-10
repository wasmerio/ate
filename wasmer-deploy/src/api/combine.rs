use ate::prelude::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::cmd::*;
use crate::error::*;
use crate::model::*;

use super::*;

impl DeployApi {
    pub(super) async fn __combine_coins(&mut self) -> Result<(), CoinError> {
        // Loop through all the chests
        let mut wallet_mut = self.wallet.as_mut();
        let bags = wallet_mut.bags.iter_mut().await?.collect::<Vec<_>>();
        for (_, mut bag) in bags {
            // If the active bag is not full enough yet then move onto the next one
            let loose_coins = bag.coins.len();
            let coins_per_zipped_bag_plus_plus =
                ((COINS_PER_STACK_TO_BE_COMBINED * 12usize) / 10usize) + 1usize;
            if loose_coins <= coins_per_zipped_bag_plus_plus {
                continue;
            }
            trace!("enough loose coins ({}) to combine", loose_coins);

            // Start a new bag and take out the margin we use to stop combine thrashing
            let mut bag_mut = bag.as_mut();
            let mut bag_to_combine = BagOfCoins::default();
            while let Some(coin) = bag_mut.coins.pop() {
                bag_to_combine.coins.push(coin);
                if bag_to_combine.coins.len() >= COINS_PER_STACK_TO_BE_COMBINED {
                    break;
                }
            }
            drop(bag_mut);
            drop(bag);

            // Create a new ownership for the coins
            let mut new_ownership = match bag_to_combine.to_ownerships().await.into_iter().next() {
                Some(a) => a,
                None => {
                    continue;
                }
            };
            new_ownership.token = EncryptKey::generate(new_ownership.token.size());
            new_ownership.what = PrimaryKey::generate();
            trace!(
                "built a bag to combine with {} coins",
                bag_to_combine.coins.len()
            );

            // Create a rollback and commit the change (so that if something
            // goes wrong we can recover the coins) - we also add the new ownership
            // here so the coin can be collected
            let temp_ownership = {
                let mut to = Vec::new();
                let ownership = bag_to_combine.to_ownerships().await;
                trace!(
                    "adding ({}) ownership(s) as fallback coins",
                    ownership.len()
                );
                for ownership in ownership {
                    to.push(wallet_mut.inbox.push_with_dio(&self.dio, ownership)?);
                }
                to.push(
                    wallet_mut
                        .inbox
                        .push_with_dio(&self.dio, new_ownership.clone())?,
                );
                to
            };
            wallet_mut.commit()?;
            self.dio.commit().await?;

            trace!("combining bag of coins...");
            let res = coin_combine_command(
                &self.registry,
                bag_to_combine.coins,
                new_ownership,
                self.auth.clone(),
            )
            .await?;

            // We can now add the coin and remove the rollback objects
            let denomination = Denomination {
                currency: res.super_coin.currency.clone(),
                value: res.super_coin.value.clone(),
            };
            wallet_mut
                .bags
                .get_or_default(denomination)
                .await?
                .as_mut()
                .coins
                .push(res.super_coin);
            wallet_mut.commit()?;
            for temp in temp_ownership {
                temp.delete()?;
            }
            self.dio.commit().await?;
        }
        Ok(())
    }
}
