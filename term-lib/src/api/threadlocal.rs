use std::collections::HashMap;
use crate::wasmer::{Module, Store};

#[derive(Default)]
pub struct ThreadLocal
{
    pub store: Store,
    pub modules: HashMap<String, Module>,
}