use error_chain::*;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use crate::error::*;

use super::*;

impl TokApi {
    pub async fn reconcile(&mut self) -> Result<(), WalletError> {
        // If anything has been left on the DIO then we need to fail
        // as this loop will invoke cancel during its recovery process
        if self.dio.has_uncommitted() {
            bail!(WalletErrorKind::CoreError(CoreErrorKind::InternalError(ate::utils::obscure_error_str("unable to reconcile the wallet when there are uncommitted transactions on the DIO"))));
        }

        // Lock the wallet
        let lock = self.wallet.try_lock_with_timeout(self.lock_timeout).await?;
        if lock == false {
            bail!(WalletErrorKind::WalletLocked);
        }
        trace!("wallet has been locked");

        let ret = self.__reconcile().await;

        // Unlock and return the result
        self.wallet.unlock().await?;
        self.dio.commit().await?;
        trace!("wallet has been unlocked and DIO committed");
        ret
    }

    pub(super) async fn __reconcile(&mut self) -> Result<(), WalletError> {
        trace!("collecting coins...");
        self.__collect_coins().await?;
        trace!("combining coins...");
        self.__combine_coins().await?;
        Ok(())
    }
}
