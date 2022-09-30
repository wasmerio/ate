use super::cconst::CConst;
use ate::prelude::*;
use wasmer_auth::error::*;
use wasmer_auth::helper::*;
use wasmer_auth::request::*;
use std::borrow::Cow;
use std::sync::Arc;
use thrussh::server::*;
use thrussh_keys::key::PublicKey;
use wasmer_term::wasmer_os::api as term_api;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SshWizardStep {
    /// The first step before we take any actions to login
    Init,
    /// At this step the user is prompted for their password
    Login,
    /// At this step the user is prompted for their authenticator code
    Sudo,
    /// If the user is not known to Wasmer then they enter a
    /// sign-up step that gets them to enter a new password
    SignUp,
    /// Next they must agree to the terms and conditions
    Terms,
    /// Lastly they need to verify their email address with
    /// the verification code that was sent to them. Also
    /// during this step the user must scan the QR code
    /// and save it to their phone for future use
    Verify,
    /// Lastly we enter the main shell
    Shell,
    /// Indicates that the session will be terminated
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoginResult {
    LoginWithUser,
    LoginWithPublicKey,
    NoPasswordSupplied,
    IncorrectPassword,
    Unregistered,
    InvalidEmail,
    AccountLocked,
    Unverified,
    InternalError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UserExistsResult {
    Found,
    NotFound,
    InvalidEmail,
    InternalError,
    Banned,
    Suspended,
}

#[derive(Debug, Clone)]
pub enum UserCreateResult {
    NoUsernameSupplied,
    NoPasswordSupplied,
    AlreadyExists,
    InvalidEmail,
    InternalError,
    Created,
    Terms,
}

#[derive(Debug, Clone)]
pub enum SudoResult {
    Success,
    NoAuthenticatorCodeSupplied,
    WrongCode,
    InternalError,
}

#[derive(Debug, Default)]
pub struct SshWizardState {
    pub welcome: Option<String>,
    pub email: Option<String>,
    pub session: Option<AteSessionType>,
    pub public_key: Option<PublicKey>,
    pub password: Option<String>,
    pub verify_code: Option<String>,
    pub sudo_code: Option<String>,
    pub goodbye_message: Option<String>,
    pub message_of_the_day: Option<String>,
    pub needed_terms: Option<String>,
    pub accepted_terms: Option<String>,
    pub qr_code: Option<String>,
}

impl SshWizardState {
    pub fn set_public_key(&mut self, key: PublicKey) {
        self.public_key = Some(key);
    }

    pub fn set_welcome(&mut self, msg: String) {
        self.welcome = Some(msg);
    }

    pub fn parse_message(&self, msg: &str) -> String {
        msg.replace(
            "{email}",
            self.email.clone().unwrap_or("none".to_string()).as_str(),
        )
        .replace(
            "{terms}",
            self.needed_terms
                .clone()
                .unwrap_or("none".to_string())
                .as_str(),
        )
        .replace(
            "{qr_code}",
            self.qr_code.clone().unwrap_or("none".to_string()).as_str(),
        )
    }
}

pub struct SshWizard {
    pub step: SshWizardStep,
    pub state: SshWizardState,
    pub registry: Arc<Registry>,
    pub auth: url::Url,
}

impl SshWizard {
    async fn process_response<'a>(&mut self, response: Option<Vec<String>>) {
        match self.step.clone() {
            SshWizardStep::Init => {
                if self.state.session.is_some() {
                    self.step = SshWizardStep::Sudo
                } else {
                    let username = match self.state.email.clone() {
                        Some(a) => a,
                        None => {
                            warn!("user check failed: no username supplied");
                            self.state.goodbye_message =
                                Some(CConst::SSH_INVALID_USERNAME.to_string());
                            self.step = SshWizardStep::Terminate;
                            return;
                        }
                    };
                    let regex = regex::Regex::new("^([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})@([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,}).([a-z0-9\\.!#$%&'*+/=?^_`{|}~-]{1,})$").unwrap();
                    if regex.is_match(username.as_str()) == false {
                        warn!(
                            "user check failed: username was invalid (user={})",
                            username
                        );
                        self.state.goodbye_message = Some(CConst::SSH_INVALID_USERNAME.to_string());
                        self.step = SshWizardStep::Terminate;
                        return;
                    }

                    self.step = match user_exists(&self.registry, &self.auth, &mut self.state).await
                    {
                        UserExistsResult::Found => {
                            debug!("user found (user={})", self.state.email.clone().unwrap());
                            SshWizardStep::Login
                        }
                        UserExistsResult::InvalidEmail => {
                            info!("user check failed: invalid email");
                            self.state.goodbye_message =
                                Some(CConst::SSH_INVALID_USERNAME.to_string());
                            SshWizardStep::Terminate
                        }
                        UserExistsResult::Banned | UserExistsResult::Suspended => {
                            info!("user check failed: banned or suspended");
                            self.state.goodbye_message =
                                Some(CConst::SSH_ACCOUNT_LOCKED.to_string());
                            SshWizardStep::Terminate
                        }
                        UserExistsResult::NotFound => {
                            info!(
                                "user does not exist (user={})",
                                self.state.email.clone().unwrap()
                            );
                            SshWizardStep::SignUp
                        }
                        UserExistsResult::InternalError => {
                            warn!("user check failed: internal error");
                            self.state.goodbye_message =
                                Some(CConst::SSH_INTERNAL_ERROR.to_string());
                            SshWizardStep::Terminate
                        }
                    };
                }
            }
            SshWizardStep::Login => {
                self.state.password = None;
                if let Some(response) = response {
                    if response.len() == 1 {
                        self.state.password = Some(response.get(0).unwrap().clone());
                    }
                }
                self.step = match login(&self.registry, &self.auth, &mut self.state).await {
                    LoginResult::LoginWithUser | LoginResult::LoginWithPublicKey => {
                        info!(
                            "login successful (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Sudo
                    }
                    LoginResult::Unverified => {
                        info!(
                            "login successful - must verify (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Verify
                    }
                    LoginResult::Unregistered => {
                        info!(
                            "login failed - must sign-up (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::SignUp
                    }
                    LoginResult::NoPasswordSupplied => {
                        warn!("login failed: no password supplied");
                        SshWizardStep::Login
                    }
                    LoginResult::IncorrectPassword => {
                        warn!("login failed: incorrect password");
                        self.state.goodbye_message = Some(CConst::SSH_WRONG_PASSWORD.to_string());
                        SshWizardStep::Terminate
                    }
                    LoginResult::InvalidEmail => {
                        warn!("login failed: invalid email");
                        self.state.goodbye_message = Some(CConst::SSH_INVALID_USERNAME.to_string());
                        SshWizardStep::Terminate
                    }
                    LoginResult::AccountLocked => {
                        warn!("login failed: account locked");
                        self.state.goodbye_message = Some(CConst::SSH_ACCOUNT_LOCKED.to_string());
                        SshWizardStep::Terminate
                    }
                    LoginResult::InternalError => {
                        warn!("login failed: internal error");
                        self.state.goodbye_message = Some(CConst::SSH_INTERNAL_ERROR.to_string());
                        SshWizardStep::Terminate
                    }
                };
            }
            SshWizardStep::Sudo => {
                self.state.sudo_code = None;
                if let Some(response) = response {
                    if response.len() == 1 {
                        self.state.sudo_code = Some(response.get(0).unwrap().clone());
                    }
                }
                self.step = match sudo(&self.registry, &self.auth, &mut self.state).await {
                    SudoResult::Success => {
                        info!(
                            "login sudo successful (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Shell
                    }
                    SudoResult::WrongCode => {
                        warn!("login sudo failed: wrong code suppleid");
                        self.state.goodbye_message = Some(CConst::SSH_WRONG_PASSWORD.to_string());
                        SshWizardStep::Terminate
                    }
                    SudoResult::NoAuthenticatorCodeSupplied => {
                        info!(
                            "login sudo skipped (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Shell
                    }
                    _ => {
                        warn!("login sudo failed: internal error");
                        self.state.goodbye_message = Some(CConst::SSH_INTERNAL_ERROR.to_string());
                        SshWizardStep::Terminate
                    }
                };
            }
            SshWizardStep::SignUp => {
                self.state.password = None;
                if let Some(response) = response {
                    if response.len() == 2 {
                        let password = response.get(0).unwrap().clone();
                        if password.len() < 4 {
                            self.state.goodbye_message =
                                Some(CConst::SSH_PASSWORD_WEAK.to_string());
                            self.step = SshWizardStep::Terminate;
                            return;
                        }
                        if response.get(1).unwrap().clone() != password {
                            self.state.goodbye_message =
                                Some(CConst::SSH_INVALID_INPUT.to_string());
                            self.step = SshWizardStep::Terminate;
                            return;
                        }
                        self.state.password = Some(password);
                    }
                }

                self.step = match user_create(&self.registry, &self.auth, &mut self.state).await {
                    UserCreateResult::Created => {
                        info!(
                            "sign up successful (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Verify
                    }
                    UserCreateResult::Terms => {
                        info!(
                            "sign up successful - new terms (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Terms
                    }
                    UserCreateResult::AlreadyExists => {
                        warn!(
                            "sign-up failed: user already exists (user={})",
                            self.state.email.clone().unwrap()
                        );
                        self.state.goodbye_message = Some(CConst::SSH_ACCOUNT_EXISTS.to_string());
                        SshWizardStep::Terminate
                    }
                    UserCreateResult::InvalidEmail => {
                        warn!(
                            "sign-up failed: username is invalid (user={})",
                            self.state.email.clone().unwrap()
                        );
                        self.state.goodbye_message = Some(CConst::SSH_INVALID_USERNAME.to_string());
                        SshWizardStep::Terminate
                    }
                    UserCreateResult::NoPasswordSupplied => {
                        warn!("sign-up failed: no password supplied");
                        self.state.goodbye_message = Some(CConst::SSH_INVALID_INPUT.to_string());
                        SshWizardStep::Terminate
                    }
                    UserCreateResult::NoUsernameSupplied => {
                        warn!("sign-up failed: no username supplied");
                        self.state.goodbye_message = Some(CConst::SSH_INVALID_INPUT.to_string());
                        SshWizardStep::Terminate
                    }
                    UserCreateResult::InternalError => {
                        warn!("sign-up failed: internal error");
                        self.state.goodbye_message = Some(CConst::SSH_INTERNAL_ERROR.to_string());
                        SshWizardStep::Terminate
                    }
                }
            }
            SshWizardStep::Terms => {
                self.state.accepted_terms = None;
                if let Some(response) = response {
                    if response.len() == 1 {
                        let answer = response.get(0).unwrap().trim().to_lowercase();
                        match answer.as_str() {
                            "agree" | "yes" | "ok" | "y" => {
                                self.state.accepted_terms = self.state.needed_terms.clone();
                            }
                            _ => {}
                        }
                    }
                }

                if self.state.accepted_terms.is_none() {
                    self.state.goodbye_message = Some(CConst::SSH_MUST_ACCEPT_TERMS.to_string());
                    self.step = SshWizardStep::Terminate;
                    return;
                }

                self.step = match user_create(&self.registry, &self.auth, &mut self.state).await {
                    UserCreateResult::Created => {
                        info!(
                            "terms accepted (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Verify
                    }
                    UserCreateResult::Terms => {
                        info!(
                            "new terms and conditions (user={})",
                            self.state.email.clone().unwrap()
                        );
                        SshWizardStep::Terms
                    }
                    _ => {
                        warn!("terms acceptance failed: internal error");
                        self.state.goodbye_message = Some(CConst::SSH_INTERNAL_ERROR.to_string());
                        SshWizardStep::Terminate
                    }
                }
            }
            SshWizardStep::Verify => {
                self.state.verify_code = None;
                if let Some(response) = response {
                    if response.len() == 1 {
                        self.state.verify_code = Some(response.get(0).unwrap().clone());
                    }
                }
                self.step = match login(&self.registry, &self.auth, &mut self.state).await {
                    LoginResult::LoginWithUser => {
                        info!("user verified (user={})", self.state.email.clone().unwrap());
                        SshWizardStep::Shell
                    }
                    LoginResult::IncorrectPassword => {
                        warn!("user verify failed: incorrect password");
                        self.state.goodbye_message =
                            Some(CConst::SSH_WRONG_VERIFY_CODE.to_string());
                        SshWizardStep::Terminate
                    }
                    LoginResult::AccountLocked => {
                        warn!("user verify failed: account locked");
                        self.state.goodbye_message = Some(CConst::SSH_ACCOUNT_LOCKED.to_string());
                        SshWizardStep::Terminate
                    }
                    _ => {
                        warn!("user verify failed: internal error");
                        self.state.goodbye_message = Some(CConst::SSH_INTERNAL_ERROR.to_string());
                        SshWizardStep::Terminate
                    }
                };
            }
            SshWizardStep::Shell => {}
            SshWizardStep::Terminate => {}
        };
    }

    pub fn fail(&mut self, msg: &str) {
        self.state.goodbye_message = Some(msg.to_string());
        self.step = SshWizardStep::Terminate;
    }

    fn get_welcome(&mut self) -> String {
        self.state.welcome.take().unwrap_or_else(|| "".to_string())
    }

    pub async fn next_auth(&mut self, response: Option<Vec<String>>) -> Auth {
        self.process_response(response).await;
        match self.step {
            SshWizardStep::Login => Auth::Partial {
                name: self.get_welcome().into(),
                instructions: Cow::Owned(
                    self.state.parse_message(CConst::SSH_INSTRUCTIONS_PASSWORD),
                ),
                prompts: Cow::Owned(vec![("Password: ".into(), false)]),
            },
            SshWizardStep::Sudo => Auth::Partial {
                name: self.get_welcome().into(),
                instructions: Cow::Owned(self.state.parse_message(CConst::SSH_INSTRUCTIONS_SUDO)),
                prompts: Cow::Owned(vec![("Authenticator Code: ".into(), true)]),
            },
            SshWizardStep::SignUp => Auth::Partial {
                name: self.get_welcome().into(),
                instructions: Cow::Owned(
                    self.state.parse_message(CConst::SSH_INSTRUCTIONS_SIGN_UP),
                ),
                prompts: Cow::Owned(vec![
                    ("Password: ".into(), false),
                    ("Password Again: ".into(), false),
                ]),
            },
            SshWizardStep::Terms => Auth::Partial {
                name: self.get_welcome().into(),
                instructions: Cow::Owned(self.state.parse_message(CConst::SSH_INSTRUCTIONS_TERMS)),
                prompts: Cow::Owned(vec![("(agree?): ".into(), true)]),
            },
            SshWizardStep::Verify => {
                let mut instructions = String::new();
                if self.state.qr_code.is_some() {
                    instructions.push_str(
                        self.state
                            .parse_message(CConst::SSH_INSTRUCTIONS_QR)
                            .as_str(),
                    );
                }
                instructions.push_str(
                    self.state
                        .parse_message(CConst::SSH_INSTRUCTIONS_VERIFY)
                        .as_str(),
                );
                Auth::Partial {
                    name: self.get_welcome().into(),
                    instructions: Cow::Owned(instructions),
                    prompts: Cow::Owned(vec![("Verification Code: ".into(), true)]),
                }
            }
            _ => Auth::Accept,
        }
    }

    pub fn next_shell(&self) -> bool {
        self.step == SshWizardStep::Shell
    }

    pub fn goodbye_message(&self) -> Option<String> {
        self.state.goodbye_message.clone()
    }
}

#[async_trait::async_trait]
impl term_api::WizardAbi for SshWizard {
    async fn process(&mut self, responses: Vec<String>) -> term_api::WizardAction {
        let responses = if responses.len() > 0 {
            Some(responses)
        } else {
            None
        };
        match self.next_auth(responses).await {
            Auth::Accept => {
                if self.next_shell() {
                    term_api::WizardAction::Shell
                } else {
                    term_api::WizardAction::Terminate {
                        with_message: self.state.goodbye_message.clone(),
                    }
                }
            }
            Auth::Partial {
                name,
                instructions,
                prompts,
            } => term_api::WizardAction::Challenge {
                name: name.to_string(),
                instructions: instructions.to_string(),
                prompts: prompts
                    .iter()
                    .map(|(prompt, echo)| term_api::WizardPrompt {
                        prompt: prompt.to_string(),
                        echo: *echo,
                    })
                    .collect(),
            },
            Auth::Reject | Auth::UnsupportedMethod => term_api::WizardAction::Terminate {
                with_message: self.state.goodbye_message.clone(),
            },
        }
    }

    fn token(&self) -> Option<String> {
        self.state
            .session
            .clone()
            .map(|a| wasmer_auth::helper::session_to_b64(a).unwrap())
    }
}

async fn login(
    registry: &Arc<Registry>,
    auth: &url::Url,
    state: &mut SshWizardState,
) -> LoginResult {
    // Open a command chain
    let chain = match registry.open_cmd(&auth).await {
        Ok(a) => a,
        Err(err) => {
            debug!("{}", err);
            return LoginResult::InternalError;
        }
    };

    // Get the username and password
    let username = match state.email.clone() {
        Some(a) => a,
        None => {
            return LoginResult::InvalidEmail;
        }
    };
    let password = match state.password.clone() {
        Some(a) => a,
        None => {
            return LoginResult::NoPasswordSupplied;
        }
    };

    // Generate a read-key using the password and some seed data
    // (this read-key will be mixed with entropy on the server side to decrypt the row
    //  which means that neither the client nor the server can get at the data alone)
    let prefix = format!("remote-login:{}:", username);
    let read_key = password_to_read_key(&prefix, &password, 15, KeySize::Bit192);

    // Create the login command
    let login = LoginRequest {
        email: username.clone(),
        secret: read_key,
        verification_code: state.verify_code.clone(),
    };

    // Attempt the login request with a 10 second timeout
    trace!("invoking login (email={})", login.email);
    let response: Result<LoginResponse, LoginFailed> = match chain.invoke(login).await {
        Ok(a) => a,
        Err(err) => {
            debug!("{}", err);
            return LoginResult::InternalError;
        }
    };
    let result = match response {
        Ok(a) => a,
        Err(LoginFailed::AccountLocked(_)) => {
            return LoginResult::AccountLocked;
        }
        Err(LoginFailed::Unverified(_)) => {
            return LoginResult::Unverified;
        }
        Err(LoginFailed::UserNotFound(_)) => {
            return LoginResult::Unregistered;
        }
        Err(LoginFailed::WrongPassword) => {
            return LoginResult::IncorrectPassword;
        }
        _ => {
            return LoginResult::InternalError;
        }
    };
    state.session = Some(AteSessionType::User(result.authority));

    // Display the message of the day
    if let Some(message_of_the_day) = result.message_of_the_day {
        state.message_of_the_day = Some(message_of_the_day);
    }
    LoginResult::LoginWithUser
}

async fn user_exists(
    registry: &Arc<Registry>,
    auth: &url::Url,
    state: &mut SshWizardState,
) -> UserExistsResult {
    // Open a command chain
    let chain = match registry.open_cmd(&auth).await {
        Ok(a) => a,
        Err(err) => {
            debug!("{}", err);
            return UserExistsResult::InternalError;
        }
    };

    // Get the username and password
    let username = match state.email.clone() {
        Some(a) => a,
        None => {
            return UserExistsResult::InvalidEmail;
        }
    };

    // Create the login command
    let query = QueryRequest {
        identity: username.clone(),
    };

    // Attempt the login request with a 10 second timeout
    trace!("invoking query (identity={})", query.identity);
    let response: Result<QueryResponse, QueryFailed> = match chain.invoke(query).await {
        Ok(a) => a,
        Err(err) => {
            debug!("{}", err);
            return UserExistsResult::InternalError;
        }
    };
    let result = match response {
        Ok(a) => a,
        Err(QueryFailed::NotFound) => {
            return UserExistsResult::NotFound;
        }
        Err(QueryFailed::Banned) => {
            return UserExistsResult::Banned;
        }
        Err(QueryFailed::Suspended) => {
            return UserExistsResult::Suspended;
        }
        Err(QueryFailed::InternalError(_)) => {
            return UserExistsResult::InternalError;
        }
    };
    let _id = result.advert.id;

    UserExistsResult::Found
}

async fn sudo(registry: &Arc<Registry>, auth: &url::Url, state: &mut SshWizardState) -> SudoResult {
    let authenticator_code = match state.sudo_code.clone() {
        Some(a) if a.len() > 0 => a,
        _ => {
            return SudoResult::NoAuthenticatorCodeSupplied;
        }
    };

    // Open a command chain
    let chain = match registry.open_cmd(&auth).await {
        Ok(a) => a,
        Err(err) => {
            warn!("{}", err);
            return SudoResult::InternalError;
        }
    };

    let session = match state.session.clone() {
        Some(AteSessionType::User(a)) => a,
        Some(_) => {
            warn!("internal error: wrong type of session");
            return SudoResult::InternalError;
        }
        None => {
            warn!("internal error: no session");
            return SudoResult::InternalError;
        }
    };

    // Create the sudo command
    let login = SudoRequest {
        session,
        authenticator_code,
    };

    // Attempt the sudo request with a 10 second timeout
    let response: Result<SudoResponse, SudoFailed> = match chain.invoke(login).await {
        Ok(a) => a,
        Err(err) => {
            warn!("{}", err);
            return SudoResult::InternalError;
        }
    };
    let result = match response {
        Ok(a) => a,
        Err(SudoFailed::WrongCode) => {
            return SudoResult::WrongCode;
        }
        Err(SudoFailed::MissingToken) => {
            warn!("invoke sudo failed: missing token");
            return SudoResult::InternalError;
        }
        Err(SudoFailed::UserNotFound(msg)) => {
            warn!("invoke sudo failed: user not found - {}", msg);
            return SudoResult::InternalError;
        }
        Err(SudoFailed::AccountLocked(_)) => {
            warn!("invoke sudo failed: account locked");
            return SudoResult::InternalError;
        }
        Err(SudoFailed::Unverified(msg)) => {
            warn!("invoke sudo failed: unverified ({})", msg);
            return SudoResult::InternalError;
        }
        Err(SudoFailed::NoMasterKey) => {
            warn!("invoke sudo failed: no master key");
            return SudoResult::InternalError;
        }
        Err(SudoFailed::InternalError(code)) => {
            warn!("invoke sudo failed: internal error({})", code);
            return SudoResult::InternalError;
        }
    };
    state.session = Some(AteSessionType::Sudo(result.authority));
    SudoResult::Success
}

async fn user_create(
    registry: &Arc<Registry>,
    auth: &url::Url,
    state: &mut SshWizardState,
) -> UserCreateResult {
    // Grab the two main parts
    let username = match state.email.clone() {
        Some(a) => a,
        None => {
            return UserCreateResult::NoUsernameSupplied;
        }
    };
    let password = match state.password.clone() {
        Some(a) => a,
        None => {
            return UserCreateResult::NoPasswordSupplied;
        }
    };

    // Create a user using the authentication server which will give us a session with all the tokens
    let result = match wasmer_auth::cmd::create_user_command(
        registry,
        username,
        password,
        auth.clone(),
        state.accepted_terms.clone(),
    )
    .await
    {
        Ok(a) => a,
        Err(CreateError(CreateErrorKind::AlreadyExists(_), _)) => {
            return UserCreateResult::AlreadyExists;
        }
        Err(CreateError(CreateErrorKind::TermsAndConditions(terms), _)) => {
            state.needed_terms = Some(terms);
            return UserCreateResult::Terms;
        }
        Err(CreateError(CreateErrorKind::InvalidEmail, _)) => {
            return UserCreateResult::InvalidEmail;
        }
        Err(err) => {
            warn!("{}", err);
            return UserCreateResult::InternalError;
        }
    };

    if let Some(message_of_the_day) = &result.message_of_the_day {
        state.message_of_the_day = Some(message_of_the_day.clone());
    }
    state.qr_code = Some(result.qr_code);
    return UserCreateResult::Created;
}
