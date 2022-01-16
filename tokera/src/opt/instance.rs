use clap::Parser;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsInstance {
    /// Category of instances to perform an action upon
    #[clap(subcommand)]
    pub purpose: OptsInstanceFor,
}

#[derive(Parser, Clone)]
pub enum OptsInstanceFor {
    /// Instances associated to you personally
    #[clap()]
    Personal(OptsInstanceForPersonal),
    /// Instances associated with a particular group you can authorize on behalf of
    #[clap()]
    Domain(OptsInstanceForDomain),
}

impl OptsInstanceFor {
    pub fn action<'a>(&'a self) -> &'a OptsInstanceAction {
        match self {
            OptsInstanceFor::Personal(a) => &a.action,
            OptsInstanceFor::Domain(a) => &a.action,
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceForPersonal {
    /// Name of the personal wallet to use for this instance (if required)
    #[clap(index = 1, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the instance
    #[clap(subcommand)]
    pub action: OptsInstanceAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceForDomain {
    /// Name of the group that the instance is attached to
    #[clap(index = 1)]
    pub domain: String,
    /// Name of the group wallet to use in this context (if required)
    #[clap(index = 2, default_value = "default")]
    pub wallet_name: String,
    /// Action to perform on the instance
    #[clap(subcommand)]
    pub action: OptsInstanceAction,
}

#[derive(Parser, Clone)]
#[clap()]
pub enum OptsInstanceAction {
    /// Lists all the active instances
    #[clap()]
    List,
    /// Details the details of a particular active instance
    #[clap()]
    Details(OptsInstanceDetails),
    /// Starts a new instance
    #[clap()]
    Start(OptsInstanceStart),
    /// Stops are particular instance
    #[clap()]
    Stop(OptsInstanceStop),
    /// Restarts a particular instance
    #[clap()]
    Restart(OptsInstanceRestart),
    /// Backs up a particular instance
    #[clap()]
    Backup(OptsInstanceBackup),
    /// Restores a particular instance from a previous backup
    #[clap()]
    Restore(OptsInstanceRestore),
    /// Updates a particular instance to the latest version of its running software
    #[clap()]
    Upgrade(OptsInstanceUpgrade),
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceDetails {
    /// Token of the instance to get details for
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceStart {
    /// Name of the web assembly package to be started
    #[clap(index = 1)]
    pub wapm: String,
    /// Stateful instances have persistent file systems that can be backed up and restored
    #[clap(short, long)]
    pub stateful: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceStop {
    /// Token of the instance to be stopped
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceRestart {
    /// Token of the instance to be restarted
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceBackup {
    /// Token of the instance to be backed up
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceRestore {
    /// Token of the instance to be restored
    #[clap(index = 1)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceUpgrade {
    /// Token of the instance to be upgrades
    #[clap(index = 1)]
    pub token: String,
}

impl OptsPurpose<OptsInstanceAction> for OptsInstanceFor {
    fn purpose(&self) -> Purpose<OptsInstanceAction> {
        match self {
            OptsInstanceFor::Personal(a) => Purpose::Personal {
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
            OptsInstanceFor::Domain(a) => Purpose::Domain {
                domain_name: a.domain.clone(),
                wallet_name: a.wallet_name.clone(),
                action: a.action.clone(),
            },
        }
    }
}
