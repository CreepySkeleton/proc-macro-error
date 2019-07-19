extern crate proc_macro;
extern crate proc_macro2;
extern crate proc_macro_error;
extern crate quote;
extern crate syn;

use proc_macro2::Span;
use proc_macro_error::*;
use quote::quote;
use syn::token::*;
use syn::{
    punctuated::Punctuated,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    parse_macro_input, Ident, Result, Token,
};


struct IdentOrUnderscore {
    span: Span,
    part: String
}

impl IdentOrUnderscore {
    fn new(span: Span, part: String) -> Self {
        IdentOrUnderscore { span, part }
    }
}


impl Parse for IdentOrUnderscore {
    fn parse(input: ParseStream) -> Result<Self> {
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
    fn parse(input: ParseStream) -> Result<Self> {
        let args = Punctuated::<_, Token![,]>::parse_terminated(input)?;
        Ok(Args(args.into_iter().collect()))
    }
}


#[proc_macro]
pub fn make_fn(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    filter_macro_errors! {
        let mut name = String::new();

        let input = parse_macro_input!(input as Args);

        for arg in input.0 {
            match &*arg.part {
                "span_many" => span_error!(arg.span, "span_error! 3{} args {}", "+", "test"),
                "span_two" => span_error!(arg.span, "span_error! 2 args test"),
                "span_single" => {
                    let e = MacroError::new(arg.span, "span_error! single-arg test".into());
                    span_error!(e)
                },

                "call_site_many" => call_site_error!("call_site_error! 2{} args {}", "+", "test"),
                "call_site_single" => call_site_error!("call_site_error! single-arg test"),

                "trigger" => trigger_error("direct triger_error() test"),

                "result_expect" => {
                    let e = syn::Error::new(arg.span, "error");
                    Err(e).expect_or_exit("Result::expect_or_exit() test")
                },

                "result_unwrap" => {
                    let e = syn::Error::new(arg.span, "Result::unwrap_or_exit() test");
                    Err(e).unwrap_or_exit()
                },

                "option_expect" => {
                    None.expect_or_exit("Option::expect_or_exit() test")
                }

                _ => name.push_str(&arg.part),
            }
        }

        // test that all the panics from another source are not to be caught
        if name.is_empty() {
            panic!("empty name")
        }

        let name = Ident::new(&name, Span::call_site());
        quote!( fn #name() {} ).into()
    }
}
