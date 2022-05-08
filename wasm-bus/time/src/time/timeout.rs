use std::fmt::Display;
use std::future::Future;
use std::time::Duration;
use tokio::select;

use super::*;

#[derive(Debug)]
pub struct Elapsed {
    duration: Duration,
}

impl Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "timeout elasped (limit={}ms)", self.duration.as_millis())
    }
}

pub async fn timeout<T>(duration: Duration, future: T) -> Result<T::Output, Elapsed>
where
    T: Future,
{
    select! {
        _ = sleep(duration) => {
            return Err(Elapsed { duration })
        },
        a = future => {
            return Ok(a)
        }
    }
}
