#![allow(unused_imports)]
use ate::prelude::*;
use error_chain::bail;
use fxhash::FxHashSet;
use std::ops::Deref;
use std::sync::Arc;
use tracing::{debug, error, info, trace, warn};

use crate::api::DeployApi;
use crate::error::*;
use crate::model::*;

impl DeployApi {
    pub(super) async fn __get_bag(
        &mut self,
        denomination: Denomination,
    ) -> Result<Option<DaoMut<BagOfCoins>>, WalletError> {
        let ret = self.wallet.as_mut().bags.get_mut(&denomination).await?;
        Ok(ret)
    }

    pub(super) async fn __get_or_create_bag(
        &mut self,
        denomination: Denomination,
    ) -> Result<DaoMut<BagOfCoins>, WalletError> {
        let ret = self
            .wallet
            .as_mut()
            .bags
            .get_or_default(denomination)
            .await?;
        Ok(ret)
    }

    pub async fn add_coin_to_wallet(&mut self, coin: CarvedCoin) -> Result<(), WalletError> {
        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }

        // Add the coin to the chest
        self.__add_coin_to_wallet(coin).await?;

        // Unlock and return the result
        self.wallet.unlock().await?;
        Ok(())
    }

    pub async fn add_coins_to_wallet(
        &mut self,
        coins: impl IntoIterator<Item = CarvedCoin>,
    ) -> Result<(), WalletError> {
        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }

        // Add the coins to the chest
        for coin in coins {
            self.__add_coin_to_wallet(coin).await?;
        }

        // Unlock and return the result
        self.wallet.unlock().await?;
        Ok(())
    }

    pub(super) async fn __add_coin_to_wallet(
        &mut self,
        coin: CarvedCoin,
    ) -> Result<(), WalletError> {
        // Get or create the chest
        let mut bag = self
            .__get_or_create_bag(Denomination {
                value: coin.value,
                currency: coin.currency,
            })
            .await?;
        let mut active_bag = bag.as_mut();

        // If it exists then ignore it
        if active_bag.coins.iter().any(|c| c.coin == coin.coin) {
            trace!(
                "ignoing coin (value={}{}) - already in wallet",
                coin.value,
                coin.currency
            );
            return Ok(());
        }

        // Add the coin to the active wallet
        trace!(
            "adding coin to wallet (value={}{})",
            coin.value,
            coin.currency
        );
        active_bag.coins.push(coin);
        Ok(())
    }

    pub async fn remove_coin_from_wallet(
        &mut self,
        denomination: Denomination,
    ) -> Result<(), WalletError> {
        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }

        // Remove the coin from the chest
        self.__remove_coin_from_wallet(denomination).await?;

        // Unlock and return the result
        self.wallet.unlock().await?;
        Ok(())
    }

    pub(super) async fn __remove_coin_from_wallet(
        &mut self,
        denomination: Denomination,
    ) -> Result<Option<CarvedCoin>, WalletError> {
        // Get or create the chest
        let mut bag = match self.__get_bag(denomination).await? {
            Some(a) => a,
            None => {
                return Ok(None);
            }
        };
        let mut bag = bag.as_mut();

        // Extract a coin from the chest (if there are any left)
        let ret = bag.coins.pop();
        Ok(ret)
    }
}
