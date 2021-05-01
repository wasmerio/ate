#[allow(unused_imports)]
use log::{info, error, warn, debug};
use std::sync::Arc;

use crate::chain::*;
use crate::session::*;
use crate::repository::*;

pub struct InvocationContext<'a>
{
    pub session: &'a AteSession,
    pub chain: Arc<Chain>,
    pub repository: Arc<dyn ChainRepository>
}