pub struct CConst {}

impl CConst {
    pub const SSH_WELCOME: &'static str = r#"
      ██╗    ██╗ █████╗ ███████╗███╗   ███╗███████╗██████╗ 
      ██║    ██║██╔══██╗██╔════╝████╗ ████║██╔════╝██╔══██╗
      ██║ █╗ ██║███████║███████╗██╔████╔██║█████╗  ██████╔╝
      ██║███╗██║██╔══██║╚════██║██║╚██╔╝██║██╔══╝  ██╔══██╗
      ╚███╔███╔╝██║  ██║███████║██║ ╚═╝ ██║███████╗██║  ██║
       ╚══╝╚══╝ ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝"#;

    pub const SSH_INSTRUCTIONS_PASSWORD: &'static str = r#"
Welcome back ({email}),"#;

    pub const SSH_INSTRUCTIONS_SUDO: &'static str = r#"
Enter your authenticator code (or press enter to skip),"#;

    pub const SSH_ASSOCIATE: &'static str = r#"
Would you like to associate your SSH key with account?"#;

    pub const SSH_INSTRUCTIONS_TERMS: &'static str = r#"
{terms}

If you agree to the above terms and conditions then type the word 'agree' below"#;

    pub const SSH_INSTRUCTIONS_SIGN_UP: &'static str = r#"
The login user is not known to us...
...but do not fear!...as the sign-up wizard is here...

Username: {email}"#;

    pub const SSH_INSTRUCTIONS_FAILED: &'static str = r#"
Unfortunately the login has failed:
'{message}'"#;

    pub const SSH_INSTRUCTIONS_QR: &'static str = r#"
Below is your Google Authenticator QR code - scan it on your phone and
save it as this code is the only way you can recover the account.

{qr_code}
"#;

    pub const SSH_INSTRUCTIONS_VERIFY: &'static str = r#"
Check your email for a verification code and enter it below"#;

    pub const SSH_WRONG_PASSWORD: &'static str = r#"
The password was incorrect
(Warning! Repeated failed attempts will trigger a short ban)"#;

    pub const SSH_MUST_ACCEPT_TERMS: &'static str = r#"
You may only create an account by specifically agreeing to the terms
and conditions laid out above - this can only be confirmed if you
specifically type the word 'agree' which you did not enter hence
an account can not be created. If this is a mistake then please
try again."#;

    pub const SSH_WRONG_VERIFY_CODE: &'static str = r#"
The verification code was incorrect"#;

    pub const SSH_ACCOUNT_LOCKED: &'static str = r#"
Your account has been locked, please try again later"#;

    pub const SSH_ACCOUNT_EXISTS: &'static str = r#"
The account you specified already exists"#;

    pub const SSH_INTERNAL_ERROR: &'static str = r#"
An internal error has occured"#;

    pub const SSH_INVALID_INPUT: &'static str = r#"
The input you supplied was invalid"#;

    pub const SSH_INVALID_USERNAME: &'static str = r#"
The username must be a valid email address
(e.g. ssh joe@blogs.com@wasmer.sh)"#;

    pub const SSH_LOGIN_DISABLE: &'static str = r#"
wasmer.sh does not yet support authentication methods"#;

    pub const SSH_PASSWORD_MISMATCH: &'static str = r#"
The two passwords you supplied did not match"#;

    pub const SSH_PASSWORD_WEAK: &'static str = r#"
The password was not of sufficient complexity"#;
}
