extern crate rand;
extern crate rand_chacha;

mod error;
mod test;
mod crypto;
mod header;
mod event;
mod conf;
mod comms;
mod redo;
mod session;
mod validator;
mod compact;
mod index;
mod lint;
mod transform;
mod plugin;
mod chain;
mod dio;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
