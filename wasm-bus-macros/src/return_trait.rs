use derivative::*;
use syn::parse::{Parse, ParseStream};
use syn::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ReturnTrait {
    #[derivative(Debug = "ignore")]
    pub path: Path,
    pub ident: Ident,
    #[derivative(Debug = "ignore")]
    pub ty: Type,
    pub client_name: String,
    pub client_ident: Ident,
    pub service_name: String,
    pub service_ident: Ident,
}

impl Parse for ReturnTrait {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Option<Token![->]>>()?;

        let span = input.span();
        let path = input.parse::<Path>()?;

        let arc_segment = match path.segments.into_iter().last() {
            Some(a) if a.ident == "Arc" => a,
            _ => {
                return Err(Error::new(
                    span,
                    "only Arc is supported as a WASM trait return type",
                ));
            }
        };

        let span = arc_segment.ident.span();
        let inner = match arc_segment.arguments {
            PathArguments::AngleBracketed(brackets) => {
                let args = brackets.args;
                if args.len() != 1 {
                    return Err(Error::new(span, "the Arc must contain a dynamic trait"));
                }
                args.into_iter().last().unwrap()
            }
            _ => {
                return Err(Error::new(span, "the Arc must contain a dynamic trait"));
            }
        };

        let ty = match inner {
            GenericArgument::Type(a) => a,
            _ => {
                return Err(Error::new(
                    span,
                    "bindings, lifetimes, contraints and consts are not supported",
                ));
            }
        };

        let trait_bound = match ty.clone() {
            Type::TraitObject(a) => a
                .bounds
                .into_iter()
                .filter_map(|a| {
                    if let TypeParamBound::Trait(a) = a {
                        Some(a)
                    } else {
                        None
                    }
                })
                .next(),
            _ => {
                return Err(Error::new(span, "the Arc must contain a dynamic trait"));
            }
        };
        let trait_bound = match trait_bound {
            Some(a) => a,
            None => {
                return Err(Error::new(
                    span,
                    "the Arc must contain a dynamic trait without any other bounds",
                ));
            }
        };

        let path = trait_bound.path;
        let ident = if let Some(last) = path.segments.last() {
            last.ident.clone()
        } else {
            return Err(Error::new(
                input.span(),
                "return type must be a type identifier",
            ));
        };

        let client_name = format!("{}Client", ident.to_string());
        let client_ident = Ident::new(client_name.as_str(), span.clone());

        let service_name = format!("{}Service", ident.to_string());
        let service_ident = Ident::new(service_name.as_str(), span.clone());

        let ret = ReturnTrait {
            path,
            ident,
            ty,
            client_name,
            client_ident,
            service_name,
            service_ident,
        };

        Ok(ret)
    }
}
