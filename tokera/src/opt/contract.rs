use clap::Parser;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsContract {
    /// Category of contracts to perform an action upon
    #[clap(subcommand)]
    pub purpose: OptsContractFor,
}


#[derive(Parser, Clone)]
pub enum OptsContractFor {
    /// Contracts associated to you personally
    #[clap()]
    Personal(OptsContractForPersonal),
    /// Contracts associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsContractForDomain),
}

impl OptsContractFor {
    pub fn action<'a>(&'a self) -> &'a OptsContractAction {
        match self {
            OptsContractFor::Personal(a) => &a.action,
            OptsContractFor::Domain(a) => &a.action,
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsContractForPersonal {
    /// Name of the personal wallet to use in this context (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the wallet
    #[clap(subcommand)]
    pub action: OptsContractAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsContractForDomain {
    /// Name of the group that the wallet is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the wallet
    #[clap(subcommand)]
    pub action: OptsContractAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub enum OptsContractAction
{
    /// Lists all the active contracts
    #[clap()]
    List,
    /// Details the details of a particular active contract
    #[clap()]
    Details(OptsContractDetails),
    /// Cancels a particular contract
    #[clap()]
    Cancel(OptsContractCancel),
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsContractDetails {
    /// Name of the contract to retrieve details for
    #[clap(index = 1)]
    pub reference_number: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsContractCancel {
    /// Name of the contract to be cancelled
    #[clap(index = 1)]
    pub reference_number: String,
}

impl OptsPurpose<OptsContractAction>
for OptsContractFor
{
    fn purpose(&self) -> Purpose<OptsContractAction> {
        match self {
            OptsContractFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
            OptsContractFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            }
        }
    }
}