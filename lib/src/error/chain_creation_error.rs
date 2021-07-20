#[allow(unused_imports)]
use log::{info, error, debug};
use std::error::Error;

use trust_dns_proto::error::ProtoError as DnsProtoError;
use trust_dns_client::error::ClientError as DnsClientError;

use super::*;

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
    CompactError(CompactError),
    NoRootFoundInConfig,
    NoRootFoundForUrl(String),
    UnsupportedProtocol,
    UrlInvalid(url::ParseError),
    NotSupported,
    #[allow(dead_code)]
    NotThisRoot,
    #[allow(dead_code)]
    NotImplemented,
    TimeError(TimeError),
    NoValidDomain(String),
    CommsError(CommsError),
    DnsProtoError(DnsProtoError),
    DnsClientError(DnsClientError),
    ServerRejected(String),
    InternalError(String),
}

impl From<url::ParseError>
for ChainCreationError
{
    fn from(err: url::ParseError) -> ChainCreationError {
        ChainCreationError::UrlInvalid(err)
    }   
}

impl From<ProcessError>
for ChainCreationError
{
    fn from(err: ProcessError) -> ChainCreationError {
        ChainCreationError::ProcessError(err)
    }   
}

impl From<SerializationError>
for ChainCreationError
{
    fn from(err: SerializationError) -> ChainCreationError {
        ChainCreationError::SerializationError(err)
    }   
}

impl From<tokio::io::Error>
for ChainCreationError
{
    fn from(err: tokio::io::Error) -> ChainCreationError {
        ChainCreationError::IO(err)
    }   
}

impl From<CommsError>
for ChainCreationError
{
    fn from(err: CommsError) -> ChainCreationError {
        ChainCreationError::CommsError(err)
    }   
}

impl From<CompactError>
for ChainCreationError
{
    fn from(err: CompactError) -> ChainCreationError {
        ChainCreationError::CompactError(err)
    }   
}

impl From<DnsProtoError>
for ChainCreationError
{
    fn from(err: DnsProtoError) -> ChainCreationError {
        ChainCreationError::DnsProtoError(err)
    }
}

impl From<DnsClientError>
for ChainCreationError
{
    fn from(err: DnsClientError) -> ChainCreationError {
        ChainCreationError::DnsClientError(err)
    }
}

impl From<TimeError>
for ChainCreationError
{
    fn from(err: TimeError) -> ChainCreationError {
        ChainCreationError::TimeError(err)
    }
}

impl std::fmt::Display
for ChainCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ChainCreationError::ProcessError(err) => {
                write!(f, "Failed to create chain-of-trust due to a processingerror - {}", err)
            },
            ChainCreationError::SerializationError(err) => {
                write!(f, "Failed to create chain-of-trust due to a serialization error - {}", err)
            },
            ChainCreationError::UrlInvalid(err) => {
                write!(f, "Failed to create chain-of-trust due to a parsing the chain URL - {}", err)
            },
            ChainCreationError::IO(err) => {
                write!(f, "Failed to create chain-of-trust due to an IO error - {}", err)
            },
            ChainCreationError::NotImplemented => {
                write!(f, "Failed to create chain-of-trust as the method is not implemented")
            },
            ChainCreationError::NotSupported => {
                write!(f, "Failed to create chain-of-trust as the operation is not supported. Possible causes are calling 'open_by_key' on a Registry which only supports the 'open_by_url'.")
            },
            ChainCreationError::NoRootFoundInConfig => {
                write!(f, "Failed to create chain-of-trust as the root node is not found in the configuration settings")
            },
            ChainCreationError::NoRootFoundForUrl(url) => {
                write!(f, "Failed to create chain-of-trust as the root node is not found in the URL [{}]", url)
            },
            ChainCreationError::UnsupportedProtocol => {
                write!(f, "Failed to create chain-of-trust as the protocol is not supported (only TCP is supported)")
            },
            ChainCreationError::NotThisRoot => {
                write!(f, "Failed to create chain-of-trust as this is the wrong root node")
            },
            ChainCreationError::CommsError(err) => {
                write!(f, "Failed to create chain-of-trust due to a communication error - {}", err)
            },
            ChainCreationError::CompactError(err) => {
                write!(f, "Failed to create chain-of-trust due issue compacting the redo log - {}", err)
            },
            ChainCreationError::NoValidDomain(err) => {
                write!(f, "Failed to create chain-of-trust as the address does not have a valid domain name [{}]", err)
            },
            ChainCreationError::DnsProtoError(err) => {
                write!(f, "Failed to create chain-of-trust due to a DNS error - {}", err)
            },
            ChainCreationError::DnsClientError(err) => {
                write!(f, "Failed to create chain-of-trust due to a DNS error - {}", err)
            },
            ChainCreationError::ServerRejected(reason) => {
                write!(f, "Failed to create chain-of-trust as the server refused to create the chain ({})", reason)
            },
            ChainCreationError::TimeError(err) => {
                write!(f, "Failed to create chain-of-trust due error with time keeping - {}", err)
            },
            ChainCreationError::InternalError(err) => {
                write!(f, "{}", err)
            },
        }
    }
}

impl std::error::Error
for ChainCreationError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}