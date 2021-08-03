#[allow(unused_imports)]
use tracing::{info, error, warn, debug};

use crate::session::*;

pub struct InvocationContext<'a>
{
    pub session: &'a AteSession,
}