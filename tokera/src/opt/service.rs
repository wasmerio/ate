use clap::Parser;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsService {
    /// Type of services to perform an action upon
    #[clap(subcommand)]
    pub purpose: OptsServiceFor,
}

#[derive(Parser, Clone)]
pub enum OptsServiceFor {
    /// Services associated to you personally
    #[clap()]
    Personal(OptsServiceForPersonal),
    /// Services associated with a particular domain you can authorize on behalf of
    #[clap()]
    Domain(OptsServiceForDomain),
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsServiceForPersonal {
    /// Name of the personal wallet to use in this context (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the wallet
    #[clap(subcommand)]
    pub action: OptsServiceAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsServiceForDomain {
    /// Name of the domain that the wallet is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the domain wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the wallet
    #[clap(subcommand)]
    pub action: OptsServiceAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsSubscribe {
    /// Name of the service to be subscribed to
    /// (refer to the list command for available services)
    #[clap(index = 1)]
    pub service_name: String,
    /// Forces the contract to be created even if one already exists
    #[clap(short, long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsServiceDetails {
    /// Name of the service to retrieve more details for
    #[clap(index = 1)]
    pub service_name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub enum OptsServiceAction {
    /// Lists all the services available under this context
    #[clap()]
    List,
    /// Displays the details of a specific service
    #[clap()]
    Details(OptsServiceDetails),
    /// Subscribes to a particular service
    #[clap()]
    Subscribe(OptsSubscribe),
}

impl OptsPurpose<OptsServiceAction> for OptsServiceFor {
    fn purpose(&self) -> Purpose<OptsServiceAction> {
        match self {
            OptsServiceFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
            OptsServiceFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
        }
    }
}
