use super::CallError;

#[repr(C)]
#[derive(Debug)]
#[must_use = "this `Data` may be an `Error` variant, which should be handled"]
pub enum Data {
    Success(Vec<u8>),
    Error(CallError),
}
