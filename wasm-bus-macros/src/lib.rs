#![allow(
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::module_name_repetitions,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::too_many_lines
)]

extern crate proc_macro;

mod args;
mod convert;
mod method_inputs;
mod method_output;
mod parse;
mod receiver;
mod return_trait;

use crate::args::Args;
use crate::convert::convert;
use crate::parse::Item;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn wasm_bus(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let item = parse_macro_input!(input as Item);
    convert(args, item)
}
