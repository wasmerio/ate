mod test;
mod crypto;
mod header;
mod event;
mod conf;
mod comms;
mod redo;
mod validator;
mod compact;
mod index;
mod chain;
mod dio;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
