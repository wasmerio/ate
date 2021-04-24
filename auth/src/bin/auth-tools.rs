#![allow(unused_imports)]
use log::{info, warn, debug, error};
use url::Url;
use ate::{prelude::*};
use ate_auth::prelude::*;
use clap::Clap;

#[derive(Clap)]
#[clap(version = "0.1", author = "John S. <johnathan.sharratt@gmail.com>")]
struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "tcp://auth.tokera.com:5001/auth")]
    auth: Url,
    /// Token used to access your encrypted file-system (if you do not supply a token then you will
    /// be prompted for a username and password)
    #[clap(short, long)]
    token: Option<String>,
    /// Token file to read that holds a previously created token to be used to access your encrypted
    /// file-system (if you do not supply a token then you will be prompted for a username and password)
    #[clap(long)]
    token_path: Option<String>,
    /// Logs debug info to the console
    #[clap(short, long)]
    debug: bool,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    /// Actions that modify or query users
    #[clap()]
    User(OptsUser),
    /// Actions that modify or query groups
    #[clap()]
    Group(OptsGroup),
    /// Actions that create tokens that are used for authentication
    #[clap()]
    Token(OptsToken),
}

#[derive(Clap)]
#[clap()]
struct OptsUser {
    #[clap(subcommand)]
    action: UserAction,
}

#[derive(Clap)]
enum UserAction {
    /// Creates a new user
    #[clap()]
    Create(CreateUser),
    /// Returns all the details about a specific user
    #[clap()]
    Details,
}

/// Creates a new user and login credentials on the authentication server
#[derive(Clap)]
struct CreateUser {
    /// Email address of the user to be created
    #[clap(index = 1)]
    email: Option<String>,
    /// New password to be associated with this account
    #[clap(index = 2)]
    password: Option<String>,
}

#[derive(Clap)]
#[clap()]
struct OptsGroup {
    #[clap(subcommand)]
    action: GroupAction,
}

#[derive(Clap)]
enum GroupAction {
    /// Creates a new group
    #[clap()]
    Create(CreateGroup),
    /// Adds another user to an existing group
    #[clap()]
    AddUser(GroupAddUser),
    /// Removes a user from an existing group
    #[clap()]
    RemoveUser(GroupRemoveUser),
}

/// Creates a new group using the login credentials provided or prompted for
#[derive(Clap)]
struct CreateGroup {
    /// Name of the group to be created
    #[clap(index = 1)]
    group: String,
}

/// Adds a particular user to a role within a group
#[derive(Clap)]
struct GroupAddUser {
    /// Name of the group that the user will be added to
    #[clap(index = 1)]
    group: String,
    /// Role within the group that the user will be added to, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    role: AteRolePurpose,
    /// Username that will be added to the group role
    #[clap(index = 3)]
    username: String,
}

/// Removes a particular user from a role within a group
#[derive(Clap)]
struct GroupRemoveUser {
    /// Name of the group that the user will be removed from
    #[clap(index = 1)]
    group: String,
    /// Role within the group that the user will be removed from, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    role: AteRolePurpose,
    /// Username that will be removed to the group role
    #[clap(index = 3)]
    username: String,
}

#[derive(Clap)]
#[clap()]
struct OptsToken {
    #[clap(subcommand)]
    action: TokenAction,
}

#[derive(Clap)]
enum TokenAction {
    /// Generate a token with normal permissions from the supplied username and password
    #[clap()]
    Generate(GenerateToken),
    /// Generate a token with extra permissions with elevated rights to modify groups and other higher risk actions
    #[clap()]
    Sudo(CreateTokenSudo),
}

/// Logs into the authentication server using the supplied credentials
#[derive(Clap)]
struct GenerateToken {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    password: Option<String>,
}

/// Logs into the authentication server using the supplied credentials and 2nd factor authentication
#[derive(Clap)]
struct CreateTokenSudo {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    password: Option<String>,
    /// Authenticator code from your google authenticator
    #[clap(index = 3)]
    code: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), AteError>
{
    let opts: Opts = Opts::parse();

    // Prepare the logging
    let mut log_level = match opts.verbose {
        0 => "error",
        1 => "warn",
        2 => "info",
        _ => "debug",
    };
    if opts.debug { log_level = "debug"; }
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Determine what we need to do
    match opts.subcmd {
        SubCommand::User(opts_user) => {
            match opts_user.action {
                UserAction::Create(action) => {
                    let _session = ate_auth::main_create_user(action.email, action.password, opts.auth).await?;
                },
                UserAction::Details => {
                    let session = ate_auth::main_session(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone()), false).await?;
                    let identity = match session.user.identity() {
                        Some(a) => a.clone(),
                        None => {
                            eprintln!("Could not find an identity for the user");
                            std::process::exit(1);
                        }
                    };
                    println!("identity: {}", identity);
                }
            }
        },
        SubCommand::Group(opts_group) => {
            match opts_group.action {
                GroupAction::Create(action) => {
                    let session = ate_auth::main_session(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone()), true).await?;
                    let _session = ate_auth::main_create_group(Some(action.group), opts.auth, session.user.identity().map(|i| i.clone())).await?;
                },
                GroupAction::AddUser(action) => {
                    let session = ate_auth::main_session(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone()), true).await?;
                    let _session = ate_auth::main_group_user_add(Some(action.group), Some(action.role), Some(action.username), opts.auth, &session).await?;
                },
                GroupAction::RemoveUser(action) => {
                    let session = ate_auth::main_session(opts.token.clone(), opts.token_path.clone(), Some(opts.auth.clone()), true).await?;
                    let _session = ate_auth::main_group_user_remove(Some(action.group), Some(action.role), Some(action.username), opts.auth, &session).await?;
                },
            }
        },
        SubCommand::Token(opts_token) => {
            match opts_token.action {
                TokenAction::Generate(action) => {
                    let session = ate_auth::main_login(action.email, action.password, opts.auth).await?;
                    eprintln!("The token string below can be used to secure your file system.\n");
                    println!("{}", ate_auth::session_to_b64(session.clone()).unwrap());
                },
                TokenAction::Sudo(action) => {
                    let session = ate_auth::main_sudo(action.email, action.password, action.code, opts.auth).await?;
                    eprintln!("The token string below can be used to secure your file system.\n");
                    println!("{}", ate_auth::session_to_b64(session.clone()).unwrap());
                },
            }
        }
    }

    // We are done
    Ok(())
}