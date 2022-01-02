use error_chain::error_chain;

error_chain! {
    types {
        SshServerError, SshServerErrorKind, SshServerResultExt, SshServerResult;
    }
    foreign_links {
        Thrussh(thrussh::Error);
    }
    errors {
        BadData {
            description("received bad data"),
            display("received bad data"),
        }
    }
}
