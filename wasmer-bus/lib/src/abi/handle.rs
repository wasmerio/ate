use serde::*;
use std::fmt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallHandle {
    pub id: u64,
}

impl From<u64> for CallHandle {
    fn from(val: u64) -> CallHandle {
        CallHandle { id: val }
    }
}

impl Into<u64> for CallHandle {
    fn into(self) -> u64 {
        self.id
    }
}

impl fmt::Display for CallHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "call_handle={}", self.id)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusHandle {
    pub id: u32,
}

impl From<u32> for BusHandle {
    fn from(val: u32) -> BusHandle {
        BusHandle { id: val }
    }
}

impl Into<u32> for BusHandle {
    fn into(self) -> u32 {
        self.id
    }
}

impl fmt::Display for BusHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bus_handle={}", self.id)
    }
}
