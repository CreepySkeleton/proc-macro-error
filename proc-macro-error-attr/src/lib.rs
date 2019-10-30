extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use std::iter::FromIterator;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Attribute, Token,
};
use syn_mid::{Block, ItemFn};

use self::Setting::*;

/// **Either this attribute or [`proc_macro_error::entry_point`][ep] MUST be present
/// on the top level of your macro.**
///
/// This attribute helps you build the right [`entry_point`][ep] invocation while
/// keeping the indentation level.
///
/// # Syntax
///
/// `#[proc_macro_error]` or `#[proc_macro_error(settings...)]`, where `settings...`
/// is a comma-separated list of:
///
/// - `allow_not_macro`:
///
///     By default, the attribute checks that it's applied to a proc-macro.
///     If none of `#[proc_macro]`, `#[proc_macro_derive]` nor `#[proc_macro_attribute]` are
///     present it will panic. It's the intention - this crate is supposed to be used only with
///     proc-macros. This setting is made to bypass the check, useful in certain
///     circumstances.
///
/// - `assert_unwind_safe`:
///
///     Tells `proc_macro_error` that the code passed to [`entry_point`][ep]
///     should we wrapped with [`AssertUnwindSafe`].
///
///     This setting is implied if `#[proc_macro_error]` is placed on top of a function
///     marked as `#[proc_macro]`, `#[proc_macro_derive]` or `#[proc_macro_attribute]`.
///
/// [ep]: https://docs.rs/proc-macro-error/0.3/proc_macro_error/fn.entry_point.html
/// [`AssertUnwindSafe`]: https://doc.rust-lang.org/std/panic/struct.AssertUnwindSafe.html
///
#[proc_macro_attribute]
pub fn proc_macro_error(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let settings = match syn::parse::<Settings>(attr) {
        Ok(settings) => settings,
        Err(err) => {
            let err = err.to_compile_error();
            return quote!(#input #err).into();
        }
    };

    let allow_not_macro = settings.is_set(AllowNotMacro);
    let is_proc_macro = is_proc_macro(&input.attrs);
    let assert_unwind_safe = settings.is_set(AssertUnwindSafe) || is_proc_macro;

    if !(allow_not_macro || is_proc_macro) {
        return quote!(
            #input
            compile_error!("#[proc_macro_error] attribute can be used only with a proc-macro\n\n  hint: if you are really sure that #[proc_macro_error] should be applied to this exact function use #[proc_macro_error(allow_not_macro)]\n");
        )
        .into();
    }

    let ItemFn {
        attrs,
        vis,
        constness,
        asyncness,
        unsafety,
        abi,
        fn_token,
        ident,
        generics,
        inputs,
        output,
        block,
        ..
    } = input;

    let body = gen_body(block, assert_unwind_safe);

    quote!(
        #(#attrs),*
        #vis
        #constness
        #asyncness
        #unsafety
        #abi
        #fn_token
        #ident
        #generics
        (#inputs)
        #output

        { #body }
    )
    .into()
}

#[derive(PartialEq)]
enum Setting {
    AssertUnwindSafe,
    AllowNotMacro,
}

impl Parse for Setting {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match &*ident.to_string() {
            "assert_unwind_safe" => Ok(AssertUnwindSafe),
            "allow_not_macro" => Ok(AllowNotMacro),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown setting `{}`, expected one of `assert_unwind_safe`, `allow_not_macro`",
                    ident
                ),
            )),
        }
    }
}

struct Settings(Vec<Setting>);
impl Parse for Settings {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let punct = Punctuated::<Setting, Token![,]>::parse_terminated(input)?;
        Ok(Settings(Vec::from_iter(punct)))
    }
}

impl Settings {
    fn is_set(&self, setting: Setting) -> bool {
        self.0.iter().find(|s| **s == setting).is_some()
    }
}

#[rustversion::since(1.37)]
fn gen_body(block: Block, assert_unwind_safe: bool) -> proc_macro2::TokenStream {
    if assert_unwind_safe {
        quote!( ::proc_macro_error::entry_point(::std::panic::AssertUnwindSafe(|| #block )) )
    } else {
        quote!( ::proc_macro_error::entry_point(|| #block ) )
    }
}

#[rustversion::before(1.37)]
fn gen_body(block: Block, _assert_unwind_safe: bool) -> proc_macro2::TokenStream {
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
