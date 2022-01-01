use crate::args::Args;
use crate::parse::Item;
use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::quote;
use syn::parse::Parser;
use syn::{
    parse_quote, punctuated::Punctuated, Error, Field, FnArg, PathArguments, Token, TraitItem,
    TypeParamBound, TypeTraitObject,
};
use wasm_bus_types::SerializationFormat;

use super::method_inputs::*;
use super::method_output::*;

#[rustfmt::skip]
pub fn convert(args: Args, input: Item) -> proc_macro::TokenStream {
    let format = args
        .format
        .map(|a| a.format_val)
        .unwrap_or(SerializationFormat::Json);

    // Get the span
    let span = match input.clone() {
        Item::Trait(input) => input.ident.span(),
        Item::Impl(input) => {
            let trait_path = input.trait_.expect("you can only implement WASM traits").1;
            let trait_ident = trait_path.segments.last().expect("the trait path has no ident").ident.clone();
            trait_ident.span()
        },
        _ => panic!("not yet implemented")
    };

    // Convert the format to an identity so that it can be directly added
    let format = {
        let format = format.to_string().to_case(Case::Pascal);
        let format = Ident::new(format.as_str(), span);
        quote! {
            wasm_bus::abi::SerializationFormat::#format
        }
    };

    match input {
        Item::Trait(input) => {
            let trait_ident = input.ident.clone();
            let trait_name = trait_ident.to_string();

            let trait_client_name = format!("{}Client", trait_name);
            let trait_client_ident = Ident::new(trait_client_name.as_str(), span);

            let trait_service_name = format!("{}Service", trait_name);
            let trait_service_ident = Ident::new(trait_service_name.as_str(), span);

            let trait_simplified_name = format!("{}Simplified", trait_name);
            let trait_simplified_ident = Ident::new(trait_simplified_name.as_str(), span);

            let mut listens = Vec::new();
            let mut trait_methods = Vec::new();
            let mut blocking_methods = Vec::new();
            let mut blocking_client_method_impls = Vec::new();
            let mut trait_simplified_methods = Vec::new();
            let mut client_method_impls = Vec::new();
            let mut service_methods = Vec::new();
            let mut service_attach_points = Vec::new();
            let mut passthru_client_methods = Vec::new();
            let mut passthru_simplified_methods = Vec::new();
            let mut output = proc_macro2::TokenStream::new();

            // We process all the methods in the trait and emit code that supports
            // client and server side code
            for item in input.items {
                if let TraitItem::Method(method) = item {
                    let method_ident = method.sig.ident.clone();
                    let method_attrs = method.attrs;
                    let span = method_ident.span();

                    // Create a method name for blocking calls
                    let blocking_method_name = format!("blocking_{}", method_ident.to_string());
                    let blocking_method_ident = Ident::new(blocking_method_name.as_str(), span);

                    // Wasm bus is a fully asynchronous library and thus all its
                    // methods contained within must be asynchronous
                    let method_async = method.sig.asyncness;
                    if method_async.is_none() {
                        output.extend(Error::new(span, "all bus methods must be async").to_compile_error());
                        continue;
                    }

                    // Parse all the arguments of the method (if it has no self then fail)
                    let method_inputs = method.sig.inputs.clone();
                    let method_inputs: MethodInputs = parse_quote! { #method_inputs };
                    if method_inputs.has_self == false {
                        output.extend(Error::new(span, "all bus methods must be have a self reference").to_compile_error());
                        continue;
                    }

                    let request_name =
                        format!("{}_{}_request", trait_name, method_ident.to_string())
                            .to_case(Case::Pascal);
                    let request_name = Ident::new(request_name.as_str(), span);

                    let mut method_callbacks = Vec::new();
                    let mut method_lets = Vec::new();
                    let mut method_callback_handlers = Vec::new();
                    let mut method_transformed_inputs: Punctuated<_, Token![,]> = Punctuated::new();
                    let mut field_idents: Punctuated<_, Token![,]> = Punctuated::new();
                    let mut field_idents_plus: Punctuated<_, Token![,]> = Punctuated::new();
                    let mut fields: Punctuated<Field, Token![,]> = Punctuated::new();
                    for input in method_inputs.inputs {
                        let attrs = input.attrs.clone();
                        let name = input.ident.clone();
                        let ty = input.ty.as_ref().clone();
                        let span = name.span();

                        // If this is a callback then we need to type the input
                        // parameteter into an implementation that we will wrap
                        let (bounds, ty) = match ty.clone() {
                            syn::Type::ImplTrait(ty) => {
                                let ty = TypeTraitObject {
                                    dyn_token: None,
                                    bounds: ty.bounds.clone()
                                };
                                (Some(ty.bounds.clone()), syn::Type::TraitObject(ty))
                            },
                            syn::Type::TraitObject(_) => {
                                output.extend(Error::new(span, "callbacks must be explicit implementations and not dynamic traits - replace the 'dyn' with an 'impl'").to_compile_error());
                                continue;
                            },
                            ty => (None, ty)
                        };
                        if let Some(bounds) = bounds
                        {
                            // We only support a single field
                            let callback_fields = bounds
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
                            if callback_fields.len() != 1 {
                                panic!("WASM callbacks only support a single field as arguments")
                            }
                            let callback_field = callback_fields.into_iter().next().unwrap();
                            let callback_field_type = if let PathArguments::Parenthesized(a) = callback_field {
                                a.inputs
                                    .first()
                                    .map(|a| a.clone())
                                    .expect("WASM callbacks only support a single argument with a type")
                            } else {
                                panic!("WASM callbacks must have a single parenthesized field");
                            };
                            
                            let callback_name = format!(
                                "{}_{}_{}_callback",
                                trait_name,
                                method_ident.to_string(),
                                name
                            ).to_case(Case::Pascal);
                            let callback_name =
                                Ident::new(callback_name.as_str(), span);

                            // Create a struct that represents this callback
                            output.extend(quote! {
                                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                                pub struct #callback_name ( pub #callback_field_type );
                            }.into_iter());

                            // Add code that will register a callback to a user supplied function
                            method_callbacks.push(quote! {
                                .callback(#format, move |req: #callback_name| #name ( req.0 ) )
                            });

                            // We need a method argument that will accept in a callback implementation
                            // that we will invoke during callbacks
                            let arg: FnArg = parse_quote! {
                                #name : Box<dyn #ty + Send + Sync + 'static>
                            };
                            method_transformed_inputs.push(arg.clone());

                            // Add the callback handler to the service implementation
                            method_callback_handlers.push(quote! {
                                let #name = {
                                    let wasm_handle = wasm_handle.clone();
                                    Box::new(move |response: #callback_field_type| {
                                        let response = #callback_name ( response );
                                        wasm_bus::abi::reply_callback(wasm_handle, #format, response);
                                    })
                                };
                            });

                            field_idents_plus.push(name.clone());

                        } else {
                            fields.push(
                                Field::parse_named
                                    .parse2(parse_quote! {
                                        #( #attrs )* pub #name : #ty
                                    })
                                    .unwrap(),
                            );

                            field_idents.push(name.clone());
                            field_idents_plus.push(name.clone());

                            method_transformed_inputs.push(FnArg::Typed(input.pat_type));

                            method_lets.push(quote! {
                                let #name = wasm_req.#name;
                            });
                        }
                    }

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

                    // Get the return type
                    let method_ret = method.sig.output.clone();
                    let method_ret = match syn::parse::<MethodOutput>(quote!(#method_ret).into()) {
                        Ok(a) => a,
                        Err(err) => {
                            output.extend(err.to_compile_error());
                            return proc_macro::TokenStream::from(output);
                        }
                    };

                    // Attempt to parse the type into an object
                    if method_ret.is_trait() {
                        let svc = method_ret.ident_service();
                        service_attach_points.push(quote! {
                            {
                                let wasm_me = wasm_me.clone();
                                wasm_bus::task::respond_to(
                                    session_handle,
                                    #format,
                                    #[allow(unused_variables)]
                                    move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: #request_name| {
                                        let wasm_me = wasm_me.clone();
                                        #( #method_lets )*
                                        async move {
                                            #( #method_callback_handlers )*
                                            let svc = wasm_me.#method_ident(#field_idents_plus).await?;
                                            #svc::attach(svc, wasm_handle);
                                            std::result::Result::<(), _>::Err(wasm_bus::abi::CallError::Fork)
                                        }
                                    },
                                );
                            }
                        });
                    }

                    // Otherwise the operation can be invoked arbitarily so we emit an object
                    // thats used to make the actual invocation
                    else 
                    {
                        service_attach_points.push(quote! {
                            {
                                let wasm_me = wasm_me.clone();
                                wasm_bus::task::respond_to(
                                    session_handle,
                                    #format,
                                    #[allow(unused_variables)]
                                    move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: #request_name| {
                                        let wasm_me = wasm_me.clone();
                                        #( #method_lets )*
                                        async move {
                                            #( #method_callback_handlers )*
                                            wasm_me.#method_ident(#field_idents_plus).await
                                        }
                                    },
                                );
                            }
                        });
                    }

                    // If the method returns another service then we need to add special things
                    // that stop the method from immediately returning
                    if method_ret.is_trait() {
                        let svc = method_ret.ident_service();
                        listens.push(quote! {
                            {
                                let wasm_me = wasm_me.clone();
                                wasm_bus::task::listen(
                                    #format,
                                    #[allow(unused_variables)]
                                    move |wasm_handle: wasm_bus::abi::CallHandle, wasm_req: #request_name| {
                                        let wasm_me = wasm_me.clone();
                                        #( #method_lets )*
                                        async move {
                                            #( #method_callback_handlers )*
                                            let svc = wasm_me.#method_ident(#field_idents_plus).await?;
                                            #svc::attach(svc, wasm_handle);
                                            std::result::Result::<(), _>::Err(wasm_bus::abi::CallError::Fork)
                                        }
                                    }
                                );
                            }
                        });

                        let ret = method_ret.ident();
                        let ret_client = method_ret.ident_client();
                        trait_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError>;
                        });
                        trait_simplified_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError>;
                        });
                        client_method_impls.push(quote! {
                            pub async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                let request = #request_name {
                                    #field_idents
                                };
                                let task = wasm_bus::abi::call_ext(
                                    self.parent.clone(),
                                    self.wapm.clone(),
                                    #format,
                                    self.session.clone(),
                                    request)
                                #( #method_callbacks )*
                                .invoke();
                                let ret = Arc::new(#ret_client::attach(task.wapm(), task.session().map(|a| a.to_string()), task.handle()));
                                let _: () = task.join().await?;
                                Ok(ret)
                            }
                        });
                        blocking_methods.push(quote! {
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError>;
                        });
                        blocking_client_method_impls.push(quote! {
                            pub fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                wasm_bus::task::block_on(self.#method_ident(#field_idents_plus))
                            }
                        });
                        passthru_client_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                #trait_client_ident::#method_ident(self, #field_idents_plus).await
                            }
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                #trait_client_ident::#blocking_method_ident(self, #field_idents_plus)
                            }
                        });
                        passthru_simplified_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                #trait_simplified_ident::#method_ident(self, #field_idents_plus).await
                            }
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<std::sync::Arc<dyn #ret + Send + Sync + 'static>, wasm_bus::abi::CallError> {
                                wasm_bus::task::block_on(#trait_simplified_ident::#method_ident(self, #field_idents_plus))
                            }
                        });
                    } else {
                        listens.push(quote! {
                            {
                                let wasm_me = wasm_me.clone();
                                wasm_bus::task::listen(
                                    #format,
                                    #[allow(unused_variables)]
                                    move |_wasm_handle: wasm_bus::abi::CallHandle, wasm_req: #request_name| {
                                        let wasm_me = wasm_me.clone();
                                        #( #method_lets )*
                                        async move {
                                            #( #method_callback_handlers )*
                                            wasm_me.#method_ident(#field_idents_plus).await
                                        }
                                    }
                                );
                            }
                        });

                        let ret = method_ret.ident();
                        trait_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError>;
                        });
                        trait_simplified_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> #ret;
                        });
                        client_method_impls.push(quote! {
                            pub async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                let request = #request_name {
                                    #field_idents
                                };
                                wasm_bus::abi::call_ext(
                                    self.parent.clone(),
                                    self.wapm.clone(),
                                    #format,
                                    self.session.clone(),
                                    request)
                                #( #method_callbacks )*
                                .invoke()
                                .join()
                                .await
                            }
                        });
                        blocking_methods.push(quote! {
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError>;
                        });
                        blocking_client_method_impls.push(quote! {
                            pub fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                wasm_bus::task::block_on(self.#method_ident(#field_idents_plus))
                            }
                        });
                        passthru_client_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                #trait_client_ident::#method_ident(self, #field_idents_plus).await
                            }
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                #trait_client_ident::#blocking_method_ident(self, #field_idents_plus)
                            }
                        });
                        passthru_simplified_methods.push(quote! {
                            async fn #method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                Ok(#trait_simplified_ident::#method_ident(self, #field_idents_plus).await)
                            }
                            fn #blocking_method_ident ( &self, #method_transformed_inputs ) -> std::result::Result<#ret, wasm_bus::abi::CallError> {
                                Ok(wasm_bus::task::block_on(#trait_simplified_ident::#method_ident(self, #field_idents_plus)))
                            }
                        });
                    }

                    let ret = method_ret.ident_service();
                    service_methods.push(quote! {
                        #( #method_attrs )*
                        async fn #method_ident ( &self, #method_transformed_inputs ) -> #ret;
                    });
                }
            }

            // First of all we emit out the trait itself (unmodified) except
            // for the removal of the wasm_bus key word and replacement
            // of it with the async-trait
            output.extend(quote! {
                #[async_trait::async_trait]
                pub trait #trait_ident
                where Self: std::fmt::Debug + Send + Sync
                {
                    #( #trait_methods )*

                    #( #blocking_methods )*

                    fn as_client(&self) -> Option<#trait_client_ident>;

                    fn handle(&self) -> Option<wasm_bus::abi::CallHandle>;
                }

                #[async_trait::async_trait]
                pub trait #trait_simplified_ident
                where Self: std::fmt::Debug + Send + Sync
                {
                    #( #trait_simplified_methods )*
                }

                #[async_trait::async_trait]
                impl<T> #trait_ident
                for T
                where T: #trait_simplified_ident
                {
                    #( #passthru_simplified_methods )*

                    fn as_client(&self) -> Option<#trait_client_ident> {
                        None
                    }

                    fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
                        None
                    }
                }
            });

            // Every trait also has a service implementation
            output.extend(
                quote! {
                    #[derive(Debug, Clone)]
                    pub struct #trait_service_ident
                    {
                    }

                    impl #trait_service_ident
                    {
                        #[allow(dead_code)]
                        pub(crate) fn attach(wasm_me: std::sync::Arc<dyn #trait_ident + Send + Sync + 'static>, session_handle: wasm_bus::abi::CallHandle) {
                            #( #service_attach_points )*
                        }

                        pub fn listen(wasm_me: std::sync::Arc<dyn #trait_ident + Send + Sync + 'static>) {
                            #( #listens )*
                        }
    
                        pub fn serve() {
                            wasm_bus::task::serve();
                        }
                    }
                }
            );

            // If there is a reference argument then we need to emit a struct
            // that will represent this invokable object
            output.extend(
                quote! {
                    #[derive(Debug, Clone)]
                    pub struct #trait_client_ident
                    {
                        wapm: std::borrow::Cow<'static, str>,
                        session: Option<String>,
                        parent: Option<wasm_bus::abi::CallHandle>,
                        task: Option<wasm_bus::abi::Call>,
                        join: Option<wasm_bus::abi::CallJoin<()>>,
                    }

                    impl #trait_client_ident {
                        pub fn new(wapm: &str) -> Self {
                            Self {
                                wapm: wapm.to_string().into(),
                                session: None,
                                parent: None,
                                task: None,
                                join: None,
                            }
                        }

                        pub fn new_with_session(wapm: &str, session: &str) -> Self {
                            Self {
                                wapm: wapm.to_string().into(),
                                session: Some(session.to_string()),
                                parent: None,
                                task: None,
                                join: None,
                            }
                        }

                        pub fn attach(wapm: std::borrow::Cow<'static, str>, session: Option<String>, parent: wasm_bus::abi::CallHandle) -> Self {
                            Self {
                                wapm,
                                session,
                                parent: Some(parent),
                                task: None,
                                join: None,
                            }
                        }

                        pub fn id(&self) -> u32 {
                            self.task.as_ref().map(|a| a.id()).unwrap_or(
                                self.parent.map(|a| a.id).unwrap_or(0u32)
                            )
                        }

                        pub fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
                            if let Some(handle) = self.task.as_ref().map(|a| a.handle()) {
                                return Some(handle);
                            }
                            self.parent.clone()
                        }
                        
                        pub fn wait(self) -> Result<(), wasm_bus::abi::CallError> {
                            if let Some(join) = self.join {
                                join.wait()?;
                            }
                            if let Some(task) = self.task {
                                task.join().wait()?;
                            }
                            Ok(())
                        }
                        
                        pub fn try_wait(&mut self) -> Result<Option<()>, wasm_bus::abi::CallError> {
                            if let Some(task) = self.task.take() {
                                self.join.replace(task.join());
                            }
                            if let Some(join) = self.join.as_mut() {
                                join.try_wait()
                            } else {
                                Ok(None)
                            }
                        }

                        #( #client_method_impls )*

                        #( #blocking_client_method_impls )*
                    }

                    impl std::future::Future
                    for #trait_client_ident
                    {
                        type Output = Result<(), wasm_bus::abi::CallError>;

                        fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                            if let Some(task) = self.task.take() {
                                self.join.replace(task.join());
                            }
                            if let Some(join) = self.join.as_mut() {
                                let join = std::pin::Pin::new(join);
                                return join.poll(cx);
                            } else {
                                std::task::Poll::Ready(Ok(()))
                            }
                        }
                    }

                    #[async_trait::async_trait]
                    impl #trait_ident
                    for #trait_client_ident
                    {
                        #( #passthru_client_methods )*

                        fn as_client(&self) -> Option<#trait_client_ident> {
                            Some(self.clone())
                        }

                        fn handle(&self) -> Option<wasm_bus::abi::CallHandle> {
                            #trait_client_ident::handle(self)
                        }
                    }
                }
                .into_iter(),
            );

            // Return the token stream
            //panic!("CODE {}", proc_macro::TokenStream::from(output));
            proc_macro::TokenStream::from(output)
        }
        _ => {
            panic!("the wasm bus trait can only be used on traits");
        }
    }
}
