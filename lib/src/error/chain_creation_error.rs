use error_chain::error_chain;

error_chain! {
    types {
        ChainCreationError, ChainCreationErrorKind, ResultExt, Result;
    }
    links {
        CompactError(super::CompactError, super::CompactErrorKind);
        TimeError(super::TimeError, super::TimeErrorKind);
        CommsError(super::CommsError, super::CommsErrorKind);
        SerializationError(super::SerializationError, super::SerializationErrorKind);
    }
    foreign_links {
        IO(::tokio::io::Error);
        UrlInvalid(::url::ParseError);
        ProcessError(super::ProcessError);
    }
    errors {
        NoRootFoundInConfig {
            display("failed to create chain-of-trust as the root node is not found in the configuration settings"),
        }
        RootRedirect(expected: u32, actual: u32) {
            description("failed to create chain-of-trust as the server you connected is not hosting these chains"),
            display("failed to create chain-of-trust as the server you connected (node_id={}) is not hosting these chains - instead you must connect to another node (node_id={})", actual, expected),
        }
        NoRootFoundForDomain(domain: String) {
            description("failed to create chain-of-trust as the root node is not found in the domain"),
            display("failed to create chain-of-trust as the root node is not found in the domain [{}]", domain),
        }
        UnsupportedProtocol(proto: String) {
            description("failed to create chain-of-trust as the protocol is not supported"),
            display("failed to create chain-of-trust as the protocol is not supported ({})", proto),
        }
        NotSupported {
            display("failed to create chain-of-trust as the operation is not supported. possible causes are calling 'open_by_key' on a Registry which only supports the 'open_by_url'."),
        }
        NotThisRoot {
            display("failed to create chain-of-trust as this is the wrong root node"),
        }
        NotImplemented {
            display("failed to create chain-of-trust as the method is not implemented"),
        }
        NoValidDomain(domain: String) {
            description("failed to create chain-of-trust as the address does not have a valid domain name"),
            display("failed to create chain-of-trust as the address does not have a valid domain name [{}]", domain),
        }
        InvalidRoute(route: String) {
            description("failed to create chain-of-trust as the chain path is not hosted as a route"),
            display("failed to create chain-of-trust as the chain path ({}) is not hosted as a route", route),
        }
        ServerRejected(reason: crate::mesh::FatalTerminate) {
            description("failed to create chain-of-trust as the server refused to create the chain"),
            display("failed to create chain-of-trust as the server refused to create the chain ({})", reason),
        }
        #[cfg(feature="enable_dns")]
        DnsProtoError(err: ::trust_dns_proto::error::ProtoError) {
            description("failed to create chain-of-trust due to a DNS error"),
            display("failed to create chain-of-trust due to a DNS error - {}", err),
        }
        #[cfg(feature="enable_dns")]
        DnsClientError(err: ::trust_dns_client::error::ClientError) {
            description("failed to create chain-of-trust due to a DNS error"),
            display("failed to create chain-of-trust due to a DNS error - {}", err),
        }
        InternalError(err: String) {
            description("internal error"),
            display("{}", err),
        }
    }
}

#[cfg(feature="enable_dns")]
impl From<::trust_dns_proto::error::ProtoError>
for ChainCreationError
{
    fn from(err: ::trust_dns_proto::error::ProtoError) -> ChainCreationError {
        ChainCreationErrorKind::DnsProtoError(err).into()
    }   
}

#[cfg(feature="enable_dns")]
impl From<::trust_dns_client::error::ClientError>
for ChainCreationError
{
    fn from(err: ::trust_dns_client::error::ClientError) -> ChainCreationError {
        ChainCreationErrorKind::DnsClientError(err).into()
    }   
}