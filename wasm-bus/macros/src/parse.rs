use proc_macro2::Span;
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::{Attribute, ItemImpl, ItemStruct, ItemTrait, Token};

#[derive(Clone)]
pub enum Item {
    Trait(ItemTrait),
    Impl(ItemImpl),
    Struct(ItemStruct),
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let mut lookahead = input.lookahead1();
        while lookahead.peek(Token![unsafe]) || lookahead.peek(Token![pub]) {
            let ahead = input.fork();
            if lookahead.peek(Token![unsafe]) {
                ahead.parse::<Token![unsafe]>()?;
            }
            if lookahead.peek(Token![pub]) {
                ahead.parse::<Token![pub]>()?;
            }
            lookahead = ahead.lookahead1();
        }
        if lookahead.peek(Token![trait]) {
            let mut item: ItemTrait = input.parse()?;
            item.attrs = attrs;
            Ok(Item::Trait(item))
        } else if lookahead.peek(Token![struct]) {
            let mut item: ItemStruct = input.parse()?;
            item.attrs = attrs;
            Ok(Item::Struct(item))
        } else if lookahead.peek(Token![impl]) {
            let mut item: ItemImpl = input.parse()?;
            if item.trait_.is_none() {
                return Err(Error::new(Span::call_site(), "expected a trait impl"));
            }
            item.attrs = attrs;
            Ok(Item::Impl(item))
        } else {
            Err(lookahead.error())
        }
    }
}
