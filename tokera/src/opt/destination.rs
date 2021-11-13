use clap::Parser;

use super::purpose::*;

#[derive(Parser, Clone)]
#[clap()]
pub struct OptWalletDestinationPersonal {
    /// Name of the personal wallet
    #[clap(index = 1, default_value = "default")]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptWalletDestinationDomain {
    /// Name of the domain group that the wallet is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the wallet within this domain
    #[clap(index = 2, default_value = "default")]
    pub name: String,
}

#[derive(Parser, Clone)]
pub enum OptsWalletDestination {
    /// One of your personal wallets
    #[clap()]
    Personal(OptWalletDestinationPersonal),
    /// Particular wallet attached to a group
    #[clap()]
    Domain(OptWalletDestinationDomain),
}

impl OptsPurpose<()>
for OptsWalletDestination
{
    fn purpose(&self) -> Purpose<()> {
        match self {
            OptsWalletDestination::Personal(a) => Purpose::Personal {
                wallet_name: a.name.clone(),
                action: ()
            },
            OptsWalletDestination::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.name.clone(),
                action: (),
            }
        }
    }
}