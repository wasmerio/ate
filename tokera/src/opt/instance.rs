use clap::Parser;
use url::Url;

use super::purpose::*;

#[allow(dead_code)]
#[derive(Parser, Clone)]
#[clap(version = "1.5", author = "Tokera Pty Ltd <info@tokera.com>")]
pub struct OptsInstance {
    /// Category of instances to perform an action upon
    #[clap(subcommand)]
    pub purpose: OptsInstanceFor,
    /// URL where the data is remotely stored on a distributed commit log (e.g. wss://tokera.sh/db).
    #[clap(short, long)]
    pub db_url: Option<Url>,
    /// URL where the instances can be accessed from (e.g. wss://tokera.sh/sess)
    #[clap(short, long)]
    pub sess_url: Option<Url>,
    /// Indicates that the server certificate should be ignored
    #[clap(long)]
    pub ignore_certificate: bool
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

    pub fn is_personal(&self) -> bool {
        if let OptsInstanceFor::Personal(..) = self {
            true
        } else {
            false
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
    /// Creates a new instance
    #[clap()]
    Create(OptsInstanceCreate),
    /// Exports an interface from a particular instance
    #[clap()]
    Export(OptsInstanceExport),
    /// Deports a previously exposed interface for a particular instance
    #[clap()]
    Deport(OptsInstanceDeport),
    /// Kills are particular instance - killed instances are totally destroyed
    #[clap()]
    Kill(OptsInstanceKill),
    /// Switches to a shell that runs against a particular instance
    #[clap()]
    Shell(OptsInstanceShell),
    /// Calls a function within a process in a particular instance
    #[clap()]
    Call(OptsInstanceCall),
    /// Mount an existing instance file system to a particular path
    #[clap()]
    Mount(OptsInstanceMount),
    /// Clones a particular instance
    #[clap()]
    Clone(OptsInstanceClone),
}

impl OptsInstanceAction
{
    pub fn needs_sudo(&self) -> bool {
        /*
        match self {
            OptsInstanceAction::Create(_) => true,
            OptsInstanceAction::Kill(_) => true,
            OptsInstanceAction::Restore(_) => true,
            OptsInstanceAction::Upgrade(_) => true,
            OptsInstanceAction::Stop(_) => true,
            OptsInstanceAction::Start(_) => true,
            _ => false,
        }
        */
        true
    }

    pub fn name(&self) -> Option<String> {
        match self {
            OptsInstanceAction::List => None,
            OptsInstanceAction::Create(_) => None,
            OptsInstanceAction::Details(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Kill(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Clone(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Shell(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Call(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Export(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Deport(opts) => Some(opts.name.clone()),
            OptsInstanceAction::Mount(opts) => Some(opts.name.clone()),
        }
    }
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceDetails {
    /// Token of the instance to get details for
    #[clap(index = 1)]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceCreate {
    /// Name of the new instance (which will be generated if you dont supply one)
    #[clap(index = 1)]
    pub name: Option<String>,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceExport {
    /// Name of the instance to export an interface from
    #[clap(index = 1)]
    pub name: String,
    /// Name of the binary that will be exported
    #[clap(index = 2)]
    pub binary: String,
    /// Distributed instances run all over the world concurrently on the same file system
    /// They are started on-demand as needed and shutdown when idle.
    #[clap(short, long)]
    pub distributed: bool,
    /// Indicates if the exported endpoint will be accessible via https (API calls)
    #[clap(long)]
    pub no_https: bool,
    /// Indicates if the exported endpoint will be accessible via wasm-bus
    #[clap(long)]
    pub no_bus: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceDeport {
    /// Name of the instance to export an interface from
    #[clap(index = 1)]
    pub name: String,
    /// Token of the exported interface to be deleted
    #[clap(index = 2)]
    pub token: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceKill {
    /// Name of the instance to be killed
    /// (killed instances are perminently destroyed)
    #[clap(index = 1)]
    pub name: String,
    /// Forces the removal of the instance from the wallet even
    /// if access is denied to its data and thus this would create an orphan chain.
    #[clap(short, long)]
    pub force: bool,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceClone {
    /// Name of the instance to be cloned
    #[clap(index = 1)]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceShell {
    /// Name of the instance to run commmands against
    #[clap(index = 1)]
    pub name: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceCall {
    /// Name of the instance to run commmands against
    #[clap(index = 1)]
    pub name: String,
    /// WAPM name of the process to be invoked
    #[clap(index = 2)]
    pub binary: String,
    /// Topic of the invocation call
    #[clap(index = 3)]
    pub topic: String,
}

#[derive(Parser, Clone)]
#[clap()]
pub struct OptsInstanceMount {
    /// Name of the instance to mounted
    #[clap(index = 1)]
    pub name: String,
    /// Path that the instance will be mounted at
    #[clap(index = 2)]
    pub path: String,
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
