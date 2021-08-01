#[allow(unused_imports)]
use tracing::{info, debug, warn, error, trace};
use std::error::Error;

#[cfg(feature="enable_dns")]
use trust_dns_proto::error::ProtoError as DnsProtoError;
#[cfg(feature="enable_dns")]
use trust_dns_client::error::ClientError as DnsClientError;

use super::*;
use crate::mesh::FatalTerminate;

#[derive(Debug)]
pub enum ChainCreationError {
    ProcessError(ProcessError),
    IO(tokio::io::Error),
    SerializationError(SerializationError),
    CompactError(CompactError),
    NoRootFoundInConfig,
    RootRedirect(u32, u32),
    NoRootFoundForDomain(String),
    UnsupportedProtocol(String),
    UrlInvalid(url::ParseError),
    NotSupported,
    #[allow(dead_code)]
    NotThisRoot,
    #[allow(dead_code)]
    NotImplemented,
    TimeError(TimeError),
    NoValidDomain(String),
    InvalidRoute(String),
    CommsError(CommsError),
    #[cfg(feature="enable_dns")]
    DnsProtoError(DnsProtoError),
    #[cfg(feature="enable_dns")]
    DnsClientError(DnsClientError),
    ServerRejected(FatalTerminate),
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

#[cfg(feature="enable_dns")]
impl From<DnsProtoError>
for ChainCreationError
{
    fn from(err: DnsProtoError) -> ChainCreationError {
        ChainCreationError::DnsProtoError(err)
    }
}

#[cfg(feature="enable_dns")]
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
            ChainCreationError::NoRootFoundForDomain(url) => {
                write!(f, "Failed to create chain-of-trust as the root node is not found in the domain [{}]", url)
            },
            ChainCreationError::RootRedirect(expected, actual) => {
                write!(f, "Failed to create chain-of-trust as the server you connected (node_id={}) is not hosting these chains - instead you must connect to another node (node_id={})", actual, expected)
            }
            ChainCreationError::UnsupportedProtocol(proto) => {
                write!(f, "Failed to create chain-of-trust as the protocol is not supported ({})", proto)
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
            ChainCreationError::InvalidRoute(chain) => {
                write!(f, "Failed to create chain-of-trust as the chain path ({}) is not hosted as a route", chain)
            }
            ChainCreationError::NoValidDomain(err) => {
                write!(f, "Failed to create chain-of-trust as the address does not have a valid domain name [{}]", err)
            },
            #[cfg(feature="enable_dns")]
            ChainCreationError::DnsProtoError(err) => {
                write!(f, "Failed to create chain-of-trust due to a DNS error - {}", err)
            },
            #[cfg(feature="enable_dns")]
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