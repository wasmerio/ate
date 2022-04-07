use proc_macro2::Span;
use std::str::FromStr;
use syn::parse::{Parse, ParseStream, Result};
use syn::{LitStr, Token};
use wasm_bus_types::SerializationFormat;

pub struct ArgsFormat {
    pub format_token: kw::format,
    pub eq_token: Token![=],
    pub format_val: SerializationFormat,
}

#[derive(Default)]
pub struct Args {
    pub format: Option<ArgsFormat>,
}

mod kw {
    syn::custom_keyword!(format);
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Args> {
        try_parse(input)
    }
}

pub(crate) fn try_parse(input: ParseStream) -> Result<Args> {
    let mut args = Args::default();

    let lookahead = input.lookahead1();
    if lookahead.peek(kw::format) {
        args.format = Some(ArgsFormat {
            format_token: input.parse::<kw::format>()?,
            eq_token: input.parse()?,
            format_val: SerializationFormat::from_str(input.parse::<LitStr>()?.value().as_str())
                .map_err(|e| syn::Error::new(Span::call_site(), e))?,
        });
    } else {
        return Err(lookahead.error());
    }

    Ok(args)
}
