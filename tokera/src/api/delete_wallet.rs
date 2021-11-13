#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace};
use error_chain::*;

use crate::error::*;

use super::*;

impl TokApi
{
    pub async fn is_wallet_empty(&self) -> Result<bool, WalletError>
    {
        if self.wallet.inbox.iter().await?.next().is_some() == true {
            return Ok(false);
        }
        for (_, bag) in self.wallet.bags.iter().await? {
            if bag.coins.len() > 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
    
    pub async fn delete_wallet(self, force: bool) -> Result<(), WalletError>
    {
        if self.is_wallet_empty().await? == false && force == false {
            bail!(WalletErrorKind::WalletNotEmpty); 
        }
    
        self.wallet.delete()?;
        self.dio.commit().await?;
        Ok(())
    }
}