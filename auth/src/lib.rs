#![cfg_attr(not(debug_assertions), allow(dead_code, unused_imports, unused_variables))]

mod model;

pub use crate::model::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
