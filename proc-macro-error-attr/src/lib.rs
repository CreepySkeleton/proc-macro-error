extern crate proc_macro;

use version_check::Version;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Attribute, parse_macro_input};


#[proc_macro_attribute]
pub fn proc_macro_error(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream)
-> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    match impl_proc_macro_error(&input) {
        Ok(ts) => ts.into(),
        Err(e) => quote!(#input #e).into()
    }
}

fn impl_proc_macro_error(input: &ItemFn) -> Result<TokenStream, TokenStream> {
    if !is_proc_macro(&input.attrs) {
        return Err(quote! {
            compile_error!("#[proc_macro_error] attribute can be used only with a proc-macro");
        });
    }

    let ItemFn { attrs, vis, sig, block } = input;

    let version = Version::read().unwrap();
    let body = if version.at_least("1.37.0") {
        quote! {
            ::proc_macro_error::entry_point(|| #block )
        }
    } else {
        quote! {
            ::proc_macro_error::entry_point(::std::panic::AssertUnwindSafe(|| #block ))
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            #body
        }
    })
}

fn is_proc_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path.is_ident("proc_macro") ||
        attr.path.is_ident("proc_macro_derive") ||
        attr.path.is_ident("proc_macro_attribute")
    })
}