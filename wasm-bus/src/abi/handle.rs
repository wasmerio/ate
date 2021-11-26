#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
