#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};

use ::ate::prelude::*;

pub fn conf_auth() -> ConfAte
{
    let mut cfg_ate = ConfAte::default();
    cfg_ate.configured_for(ConfiguredFor::BestSecurity);
    cfg_ate.log_format.meta = SerializationFormat::Json;
    cfg_ate.log_format.data = SerializationFormat::Json;
    cfg_ate.record_type_name = true;
    cfg_ate
}

pub fn conf_cmd() -> ConfAte
{
    let cfg_cmd = conf_auth();
    cfg_cmd
}