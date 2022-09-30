mod caller_context;
mod factory;
mod feeder;
mod invokable;
mod process;
mod reqwest;
mod standard;
mod sub_process;
mod time;
mod util;
mod ws;
mod tty;
//mod webgl;

use std::convert::TryInto;

pub use caller_context::*;
pub(crate) use invokable::*;
pub(crate) use process::*;
pub(crate) use sub_process::*;
use util::*;

pub use factory::BusFactory;
pub use process::ProcessExecFactory;
pub use process::LaunchEnvironment;
pub use feeder::BusStatefulFeeder;
pub use feeder::BusStatelessFeeder;
pub use feeder::BusFeederUtils;
pub use feeder::CallHandle;
pub use feeder::BusError;
pub use sub_process::SubProcessMultiplexer;
pub use invokable::Processable;
pub use invokable::InvokeResult;
pub use invokable::Session;
pub use util::*;
pub use standard::StandardBus;

pub fn hash_topic(topic: &str) -> u128 {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&topic.bytes().collect::<Vec<_>>());
    let hash: [u8; 16] = hasher.finalize()[..16].try_into().unwrap();
    u128::from_le_bytes(hash)
}

pub fn type_name_hash<T: ?Sized>() -> u128 {
    hash_topic(std::any::type_name::<T>())
}
