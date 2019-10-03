#[macro_use]
extern crate proc_macro_error;
#[macro_use]
extern crate syn;
extern crate proc_macro;

use std::panic::AssertUnwindSafe;
use proc_macro2::Span;
use syn::{
    Ident,
    parse::{Parse, ParseStream},
    token::Underscore,
    punctuated::Punctuated,
    spanned::Spanned
};
use proc_macro_error::{
    entry_point,
    set_dummy,
    ResultExt,
    OptionExt,
};


struct IdentOrUnderscore {
    span: Span,
    part: String,
}

impl IdentOrUnderscore {
    fn new(span: Span, part: String) -> Self {
        IdentOrUnderscore { span, part }
    }
}

impl Parse for IdentOrUnderscore {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let la = input.lookahead1();

        if la.peek(Ident) {
            let t = input.parse::<Ident>().unwrap();
            Ok(IdentOrUnderscore::new(t.span(), t.to_string()))
        } else if la.peek(Underscore) {
            let t = input.parse::<Underscore>().unwrap();
            Ok(IdentOrUnderscore::new(t.span(), "_".to_string()))
        } else {
            Err(la.error())
        }
    }
}

struct Args(Vec<IdentOrUnderscore>);

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<_, Token![,]>::parse_terminated(input)?;
        Ok(Args(args.into_iter().collect()))
    }
}

#[proc_macro]
pub fn make_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // FIXME: proc_macro::TokenStream does not implement UnwindSafe until 1.37.0
    entry_point(AssertUnwindSafe(|| {
        let mut name = String::new();

        let input = parse_macro_input!(input as Args);

        for arg in input.0 {
            match &*arg.part {
                "abort" => abort!(arg.span, "abort! 3{} args {}", "+", "test"),

                "abort_call_site" => abort_call_site!("abort_call_site! 2{} args {}", "+", "test"),

                "direct_abort" =>
                    macro_error!(arg.span, "direct MacroError::abort() test").abort(),

                "result_expect" => {
                    let e = syn::Error::new(arg.span, "error");
                    Err(e).expect_or_abort("Result::expect_or_abort() test")
                },

                "result_unwrap" => {
                    let e = syn::Error::new(arg.span, "Result::unwrap_or_abort() test");
                    Err(e).unwrap_or_abort()
                },

                "option_expect" => {
                    None.expect_or_abort("Option::expect_or_abort() test")
                },

                "need_default" => {
                    set_dummy(quote! {
                        impl Default for NeedDefault {
                            fn default() -> Self {
                                NeedDefault::A
                            }
                        }
                    });

                    abort!(arg.span, "set_dummy test")
                },

                part if part.starts_with("multi") =>
                    emit_error!(arg.span, "multiple error part: {}", part),

                _ => name.push_str(&arg.part),
            }
        }

        // test that all the panics from another source are not to be caught
        if name.is_empty() {
            panic!("unrelated panic test")
        }

        let name = Ident::new(&name, Span::call_site());
        quote!( fn #name() {} ).into()
    }))
}
