mod test;
mod header;
mod meta;
mod event;
mod conf;
mod comms;
mod redo;
mod validator;
mod chain;
mod historian;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
