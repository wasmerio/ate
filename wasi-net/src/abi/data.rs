use super::{Buffer, CallError};

#[repr(C)]
#[derive(Debug)]
#[must_use = "this `Data` may be an `Error` variant, which should be handled"]
pub enum Data {
    Success(Buffer),
    Error(CallError),
}