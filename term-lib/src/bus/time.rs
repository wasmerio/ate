#![allow(dead_code)]
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use wasmer_vbus::{InstantInvocation, VirtualBusInvoked};

use crate::api::*;

pub fn sleep(system: System, duration_ms: u128) -> Box<dyn VirtualBusInvoked> {
    let result = system.sleep(duration_ms);
    Box::new(InstantInvocation::call(Box::new(result)))
}
