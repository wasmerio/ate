pub mod cmd;
pub mod error;
#[cfg(all(feature = "server"))]
pub mod flow;
pub mod helper;
pub mod model;
pub mod opt;
pub mod prelude;
pub mod request;
pub mod service;
mod test;
pub mod work;
pub mod util;

pub static GENERIC_TERMS_AND_CONDITIONS: &str = include_str!("generic_terms.txt");
