extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Block, ItemFn};

#[proc_macro_attribute]
pub fn proc_macro_error(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    if !is_proc_macro(&input.attrs) {
        return quote!(
            #input
            compile_error!("#[proc_macro_error] attribute can be used only with a proc-macro");
        )
        .into();
    }

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    let body = gen_body(block);

    quote!( #(#attrs)* #vis #sig { #body } ).into()
}

#[rustversion::since(1.37)]
fn gen_body(block: Box<Block>) -> proc_macro2::TokenStream {
    quote! {
        ::proc_macro_error::entry_point(|| #block )
    }
}

#[rustversion::before(1.37)]
fn gen_body(block: Box<Block>) -> proc_macro2::TokenStream {
    quote! {
        // FIXME:
        // proc_macro::TokenStream does not implement UnwindSafe until 1.37.0.
        // Considering this is the closure's return type the safety check would fail
        // for virtually every closure possible, the check is meaningless.
        ::proc_macro_error::entry_point(::std::panic::AssertUnwindSafe(|| #block ))
    }
}

fn is_proc_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path.is_ident("proc_macro")
            || attr.path.is_ident("proc_macro_derive")
            || attr.path.is_ident("proc_macro_attribute")
    })
}
