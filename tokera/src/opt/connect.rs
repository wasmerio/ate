use clap::Parser;
use url::Url;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsConnect {
    /// Category of network to connect to
    #[clap(subcommand)]
    pub purpose: OptsConnectFor,
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://tokera.sh/db).
    #[clap(short, long)]
    pub db_url: Option<Url>,
    /// URL where the network can be accessed from (e.g. wss://tokera.sh/net)
    #[clap(short, long)]
    pub net_url: Option<Url>,
    /// Indicates that the server certificate should be ignored
    #[clap(long)]
    pub ignore_certificate: bool
}

#[derive(Parser, Clone)]
pub enum OptsConnectFor {
    /// Networks associated to you personally
    #[clap()]
    Personal(OptsConnectForPersonal),
    /// Networks associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsConnectForDomain),
}

impl OptsConnectFor {
    pub fn is_personal(&self) -> bool {
        if let OptsConnectFor::Personal(..) = self {
            true
        } else {
            false
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsConnectForPersonal {
    /// Name of the personal wallet to use for this network (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsConnectForDomain {
    /// Name of the group that the network is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
}

impl OptsPurpose<()> for OptsConnectFor {
    fn purpose(&self) -> Purpose<()> {
        match self {
            OptsConnectFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
            OptsConnectFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: (),
            },
        }
    }
}
