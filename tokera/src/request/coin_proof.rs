use crate::model::*;
use ate::crypto::SignedProtectedData;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinProofInner {
    /// Amount to be deposited into this account
    pub amount: Decimal,
    /// National currency to be deposited into this account (e.g. aud,eur,gbp,usd,hkd)
    pub currency: NationalCurrency,
    /// Who has to pay for this invoice
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoinProof {
    /// Proof that the caller has write access to the account specified
    pub inner: SignedProtectedData<CoinProofInner>,
}
