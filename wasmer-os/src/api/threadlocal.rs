use crate::wasmer::{Module, Store};
use std::collections::HashMap;

#[derive(Default)]
pub struct ThreadLocal {
    pub store: Store,
    pub modules: HashMap<String, Module>,
}
