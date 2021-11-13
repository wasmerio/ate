use serde::*;
use ate::prelude::*;
use chrono::DateTime;
use chrono::Utc;

use super::*;

/// Contracts are agreement between a consumer and provider for
/// particular services. Only brokers may perform actions on
/// active contracts
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contract
{
    /// Reference number assigned to this contract
    pub reference_number: String,
    /// The country that you pay GST tax in for this services
    pub gst_country: Country,
    /// The wallet that will be debited
    pub debit_wallet: PrimaryKey,
    /// The rate card that will be used for this contract
    pub rate_card: RateCard,
    /// The advertised service being consumed by the provider
    pub service: AdvertisedService,
    /// Status of the contract
    pub status: ContractStatus,
    /// Limited duration contracts will expire after a
    /// certain period of time without incurring further
    /// charges
    pub expires: Option<DateTime<Utc>>,
    /// Key used by the broker to gain access to the wallet
    /// (only after the provider supplies their key)
    pub broker_unlock_key: EncryptKey,
    /// Broker key encrypted with the providers public key
    pub broker_key: PublicEncryptedSecureData<EncryptKey>,
    /// Metrics for difference instance of this service with
    /// unqiue reference numbers (field=related_to)
    pub metrics: DaoVec<ContractMetrics>,
}