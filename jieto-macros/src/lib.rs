use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn scheduled(attr: TokenStream, item: TokenStream) -> TokenStream {
    let cron_expr = parse_macro_input!(attr as LitStr);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;
    let fn_asyncness = &input_fn.sig.asyncness;

    let wrapper_name = syn::Ident::new(&format!("__JietoScheduled_{}", fn_name), fn_name.span());

    let expanded = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name() #fn_block

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #wrapper_name;

        impl #wrapper_name {
            pub const CRON: &'static str = #cron_expr;
            pub const NAME: &'static str = stringify!(#fn_name);
        }

        impl ScheduledTask for #wrapper_name {
            fn cron_expression(&self) -> &'static str {
                Self::CRON
            }

            fn task_name(&self) -> &'static str {
                Self::NAME
            }

            fn execute(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'static>> {
                Box::pin(#fn_name())
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn task(input: TokenStream) -> TokenStream {
    let fn_name = parse_macro_input!(input as syn::Ident);
    let wrapper_name = syn::Ident::new(&format!("__JietoScheduled_{}", fn_name), fn_name.span());

    let expanded = quote! {
        Box::new(#wrapper_name) as Box<dyn ScheduledTask>
    };

    TokenStream::from(expanded)
}
