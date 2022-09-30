use crate::wasmer::Module;
use std::collections::HashMap;

#[derive(Default)]
pub struct ThreadLocal {
    pub modules: HashMap<String, Module>,
}
