#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone, Default)]
pub struct Val {
    pub var_eq: Option<String>,
    pub export: bool,
    pub readonly: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Environment {
    vars: HashMap<String, Val>,
}

impl Environment {
    pub fn set_var(&mut self, key: &str, val: String) {
        self.set_vareq_with_key(key.to_string(), val);
    }

    pub fn set_vareq(&mut self, var_eq: String) {
        let (key, value) = self.parse_key_value(&var_eq);
        self.set_vareq_with_key(key, value);
    }

    pub fn set_vareq_with_key(&mut self, key: String, var_eq: String) {
        match self.vars.entry(key) {
            Entry::Occupied(mut o) => {
                let v = o.get_mut();
                if !v.readonly {
                    v.var_eq = Some(var_eq);
                }
            }
            Entry::Vacant(o) => {
                o.insert(Val {
                    var_eq: Some(var_eq),
                    ..Default::default()
                });
            }
        }
    }

    pub fn unset(&mut self, key: &str) {
        if let Entry::Occupied(o) = self.vars.entry(key.to_string()) {
            if !o.get().readonly {
                o.remove();
            }
        }
    }

    pub fn export(&mut self, key: &str) {
        self.vars
            .entry(key.to_string())
            .or_insert(Val {
                ..Default::default()
            })
            .export = true;
    }

    pub fn readonly(&mut self, key: &str) {
        self.vars
            .entry(key.to_string())
            .or_insert(Val {
                ..Default::default()
            })
            .readonly = true;
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let entry = self.vars.get(key)?;

        return if let Some(var_eq) = &entry.var_eq {
            return Some(var_eq.clone());
        } else {
            None
        };
    }

    pub fn into_exported(self) -> Vec<String> {
        self.vars
            .into_iter()
            .filter(|(_, v)| v.export && v.var_eq.is_some())
            .map(|(_, v)| v.var_eq.unwrap())
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Val)> {
        self.vars.iter()
    }

    pub fn parse_key_value(&self, var_eq: &String) -> (String, String) {
        let mut split = var_eq.as_bytes().split(|b| *b == b'=');
        let key = String::from_utf8_lossy(split.next().unwrap()).to_string();
        let value = String::from_utf8_lossy(split.next().unwrap()).to_string();
        (key, value)
    }
}

pub fn empty() -> Environment {
    Environment {
        vars: HashMap::new(),
    }
}

pub fn from_system() -> Environment {
    let mut e = empty();
    env::vars().for_each(|(k, v)| {
        e.set_var(&k, v);
        e.export(&k);
    });
    e
}
