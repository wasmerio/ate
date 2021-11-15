use clap::Parser;

use super::purpose::*;
use super::wallet_action::*;

#[derive(Parser, Clone)]
pub enum OptsWalletSource {
    /// One of your personal wallets
    #[clap()]
    Personal(OptWalletPersonal),
    /// Particular wallet attached to a domain group
    #[clap()]
    Domain(OptWalletDomain),
}

impl OptsWalletSource {
    pub fn action<'a>(&'a self) -> &'a OptWalletAction {
        match self {
            OptsWalletSource::Personal(a) => &a.action,
            OptsWalletSource::Domain(a) => &a.action,
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptWalletPersonal {
    /// Name of the personal wallet to perform this action on
    #[clap(index = 1, default_value = "default")]
    pub name: String,
    /// Action to perform on the wallet
    #[clap(subcommand)]
    pub action: OptWalletAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptWalletDomain {
    /// Name of the group that the wallet is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the wallet within this group to perform this action on
    #[clap(index = 2, default_value = "default")]
    pub name: String,
    /// Action to performed in this context
    #[clap(subcommand)]
    pub action: OptWalletAction,
}

impl OptsPurpose<OptWalletAction> for OptsWalletSource {
    fn purpose(&self) -> Purpose<OptWalletAction> {
        match self {
            OptsWalletSource::Personal(a) => Purpose::Personal {
                wallet_name: a.name.clone(),
                action: a.action.clone(),
            },
            OptsWalletSource::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.name.clone(),
                action: a.action.clone(),
            },
        }
    }
}
