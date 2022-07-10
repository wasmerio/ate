use error_chain::error_chain;
use ate_files::error::FileSystemError;
use ate_files::error::FileSystemErrorKind;

error_chain! {
    types {
        SshServerError, SshServerErrorKind, SshServerResultExt, SshServerResult;
    }
    links {
        FileSystemError(FileSystemError, FileSystemErrorKind);
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
