pub mod activities {
    use crate::model::*;
    use chrono::prelude::*;
    use serde::*;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WalletCreated {
        pub when: DateTime<Utc>,
        pub by: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct DepositCreated {
        pub when: DateTime<Utc>,
        pub by: String,
        pub invoice_number: String,
        pub invoice_id: String,
        pub amount: Decimal,
        pub currency: NationalCurrency,
        pub pay_url: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct DepositCompleted {
        pub when: DateTime<Utc>,
        pub by: String,
        pub invoice_number: String,
        pub amount: Decimal,
        pub currency: NationalCurrency,
        pub invoice_url: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct FundsTransferred {
        pub when: DateTime<Utc>,
        pub by: String,
        pub amount: Decimal,
        pub currency: NationalCurrency,
        pub from: String,
        pub to: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct FundsWithdrawn {
        pub when: DateTime<Utc>,
        pub by: String,
        pub amount_less_fees: Decimal,
        pub fees: Decimal,
        pub currency: NationalCurrency,
        pub receipt_number: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ContractCreated {
        pub when: DateTime<Utc>,
        pub by: String,
        pub service: AdvertisedService,
        pub contract_reference: String,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ContractInvoice {
        pub when: DateTime<Utc>,
        pub by: String,
        pub invoice: Invoice,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InstanceCreated {
        pub when: DateTime<Utc>,
        pub by: String,
        pub wapm: String,
        pub alias: Option<String>,
        pub stateful: bool,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InstanceDestroyed {
        pub when: DateTime<Utc>,
        pub by: String,
        pub wapm: String,
        pub alias: Option<String>,
    }
}

use chrono::prelude::*;
use num_traits::*;
use serde::*;

use crate::model::*;

use activities::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HistoricActivity {
    WalletCreated(WalletCreated),
    DepositCreated(DepositCreated),
    DepositCompleted(DepositCompleted),
    TransferOut(FundsTransferred),
    TransferIn(FundsTransferred),
    FundsWithdrawn(FundsWithdrawn),
    ContractCreated(ContractCreated),
    ContractCharge(ContractInvoice),
    ContractIncome(ContractInvoice),
    InstanceCreated(InstanceCreated),
    InstanceDestroyed(InstanceDestroyed),
}

impl HistoricActivity {
    pub fn when(&self) -> &DateTime<Utc> {
        match self {
            HistoricActivity::WalletCreated(a) => &a.when,
            HistoricActivity::DepositCreated(a) => &a.when,
            HistoricActivity::DepositCompleted(a) => &a.when,
            HistoricActivity::TransferIn(a) => &a.when,
            HistoricActivity::TransferOut(a) => &a.when,
            HistoricActivity::FundsWithdrawn(a) => &a.when,
            HistoricActivity::ContractCreated(a) => &a.when,
            HistoricActivity::ContractCharge(a) => &a.when,
            HistoricActivity::ContractIncome(a) => &a.when,
            HistoricActivity::InstanceCreated(a) => &a.when,
            HistoricActivity::InstanceDestroyed(a) => &a.when,
        }
    }

    pub fn by(&self) -> &str {
        match self {
            HistoricActivity::WalletCreated(a) => a.by.as_str(),
            HistoricActivity::DepositCreated(a) => a.by.as_str(),
            HistoricActivity::DepositCompleted(a) => a.by.as_str(),
            HistoricActivity::TransferIn(a) => a.by.as_str(),
            HistoricActivity::TransferOut(a) => a.by.as_str(),
            HistoricActivity::FundsWithdrawn(a) => a.by.as_str(),
            HistoricActivity::ContractCreated(a) => a.by.as_str(),
            HistoricActivity::ContractCharge(a) => a.by.as_str(),
            HistoricActivity::ContractIncome(a) => a.by.as_str(),
            HistoricActivity::InstanceCreated(a) => a.by.as_str(),
            HistoricActivity::InstanceDestroyed(a) => a.by.as_str(),
        }
    }

    pub fn financial<'a>(&'a self) -> Option<HistoricFinancialActivity<'a>> {
        match self {
            HistoricActivity::WalletCreated(_) => None,
            HistoricActivity::DepositCreated(_) => None,
            HistoricActivity::InstanceCreated(_) => None,
            HistoricActivity::InstanceDestroyed(_) => None,
            HistoricActivity::ContractCreated(_) => None,
            HistoricActivity::ContractCharge(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: Decimal::zero() - a.invoice.total_due,
                currency: a.invoice.currency,
            }),
            HistoricActivity::ContractIncome(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: a.invoice.total_due,
                currency: a.invoice.currency,
            }),
            HistoricActivity::DepositCompleted(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: a.amount,
                currency: a.currency,
            }),
            HistoricActivity::TransferIn(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: a.amount,
                currency: a.currency,
            }),
            HistoricActivity::TransferOut(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: Decimal::zero() - a.amount,
                currency: a.currency,
            }),
            HistoricActivity::FundsWithdrawn(a) => Some(HistoricFinancialActivity {
                activity: self,
                amount: Decimal::zero() - (a.amount_less_fees + a.fees),
                currency: a.currency,
            }),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            HistoricActivity::WalletCreated(_) => {
                format!("Wallet created")
            }
            HistoricActivity::ContractCreated(a) => {
                format!("Contract created ({})", a.contract_reference)
            }
            HistoricActivity::ContractCharge(a) => {
                format!("Invoice was paid ({})", a.invoice.related_to)
            }
            HistoricActivity::ContractIncome(a) => {
                format!("Invoice was paid ({})", a.invoice.related_to)
            }
            HistoricActivity::DepositCreated(a) => {
                format!("Deposit invoiced ({})", a.invoice_number)
            }
            HistoricActivity::DepositCompleted(a) => {
                format!("Deposit was paid ({})", a.invoice_number)
            }
            HistoricActivity::TransferIn(a) => {
                format!("Transfer from {}", a.from)
            }
            HistoricActivity::TransferOut(a) => {
                format!("Transfer to {}", a.to)
            }
            HistoricActivity::FundsWithdrawn(_) => {
                format!("Funds withdrawn")
            }
            HistoricActivity::InstanceCreated(a) => {
                if let Some(alias) = &a.alias {
                    format!("Instance created ({} with alias {})", a.wapm, alias)
                } else {
                    format!("Instance created ({})", a.wapm)
                }
            }
            HistoricActivity::InstanceDestroyed(a) => {
                if let Some(alias) = &a.alias {
                    format!("Instance destroyed ({} with alias {})", a.wapm, alias)
                } else {
                    format!("Instance destroyed ({})", a.wapm)
                }
            }
        }
    }

    pub fn details(&self) -> std::result::Result<String, serde_json::Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

pub struct HistoricFinancialActivity<'a> {
    pub activity: &'a HistoricActivity,
    pub currency: NationalCurrency,
    pub amount: Decimal,
}
