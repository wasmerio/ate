extern crate rand;
extern crate rand_chacha;
extern crate sha3;

mod error;
mod test;
mod crypto;
mod header;
mod meta;
mod event;
mod conf;
mod comms;
mod redo;
mod sink;
mod session;
mod validator;
mod compact;
mod index;
mod lint;
mod transform;
mod plugin;
mod signature;
mod chain;
mod dio;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
