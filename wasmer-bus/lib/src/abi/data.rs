use super::BusError;

#[repr(C)]
#[derive(Debug)]
#[must_use = "this `Data` may be an `Error` variant, which should be handled"]
pub enum Data {
    Prepared(Vec<u8>),
    Error(BusError),
}
