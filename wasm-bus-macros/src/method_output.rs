use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::*;
use derivative::*;

use super::return_trait::*;

#[derive(Debug, Clone)]
pub enum MethodOutput {
    Trait(ReturnTrait),
    Message(ReturnMessage),
    Nothing,
}

impl MethodOutput {
    pub fn is_trait(&self) -> bool {
        if let MethodOutput::Trait(_) = self {
            true
        } else {
            false
        }
    }

    pub fn ident(&self) -> TokenStream {
        match self {
            MethodOutput::Trait(a) => {
                let ident = a.ident.clone();
                quote! { #ident }
            }
            MethodOutput::Message(a) => {
                let path = a.path.clone();
                quote! { #path }
            }
            MethodOutput::Nothing => quote! { () },
        }
    }

    pub fn ident_client(&self) -> TokenStream {
        match self {
            MethodOutput::Trait(a) => {
                let ident = a.client_ident.clone();
                quote! { #ident }
            }
            MethodOutput::Message(a) => {
                let path = a.path.clone();
                quote! { #path }
            }
            MethodOutput::Nothing => quote! { () },
        }
    }

    pub fn ident_service(&self) -> TokenStream {
        match self {
            MethodOutput::Trait(a) => {
                let ident = a.service_ident.clone();
                quote! { #ident }
            }
            MethodOutput::Message(a) => {
                let path = a.path.clone();
                quote! { #path }
            }
            MethodOutput::Nothing => quote! { () },
        }
    }
}

impl Parse for MethodOutput {
    fn parse(input: ParseStream) -> Result<Self> {
        {
            let input_try = input.fork();
            if let Ok(trait_) = input_try.parse::<ReturnTrait>() {
                input.parse::<ReturnTrait>()?;
                return Ok(MethodOutput::Trait(trait_));
            }
        }

        let span = input.span();
        match input.parse::<ReturnType>()? {
            ReturnType::Default => Ok(MethodOutput::Nothing),
            ReturnType::Type(_, b) => Ok(MethodOutput::Message(ReturnMessage::new(span, b)?)),
        }
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ReturnMessage {
    #[derivative(Debug = "ignore")]
    pub path: Path,
}

impl ReturnMessage {
    fn new(span: Span, ty: Box<Type>) -> Result<Self> {
        match *ty {
            Type::Array(_) => Err(Error::new(span, "arrays are not supported as a return type")),
            Type::ImplTrait(_) => Err(Error::new(span, "trait implementations are not supported - instead wrap the dynamic type in an Arc. e.g. Arc<dyn MyType>")),
            Type::TraitObject(_) => Err(Error::new(span, "dynamic traits must be wrapped in an an Arc. e.g. Arc<dyn MyType>")),
            Type::Infer(_) => Err(Error::new(span, "all return types must be strongly typed")),
            Type::Tuple(_) => Err(Error::new(span, "returning tuples is not yet supported, instead make a struct that contains your named fields")),
            Type::Reference(_) => Err(Error::new(span, "returning reference types is not supported - all returned objects must be owned")),
            Type::Path(path) => {
                if path.qself.is_some() {
                    return Err(Error::new(span, "return self types are not supported"));
                }
                let path = path.path;
                if is_path_concrete(&path) == false {
                    return Err(Error::new(span, "only concrete types are supported as return arguments - traits must be returned in an Arc. e.g. Arc<dyn MyType> - otherwise lifetimes, bounds, etc... are all prohibited."));
                }
                if path.segments.is_empty() {
                    return Err(Error::new(span, "the return type does not return anything which is not supported by WASM bus."));
                }

                Ok(ReturnMessage {
                    path,
                })
            }
            _ => Err(Error::new(span, "the returned type is not supported")),
        }
    }
}

fn is_tuple_concrete(tuple: &TypeTuple) -> bool {
    for ty in tuple.elems.iter() {
        if is_type_concrete(ty) == false {
            return false;
        }
    }
    true
}

fn is_path_concrete(path: &Path) -> bool {
    for segment in path.segments.iter() {
        match &segment.arguments {
            PathArguments::None => {
                continue;
            }
            PathArguments::Parenthesized(_) => {
                return false;
            }
            PathArguments::AngleBracketed(angle) => {
                for arg in angle.args.iter() {
                    match arg {
                        GenericArgument::Type(ty) => {
                            if is_type_concrete(ty) == false {
                                return false;
                            }
                            continue;
                        }
                        _ => {
                            return false;
                        }
                    }
                }
            }
        }
    }

    true
}

fn is_type_concrete(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            if is_path_concrete(&path.path) == false {
                return false;
            }
        }
        Type::Tuple(tuple) => {
            if is_tuple_concrete(&tuple) == false {
                return false;
            }
        }
        _ => {
            return false;
        }
    }
    true
}
