#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallHandle
{
    pub id: u32,
}

impl From<u32>
for CallHandle
{
    fn from(val: u32) -> CallHandle {
        CallHandle {
            id: val
        }
    }
}