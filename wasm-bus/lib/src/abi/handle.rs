use serde::*;
use std::fmt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallHandle {
    pub id: u32,
}

impl From<u32> for CallHandle {
    fn from(val: u32) -> CallHandle {
        CallHandle { id: val }
    }
}

impl Into<u32> for CallHandle {
    fn into(self) -> u32 {
        self.id
    }
}

impl fmt::Display for CallHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "handle_id={}", self.id)
    }
}
