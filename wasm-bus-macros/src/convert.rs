use crate::args::Args;
use crate::parse::Item;
use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::quote;
use syn::parse::Parser;
use syn::{
    parse_quote, punctuated::Punctuated, Field, FnArg, Pat, PathArguments, Token, TraitItem,
    TypeParamBound,
};
use wasm_bus_types::SerializationFormat;

#[rustfmt::skip]
pub fn convert(args: Args, input: Item) -> proc_macro::TokenStream {
    let format = args
        .format
        .map(|a| a.format_val)
        .unwrap_or(SerializationFormat::Json);

    match input {
        Item::Trait(input) => {
            let trait_ident = input.ident.clone();
            let trait_name = trait_ident.to_string();

            let mut trait_static_methods = Vec::new();
            let mut trait_self_methods = Vec::new();
            let mut output = proc_macro2::TokenStream::new();

            for item in input.items {
                if let TraitItem::Method(method) = item {
                    let method_ident = method.sig.ident.clone();
                    let span = method_ident.span();

                    let method_with_session_ident =
                        format!("{}_with_session", method_ident.to_string());
                    let method_with_session_ident =
                        Ident::new(method_with_session_ident.as_str(), span);

                    let request_name =
                        format!("{}_{}_request", trait_name, method_ident.to_string())
                            .to_case(Case::Pascal);
                    let request_name = Ident::new(request_name.as_str(), span);

                    let call_name = format!("{}_{}_call", trait_name, method_ident.to_string())
                        .to_case(Case::Pascal);
                    let call_name = Ident::new(call_name.as_str(), span);

                    let mut has_self = None;

                    // Convert the format to an identity so t hat it can be directly added
                    let format = {
                        let format = format.to_string().to_case(Case::Pascal);
                        let format = Ident::new(format.as_str(), span);
                        quote! {
                            wasm_bus::abi::SerializationFormat::#format
                        }
                    };

                    let mut callbacks = Vec::new();
                    let mut method_inputs: Punctuated<_, Token![,]> = Punctuated::new();
                    let mut field_idents: Punctuated<_, Token![,]> = Punctuated::new();
                    let mut fields: Punctuated<Field, Token![,]> = Punctuated::new();
                    for arg in method.sig.inputs {
                        match arg {
                            FnArg::Receiver(a) => {
                                has_self = Some(a);
                            }
                            FnArg::Typed(typed_arg) => {
                                if let Pat::Ident(arg_name) = typed_arg.pat.as_ref() {
                                    let attrs = arg_name.attrs.clone();
                                    let name = arg_name.ident.clone();
                                    let mut ty = typed_arg.ty.as_ref().clone();

                                    // If this is a callback then we need to type the input
                                    // parameteter into an implementation that we will wrap
                                    if let syn::Type::TraitObject(ty) = &mut ty {
                                        ty.dyn_token = None;

                                        let callback_fields = ty
                                            .bounds
                                            .clone()
                                            .into_iter()
                                            .filter_map(|a| {
                                                if let TypeParamBound::Trait(a) = a {
                                                    Some(a)
                                                } else {
                                                    None
                                                }
                                            })
                                            .flat_map(|a| a.path.segments.into_iter())
                                            .map(|a| a.arguments)
                                            .collect::<Vec<_>>();
                                        let callback_fields = {
                                            let mut a = Punctuated::<_, Token![,]>::new();
                                            a.extend(
                                                callback_fields
                                                    .into_iter()
                                                    .filter_map(|a| {
                                                        if let PathArguments::Parenthesized(a) = a {
                                                            Some(a.inputs)
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .flat_map(|a| a.into_iter())
                                                    .map(|a| quote! { pub #a }),
                                            );
                                            a
                                        };

                                        let callback_name = format!(
                                            "{}_{}_{}_callback",
                                            trait_name,
                                            method_ident.to_string(),
                                            name
                                        )
                                        .to_case(Case::Pascal);
                                        let callback_name =
                                            Ident::new(callback_name.as_str(), span);

                                        // Create a struct that represents this callback
                                        output.extend(quote! {
                                            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                                            pub struct #callback_name ( #callback_fields );
                                        }.into_iter());

                                        // Add the callback handler to the invocation
                                        callbacks.push(quote! {
                                            .callback(#format, move |req: #callback_name| #name ( req.0 ) )
                                        });

                                        let arg: FnArg = parse_quote! {
                                            #name : impl #ty + Send + 'static
                                        };
                                        method_inputs.push(arg.clone());
                                    } else {
                                        fields.push(
                                            Field::parse_named
                                                .parse2(parse_quote! {
                                                    #( #attrs )* pub #name : #ty
                                                })
                                                .unwrap(),
                                        );

                                        field_idents.push(arg_name.ident.clone());

                                        method_inputs.push(FnArg::Typed(typed_arg));
                                    }
                                }
                            }
                        }
                    }

                    // Get the return type
                    let method_ret = method.sig.output.clone();

                    // All the methods within the trait need to have a data object to pass
                    // via the WASM bus
                    output.extend(
                        quote! {
                            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                            pub struct #request_name {
                                #fields
                            }
                        }
                        .into_iter(),
                    );

                    // There has to be some return value
                    let mut ret_type = quote! { #call_name };
                    let mut is_ret_trait = false;
                    let method_ret: syn::Type = match method_ret {
                        syn::ReturnType::Type(_, mut ty) => {
                            if let syn::Type::TraitObject(ty) = ty.as_mut() {
                                ty.dyn_token = None;
                                is_ret_trait = true;
                                ret_type = quote! { #ty };
                                parse_quote! { #ty }
                            } else {
                                parse_quote! { #ty }
                            }
                        }
                        syn::ReturnType::Default => parse_quote! { () },
                    };

                    // When the operation can be invoked arbitarily then we emit an object
                    // thats used to make the actual invocation
                    if is_ret_trait == false {
                        output.extend(
                            quote! {
                                pub struct #call_name {
                                    task: wasm_bus::abi::Call
                                }

                                impl #call_name {
                                    /// Allow the caller to wait for the result of the invocation
                                    pub fn join(self) -> wasm_bus::abi::CallJoin<#method_ret>
                                    {
                                        self.task.join()
                                    }
                                }
                            }
                            .into_iter(),
                        );
                    }

                    // When it contains a self reference then its a call made relative to another
                    // call which thus allows for context aware invocations.
                    if let Some(_) = has_self {
                        trait_self_methods.push(quote! {
                            pub fn #method_ident ( &self, #method_inputs ) -> #ret_type {
                                let request = #request_name {
                                    #field_idents
                                };
                                #ret_type {
                                    task: self.task
                                        .call(#format, None, request)
                                        #( #callbacks )*
                                        .invoke()
                                }                                
                            }

                            pub fn #method_with_session_ident ( &self, session: &str, #method_inputs ) -> #ret_type {
                                let session = Some(session.to_string());
                                let request = #request_name {
                                    #field_idents
                                };
                                #ret_type {
                                    task: self.task
                                        .call(#format, session, request)
                                        #( #callbacks )*
                                        .invoke()
                                }                                
                            }
                        });

                    // Otherwise its a static method that will launch a particular call on
                    // the WASM bus
                    } else {
                        trait_static_methods.push(quote! {
                            pub fn #method_ident ( wapm: &str, #method_inputs ) -> #ret_type {
                                let request = #request_name {
                                    #field_idents
                                };
                                #ret_type {
                                    task: wasm_bus::abi::call(wapm.to_string().into(), #format, None, request)
                                        #( #callbacks )*
                                        .invoke()
                                }
                            }

                            pub fn #method_with_session_ident ( wapm: &str, session: &str, #method_inputs ) -> #ret_type {
                                let session = Some(session.to_string());
                                let request = #request_name {
                                    #field_idents
                                };
                                #ret_type {
                                    task: wasm_bus::abi::call(wapm.to_string().into(), #format, session, request)
                                        #( #callbacks )*
                                        .invoke()
                                }
                            }
                        });
                    }
                }
            }

            // If there is a reference argument then we need to emit a struct
            // that will represent this invokable object
            output.extend(
                quote! {
                    #[derive(Debug, Clone)]
                    pub struct #trait_ident {
                        task: wasm_bus::abi::Call
                    }

                    impl #trait_ident {
                        pub fn id(&self) -> u32 {
                            self.task.id()
                        }

                        /// Finishes the session and cleans up resources
                        pub fn join(self) -> wasm_bus::abi::CallJoin<()>
                        {
                            self.task.join()
                        }

                        #( #trait_self_methods )*

                        #( #trait_static_methods )*
                    }
                }
                .into_iter(),
            );

            // Return the token stream
            //panic!("CODE {}", proc_macro::TokenStream::from(output));
            proc_macro::TokenStream::from(output)
        }
        _ => {
            panic!("not yet implemented");
        }
    }
}
