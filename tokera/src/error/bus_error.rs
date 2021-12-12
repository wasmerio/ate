use error_chain::error_chain;

error_chain! {
    types {
        BusError, BusErrorKind, ResultExt, Result;
    }
    foreign_links {
        IO(tokio::io::Error);
    }
    errors {
        LoginFailed {
            description("failed to login with the supplied token"),
            display("failed to login with the supplied token"),
        }
    }
}
