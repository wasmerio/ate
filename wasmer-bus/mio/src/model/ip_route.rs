use std::fmt;
use std::net::IpAddr;
use serde::*;
use chrono::DateTime;
use chrono::Utc;

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpRoute
{
    pub cidr: IpCidr,
    pub via_router: IpAddr,
    pub preferred_until: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>
}

impl fmt::Display
for IpRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "route(cidr={},via={}", self.cidr, self.via_router)?;
        if let Some(a) = self.preferred_until {
            write!(f, ",preferred_until={}", a)?;
        }
        if let Some(a) = self.expires_at {
            write!(f, ",expires_at={}", a)?;
        }
        write!(f, ")")
    }
}
