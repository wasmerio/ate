use ate::prelude::*;
use clap::Clap;
use url::Url;

#[derive(Clap)]
#[clap(version = "1.5", author = "John S. <johnathan.sharratt@gmail.com>")]
pub struct Opts {
    /// Sets the level of log verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
    /// URL where the user is authenticated
    #[clap(short, long, default_value = "ws://tokera.com/auth")]
    pub auth: Url,
    /// Token used to access your encrypted file-system (if you do not supply a token then you will
    /// be prompted for a username and password)
    #[clap(short, long)]
    pub token: Option<String>,
    /// Token file to read that holds a previously created token to be used to access your encrypted
    /// file-system (if you do not supply a token then you will be prompted for a username and password)
    #[clap(long)]
    pub token_path: Option<String>,
    /// Logs debug info to the console
    #[clap(short, long)]
    pub debug: bool,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Users are personal accounts and services that have an authentication context
    #[clap()]
    User(OptsUser),
    /// Groups are collections of users that share something together
    #[clap()]
    Group(OptsGroup),
    /// Tokens are stored authentication and authorization secrets used by other processes
    #[clap()]
    Token(OptsToken),
}

#[derive(Clap)]
#[clap()]
pub struct OptsUser {
    #[clap(subcommand)]
    pub action: UserAction,
}

#[derive(Clap)]
pub enum UserAction {
    /// Creates a new user and generates login credentials
    #[clap()]
    Create(CreateUser),
    /// Returns all the details about a specific user
    #[clap()]
    Details,
}

/// Creates a new user and login credentials on the authentication server
#[derive(Clap)]
pub struct CreateUser {
    /// Email address of the user to be created
    #[clap(index = 1)]
    pub email: Option<String>,
    /// New password to be associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
}

#[derive(Clap)]
#[clap()]
pub struct OptsGroup {
    #[clap(subcommand)]
    pub action: GroupAction,
}

#[derive(Clap)]
pub enum GroupAction {
    /// Creates a new group
    #[clap()]
    Create(CreateGroup),
    /// Adds another user to an existing group
    #[clap()]
    AddUser(GroupAddUser),
    /// Removes a user from an existing group
    #[clap()]
    RemoveUser(GroupRemoveUser),
    /// Display the details about a particular group (token is required to see role membership)
    #[clap()]
    Details(GroupDetails),
}

/// Creates a new group using the login credentials provided or prompted for
#[derive(Clap)]
pub struct CreateGroup {
    /// Name of the group to be created
    #[clap(index = 1)]
    pub group: String,
}

/// Adds a particular user to a role within a group
#[derive(Clap)]
pub struct GroupAddUser {
    /// Name of the group that the user will be added to
    #[clap(index = 1)]
    pub group: String,
    /// Role within the group that the user will be added to, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    pub role: AteRolePurpose,
    /// Username that will be added to the group role
    #[clap(index = 3)]
    pub username: String,
}

/// Removes a particular user from a role within a group
#[derive(Clap)]
pub struct GroupRemoveUser {
    /// Name of the group that the user will be removed from
    #[clap(index = 1)]
    pub group: String,
    /// Role within the group that the user will be removed from, must be one of the following
    /// [owner, delegate, contributor, observer, other-...]. Only owners and delegates can
    /// modify the groups. Generally write actions are only allowed by members of the
    /// 'contributor' role and all read actions require the 'observer' role.
    #[clap(index = 2)]
    pub role: AteRolePurpose,
    /// Username that will be removed to the group role
    #[clap(index = 3)]
    pub username: String,
}

/// Display the details about a particular group
#[derive(Clap)]
pub struct GroupDetails {
    /// Name of the group to query
    #[clap(index = 1)]
    pub group: String,
    /// Determines if sudo permissions should be sought
    #[clap(long)]
    pub sudo: bool
}

#[derive(Clap)]
#[clap()]
pub struct OptsToken {
    #[clap(subcommand)]
    pub action: TokenAction,
}

#[derive(Clap)]
pub enum TokenAction {
    /// Generate a token with normal permissions from the supplied username and password
    #[clap()]
    Generate(GenerateToken),
    /// Generate a token with extra permissions with elevated rights to modify groups and other higher risk actions
    #[clap()]
    Sudo(CreateTokenSudo),
    /// Gather the permissions needed to access a specific group into the token using either another supplied token or the prompted credentials
    #[clap()]
    Gather(GatherPermissions),
    /// Views the contents of the supplied token
    #[clap()]
    View(ViewToken),
}

/// Logs into the authentication server using the supplied credentials
#[derive(Clap)]
pub struct GenerateToken {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
}

/// Logs into the authentication server using the supplied credentials and 2nd factor authentication
#[derive(Clap)]
pub struct CreateTokenSudo {
    /// Email address that you wish to login using
    #[clap(index = 1)]
    pub email: Option<String>,
    /// Password associated with this account
    #[clap(index = 2)]
    pub password: Option<String>,
    /// Authenticator code from your google authenticator
    #[clap(index = 3)]
    pub code: Option<String>,
}

/// Views the contents of the current token
#[derive(Clap)]
pub struct ViewToken {
}

/// Gathers the permissions needed to access a specific group into the token using either another supplied token or the prompted credentials
#[derive(Clap)]
pub struct GatherPermissions {
    /// Name of the group to gather the permissions for
    #[clap(index = 1)]
    pub group: String,
    /// Determines if sudo permissions should be sought
    #[clap(long)]
    pub sudo: bool
}

pub async fn main_opts_user(opts_user: OptsUser, token: Option<String>, token_path: Option<String>, auth: url::Url) -> Result<(), AteError>{
    match opts_user.action {
        UserAction::Create(action) => {
            let _session = crate::main_create_user(action.email, action.password, auth).await?;
        },
        UserAction::Details => {
            let session = crate::main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            crate::main_user_details(session).await?;
        }
    }
    Ok(())
}

pub async fn main_opts_group(opts_group: OptsGroup, token: Option<String>, token_path: Option<String>, auth: url::Url, hint_group: &str) -> Result<(), AteError>{
    match opts_group.action {
        GroupAction::Create(action) => {
            let session = crate::main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            crate::main_create_group(Some(action.group), auth, Some(session.identity().to_string()), hint_group).await?;
        },
        GroupAction::AddUser(action) => {
            let session = crate::main_session_group(token.clone(), token_path.clone(), action.group.clone(), true, None, Some(auth.clone()), hint_group).await?;
            crate::main_group_user_add(Some(action.role), Some(action.username), auth, &session, hint_group).await?;
        },
        GroupAction::RemoveUser(action) => {
            let session = crate::main_session_group(token.clone(), token_path.clone(), action.group.clone(), true, None, Some(auth.clone()), hint_group).await?;
            crate::main_group_user_remove(Some(action.role), Some(action.username), auth, &session, hint_group).await?;
        },
        GroupAction::Details(action) => {
            if token.is_some() || token_path.is_some() {
                let session = crate::main_session_group(token.clone(), token_path.clone(), action.group.clone(), action.sudo, None, Some(auth.clone()), hint_group).await?;
                crate::main_group_details(Some(action.group), auth, Some(&session), hint_group).await?;
            } else {
                crate::main_group_details(Some(action.group), auth, None, hint_group).await?;
            }
        }
    }
    Ok(())
}

pub async fn main_opts_token(opts_token: OptsToken, token: Option<String>, token_path: Option<String>, auth: url::Url, hint_group: &str) -> Result<(), AteError>{
    match opts_token.action {
        TokenAction::Generate(action) => {
            let session = crate::main_login(action.email, action.password, auth).await?;
            eprintln!("The token string below can be used to secure your file system.\n");
            
            let session: AteSessionType = session.into();
            println!("{}", crate::session_to_b64(session).unwrap());
        },
        TokenAction::Sudo(action) => {
            let session = crate::main_login(action.email, action.password, auth.clone()).await?;
            let session = crate::main_sudo(session, action.code, auth).await?;
            eprintln!("The token string below can be used to secure your file system.\n");

            let session: AteSessionType = session.into();
            println!("{}", crate::session_to_b64(session).unwrap());
        },
        TokenAction::Gather(action) => {
            let session = crate::main_session_group(token.clone(), token_path.clone(), action.group, action.sudo, None, Some(auth.clone()), hint_group).await?;
            eprintln!("The token string below can be used to secure your file system.\n");
            
            let session: AteSessionType = session.into();
            println!("{}", crate::session_to_b64(session).unwrap());
        },
        TokenAction::View(_action) => {
            let session = crate::main_session_user(token.clone(), token_path.clone(), Some(auth.clone())).await?;
            eprintln!("The token contains the following claims.\n");
            println!("{}", session);
        },
    }
    Ok(())
}