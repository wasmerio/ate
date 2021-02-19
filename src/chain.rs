use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use std::io::Write;
#[allow(unused_imports)]
use super::event::Event;
use super::header::EmptyMeta;

pub trait ChainKey {
    fn name(&self) -> String;

    fn to_key_str(&self) -> String {
        format!("{}", self.name())
    }
}

#[allow(dead_code)]
pub struct ChainOfTrust<M>
    where M: Serialize + Deserialize<'static> + Clone
{
    pub events: Vec<Event<M>>,
}

#[allow(dead_code)]
pub type DefaultChainOfTrust = ChainOfTrust<EmptyMeta>;

#[cfg(test)]
pub fn mock_test_chain_key() -> impl ChainKey {
    struct MockChainKey {}

    impl ChainKey for MockChainKey {
        fn name(&self) -> String { "test_obj".to_string() }
    }

    MockChainKey {}
}

#[test]
pub fn test_chain_key_mocking() {
    let cfg = mock_test_chain_key();
    assert_eq!(cfg.name(), "test_obj");
}