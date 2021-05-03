#[allow(unused_imports)]
use log::{info, error, debug};
use std::{error::Error, time::SystemTime};
use chrono::Utc;
use chrono::DateTime;

extern crate rmp_serde as rmps;

use std::time::SystemTimeError;

#[derive(Debug)]
pub enum TimeError
{
    IO(std::io::Error),
    SystemTimeError(SystemTimeError),
    BeyondTolerance(u32),
    NoTimestamp,
    OutOfBounds{ cursor: SystemTime, timestamp: SystemTime},
}

impl From<std::io::Error>
for TimeError
{
    fn from(err: std::io::Error) -> TimeError {
        TimeError::IO(err)
    }   
}

impl From<SystemTimeError>
for TimeError
{
    fn from(err: SystemTimeError) -> TimeError {
        TimeError::SystemTimeError(err)
    }   
}

impl std::fmt::Display
for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeError::IO(err) => {
                write!(f, "IO error while computing the current time - {}", err.to_string())
            },
            TimeError::SystemTimeError(err) => {
                write!(f, "System clock error while computing the current time - {}", err.to_string())
            },
            TimeError::BeyondTolerance(err) => {
                write!(f, "The network latency is beyond tolerance to synchronize the clocks - {}", err.to_string())
            },
            TimeError::NoTimestamp => {
                write!(f, "The data object has no timestamp metadata attached to it")
            },
            TimeError::OutOfBounds{ cursor, timestamp} => {
                let cursor = DateTime::<Utc>::from(*cursor).format("%Y-%m-%d %H:%M:%S.%f").to_string();
                let timestamp = DateTime::<Utc>::from(*timestamp).format("%Y-%m-%d %H:%M:%S.%f").to_string();
                write!(f, "The network latency is out of bounds - cursor:{}, timestamp:{}", cursor, timestamp)
            },
        }
    }
}

impl std::error::Error
for TimeError
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}