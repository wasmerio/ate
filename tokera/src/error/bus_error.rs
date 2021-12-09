use error_chain::error_chain;

error_chain! {
    types {
        BusError, BusErrorKind, ResultExt, Result;
    }
    foreign_links {
        IO(tokio::io::Error);
    }
}
