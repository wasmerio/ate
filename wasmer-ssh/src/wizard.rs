use super::cconst::CConst;
use std::borrow::Cow;
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
    pub public_key: Option<PublicKey>,
    pub password: Option<String>,
    pub verify_code: Option<String>,
    pub sudo_code: Option<String>,
    pub shell_message: Option<String>,
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
}

impl SshWizard {
    #[allow(unused_variables)]
    async fn process_response<'a>(&mut self, response: Option<Vec<String>>) {
        match self.step.clone() {
            SshWizardStep::Init => {
                let mut username = match self.state.email.clone() {
                    Some(a) => a,
                    None => {
                        trace!("no username supplied: defaulting to guest");
                        "guest".to_string()
                    }
                };

                if username.contains("@") == false {
                    username += "@wasmer.io";
                }

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

                info!(
                    "login sudo successful (user={})",
                    self.state.email.clone().unwrap()
                );
                self.state.shell_message = Some(self.get_welcome());
                self.step = SshWizardStep::Shell;
            }
            SshWizardStep::Login | SshWizardStep::Sudo | SshWizardStep::SignUp | SshWizardStep::Terms | SshWizardStep::Verify => {
                self.state.goodbye_message = Some(CConst::SSH_LOGIN_DISABLE.to_string());
                self.step = SshWizardStep::Terminate;
                return;
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
                    term_api::WizardAction::Shell {
                        with_message: None,
                    }
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
        None
    }
}
