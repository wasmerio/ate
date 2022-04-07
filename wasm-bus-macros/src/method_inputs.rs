use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::FnArg;
use syn::*;

pub struct MethodInputs {
    pub inputs: Punctuated<MethodInput, Token![,]>,
    pub has_self: bool,
}

impl Parse for MethodInputs {
    fn parse(input: ParseStream) -> Result<Self> {
        let args: Punctuated<FnArg, Token![,]> = Punctuated::parse_terminated(input)?;

        let mut inputs: Punctuated<MethodInput, Token![,]> = Punctuated::new();
        let mut has_self = false;

        for arg in args {
            match arg {
                FnArg::Receiver(a) => {
                    if let Some((_, None)) = a.reference {
                    } else {
                        return Err(Error::new(
                            input.span(),
                            "all bus methods must have an immutable self reference",
                        ));
                    }
                    if a.mutability.is_some() {
                        return Err(Error::new(input.span(), "bus methods can not be mutable"));
                    }
                    has_self = true;
                }
                FnArg::Typed(typed_arg) => match typed_arg.pat.as_ref() {
                    Pat::Ident(arg_name) => {
                        inputs.push(MethodInput::new(typed_arg.clone(), arg_name.clone()));
                    }
                    _ => {
                        return Err(Error::new(
                            input.span(),
                            "only named arguments are supported",
                        ));
                    }
                },
            }
        }

        Ok(MethodInputs { inputs, has_self })
    }
}

pub struct MethodInput {
    pub attrs: Vec<Attribute>,
    pub ident: Ident,
    pub pat_ident: PatIdent,
    pub pat_type: PatType,
    pub ty_attrs: Vec<Attribute>,
    pub ty: Box<Type>,
}

impl MethodInput {
    fn new(pat_type: PatType, pat_ident: PatIdent) -> Self {
        MethodInput {
            attrs: pat_ident.attrs.clone(),
            ident: pat_ident.ident.clone(),
            pat_ident,
            ty_attrs: pat_type.attrs.clone(),
            ty: pat_type.ty.clone(),
            pat_type: pat_type,
        }
    }
}
