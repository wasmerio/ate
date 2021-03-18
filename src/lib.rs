#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

/// You can change the hashing routine with these features
/// - feature = "use_blake3"
/// - feature = "use_sha3"

mod test;
pub mod error;
pub mod crypto;
pub mod header;
pub mod meta;
pub mod event;
pub mod conf;
pub mod comms;
pub mod mesh;
pub mod redo;
pub mod sink;
pub mod session;
pub mod validator;
pub mod compact;
pub mod index;
pub mod lint;
pub mod transform;
pub mod plugin;
pub mod signature;
pub mod time;
pub mod tree;
pub mod chain;
pub mod accessor;
pub mod single;
pub mod multi;
pub mod transaction;
pub mod dio;
pub mod pipe;
pub mod prelude;
pub mod anti_replay;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
