extern crate proc_macro;

use version_check::Version;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, Attribute, parse_macro_input};


#[proc_macro_attribute]
pub fn proc_macro_error(
    _attr: TokenStream,
    input: TokenStream)
-> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    if !is_proc_macro(&input.attrs) {
        return quote!(
            #input
            compile_error!("#[proc_macro_error] attribute can be used only with a proc-macro");
        ).into()
    }

    let ItemFn { attrs, vis, sig, block } = input;

    let version = Version::read().unwrap();
    let body = if version.at_least("1.37.0") {
        quote! {
            ::proc_macro_error::entry_point(|| #block )
        }
    } else {
        quote! {
            // FIXME:
            // proc_macro::TokenStream does not implement UnwindSafe until 1.37.0.
            // Considering this is the closure's return type the safety check would fail
            // for virtually every closure possible, the check is meaningless.
            ::proc_macro_error::entry_point(::std::panic::AssertUnwindSafe(|| #block ))
        }
    };

    quote!( #(#attrs)* #vis #sig { #body } ).into()
}

fn is_proc_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path.is_ident("proc_macro") ||
        attr.path.is_ident("proc_macro_derive") ||
        attr.path.is_ident("proc_macro_attribute")
    })
}