use serde::{Serialize, Deserialize};

#[allow(unused_imports)]
use std::io::Write;
#[allow(unused_imports)]
use super::event::Event;
use super::header::EmptyMeta;

pub trait ChainKey {
    fn name(&self) -> &String;

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

#[allow(dead_code)]
#[derive(Default)]
pub struct DiscreteChainKey {
    pub name: String,
}

impl DiscreteChainKey
{
    #[allow(dead_code)]
    pub fn with_name(mut self, name: String) -> DiscreteChainKey {
        self.name = name;
        self
    }
}

impl ChainKey for DiscreteChainKey {
    fn name(&self) -> &String { &self.name }
}
#[allow(dead_code)]
pub struct Chain {

}

#[test]
pub fn test_chain_key_mocking() {
    let cfg = DiscreteChainKey::default()
        .with_name("test_obj".to_string());
    assert_eq!(cfg.name(), "test_obj");
}