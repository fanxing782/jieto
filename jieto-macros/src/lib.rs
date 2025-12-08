use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn scheduled(attr: TokenStream, item: TokenStream) -> TokenStream {
    let cron_expr = parse_macro_input!(attr as LitStr);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_inputs = &input_fn.sig.inputs;

    if fn_inputs.is_empty() {
        return TokenStream::from(
            quote! { compile_error!("scheduled task must have exactly 1 parameter"); },
        );
    }

    let first_param = fn_inputs.first().unwrap();

    let arg_type = match first_param {
        syn::FnArg::Typed(pat_type) => &pat_type.ty,
        syn::FnArg::Receiver(_) => {
            return TokenStream::from(
                quote! { compile_error!("scheduled task cannot have self parameter"); },
            );
        }
    };

    let wrapper_name = to_upper_camel_case(fn_name);
    let wrapper = syn::Ident::new(&wrapper_name, fn_name.span());

    let expanded = quote! {
        #input_fn

        pub struct #wrapper;

        impl jieto_web::job::ScheduledTask for #wrapper {
            fn cron_expression(&self) -> &'static str {
                #cron_expr
            }

            fn task_name(&self) -> &'static str {
                stringify!(#fn_name)
            }

            fn execute(
                &self,
                injected: std::sync::Arc<dyn std::any::Any + Send + Sync>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
                if let Some(arg_ref) = injected.downcast_ref::<#arg_type>() {
                    let arg = arg_ref.clone();
                    return Box::pin(#fn_name(arg));
                }

                match injected.downcast::<#arg_type>() {
                    Ok(arc_arg) => {
                        Box::pin(async move {
                            #fn_name(arc_arg.as_ref().clone()).await
                        })
                    }
                    Err(_) => {
                        Box::pin(async move {})
                    }
                }
            }
        }
    };

    expanded.into()
}

#[proc_macro]
pub fn task(input: TokenStream) -> TokenStream {
    let fn_name = parse_macro_input!(input as syn::Ident);
    let wrapper_name = to_upper_camel_case(&fn_name);
    let wrapper = syn::Ident::new(&wrapper_name, fn_name.span());

    let expanded = quote! {
        Box::new(#wrapper) as Box<dyn jieto_web::job::ScheduledTask>
    };

    TokenStream::from(expanded)
}

fn to_upper_camel_case(ident: &syn::Ident) -> String {
    let name = ident.to_string();
    if name.is_empty() {
        return name;
    }

    let parts: Vec<&str> = name.split('_').collect();
    let converted: String = parts.iter()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();

    format!("JietoScheduled{}", converted)
}