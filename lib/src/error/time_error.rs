use error_chain::error_chain;
use std::{time::SystemTime};
use chrono::Utc;
use chrono::DateTime;

use std::time::SystemTimeError;

error_chain! {
    types {
        TimeError, TimeErrorKind, ResultExt, Result;
    }
    foreign_links {
        IO(std::io::Error);
        SystemTimeError(SystemTimeError);
    }
    errors {
        BeyondTolerance(tolerance: u32) {
            description("the network latency is beyond tolerance to synchronize the clocks"),
            display("the network latency is beyond tolerance ({}) to synchronize the clocks", tolerance.to_string()),
        }
        NoTimestamp {
            description("the data object has no timestamp metadata attached to it")
            display("the data object has no timestamp metadata attached to it")
        }
        OutOfBounds(cursor: SystemTime, timestamp: SystemTime) {
            description("the network latency is out of bound"),
            display("the network latency is out of bounds - cursor:{}, timestamp:{}",
                    DateTime::<Utc>::from(*cursor).format("%Y-%m-%d %H:%M:%S.%f").to_string(),
                    DateTime::<Utc>::from(*timestamp).format("%Y-%m-%d %H:%M:%S.%f").to_string())
        }
    }
}