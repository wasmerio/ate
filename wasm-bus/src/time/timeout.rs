use std::time::Duration;
use std::future::Future;
use tokio::select;

use super::*;

#[derive(Debug)]
pub struct Elapsed { }

pub async fn timeout<T>(duration: Duration, future: T) -> Result<T::Output, Elapsed>
where T: Future,
{
    select! {
        _ = sleep(duration) => {
            return Err(Elapsed {})
        },
        a = future => {
            return Ok(a)
        }
    }
}