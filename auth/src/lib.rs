pub mod helper;
pub mod flow;
mod test;
pub mod model;
pub mod service;
pub mod cmd;
pub mod error;
pub mod request;
pub mod prelude;
pub mod opt;
pub mod work;

pub static GENERIC_TERMS_AND_CONDITIONS: &str = include_str!("generic_terms.txt");