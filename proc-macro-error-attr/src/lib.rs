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
/// - `proc_macro_hack`:
///
///     To correctly cooperate with `#[proc_macro_hack]` `#[proc_macro_error]`
///     attribute must be placed *before* (above) it, like this:
///
///     ```ignore
///     #[proc_macro_error]
///     #[proc_macro_hack]
///     #[proc_macro]
///     fn my_macro(input: TokenStream) -> TokenStream {
///         unimplemented!()
///     }
///     ```
///
///     If, for some reason, you can't place it like that you can use
///     `#[proc_macro_error(proc_macro_hack)]` instead.
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
///     By default, your code must be [unwind safe]. If your code is not unwind safe but you believe
///     it's correct you can use this setting to bypass the check. This is typically needed
///     for code that uses `lazy_static` or `thread_local` with `Call/RefCell` inside.
///
///     This setting is implied if `#[proc_macro_error]` is placed on top of a function
///     marked as `#[proc_macro]`, `#[proc_macro_derive]` or `#[proc_macro_attribute]`.
///
/// [ep]: https://docs.rs/proc-macro-error/0.3/proc_macro_error/fn.entry_point.html
/// [unwind safe]: https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html#what-is-unwind-safety
///
#[proc_macro_attribute]
pub fn proc_macro_error(attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let mut settings = match syn::parse::<Settings>(attr) {
        Ok(settings) => settings,
        Err(err) => {
            let err = err.to_compile_error();
            return quote!(#input #err).into();
        }
    };

    let is_proc_macro = is_proc_macro(&input.attrs);
    if is_proc_macro {
        settings.set(AssertUnwindSafe);
    }

    if detect_proc_macro_hack(&input.attrs) {
        settings.set(ProcMacroHack);
    }

    if !(settings.is_set(AllowNotMacro) || is_proc_macro) {
        return quote!(
            #input
            compile_error!("#[proc_macro_error] attribute can be used only with a proc-macro\n\n  \
                hint: if you are really sure that #[proc_macro_error] should be applied \
                to this exact function use #[proc_macro_error(allow_not_macro)]\n");
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

    let body = gen_body(block, settings);

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
    ProcMacroHack,
}

impl Parse for Setting {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match &*ident.to_string() {
            "assert_unwind_safe" => Ok(AssertUnwindSafe),
            "allow_not_macro" => Ok(AllowNotMacro),
            "proc_macro_hack" => Ok(ProcMacroHack),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown setting `{}`, expected one of \
                     `assert_unwind_safe`, `allow_not_macro`, `proc_macro_hack`",
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

    fn set(&mut self, setting: Setting) {
        self.0.push(setting)
    }
}

#[rustversion::since(1.37)]
fn gen_body(block: Block, settings: Settings) -> proc_macro2::TokenStream {
    let is_proc_macro_hack = settings.is_set(ProcMacroHack);
    let closure = if settings.is_set(AssertUnwindSafe) {
        quote!(::std::panic::AssertUnwindSafe(|| #block ))
    } else {
        quote!(|| #block)
    };

    quote!( ::proc_macro_error::entry_point(#closure, #is_proc_macro_hack) )
}

// FIXME:
// proc_macro::TokenStream does not implement UnwindSafe until 1.37.0.
// Considering this is the closure's return type the unwind safety check would fail
// for virtually every closure possible, the check is meaningless.
#[rustversion::before(1.37)]
fn gen_body(block: Block, settings: Settings) -> proc_macro2::TokenStream {
    let is_proc_macro_hack = settings.is_set(ProcMacroHack);
    let closure = quote!(::std::panic::AssertUnwindSafe(|| #block ));
    quote!( ::proc_macro_error::entry_point(#closure, #is_proc_macro_hack) )
}

fn detect_proc_macro_hack(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|attr| attr.path.is_ident("proc_macro_hack"))
}

fn is_proc_macro(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path.is_ident("proc_macro")
            || attr.path.is_ident("proc_macro_derive")
            || attr.path.is_ident("proc_macro_attribute")
    })
}
