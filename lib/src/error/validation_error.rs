use error_chain::error_chain;

error_chain! {
    types {
        ValidationError, ValidationErrorKind, ResultExt, Result;
    }
    links {
        TrustError(super::TrustError, super::TrustErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
    }
    errors {
        Denied(reason: String) {
            description("the data was rejected"),
            display("the data was rejected - {}", reason),
        }
        Many(errors: Vec<ValidationError>) {
            description("the data was rejected by one (or more) of the validators"),
            display("the data was rejected by {} of the validators", errors.len()),
        }
        AllAbstained {
            display("none of the validators approved this data object event")
        }
        Detached {
            display("the data object event is detached from the chain of trust")
        }
        NoSignatures {
            display("the data object event has no signatures and one is required to store it at this specific location within the chain of trust")
        }
    }
}