#[macro_use]
extern crate proc_macro_error;
extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use proc_macro_error::{set_dummy, Diagnostic, Level, OptionExt, ResultExt};


#[proc_macro]
#[proc_macro_error]
pub fn abort_from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    abort!(span, syn::Error::new(Span::call_site(), "abort!(span, from) test"))
}

#[proc_macro]
#[proc_macro_error]
pub fn abort_to_string(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    abort!(span, "abort!(span, single_expr) test")
}

#[proc_macro]
#[proc_macro_error]
pub fn abort_format(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    abort!(span, "abort!(span, expr1, {}) test", "expr2")
}

#[proc_macro]
#[proc_macro_error]
pub fn direct_abort(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    Diagnostic::spanned(span.into(), Level::Error, "Diagnostic::abort() test".into()).abort()
}

#[proc_macro]
#[proc_macro_error]
pub fn emit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut spans = input.into_iter().step_by(2).map(|t| t.span());
    emit_error!(spans.next().unwrap(), syn::Error::new(Span::call_site(), "emit!(span, from) test"));
    emit_error!(spans.next().unwrap(), "emit!(span, expr1, {}) test", "expr2");
    emit_error!(spans.next().unwrap(), "emit!(span, single_expr) test");
    Diagnostic::spanned(
        spans.next().unwrap().into(),
        Level::Error,
        "Diagnostic::emit() test".into()
    ).emit();

    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn option_ext(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    Option::<syn::Error>::None.expect_or_abort("Option::expect_or_abort() test");
    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn result_unwrap_or_abort(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    let err = syn::Error::new(span.into(), "Result::unwrap_or_abort() test");
    Result::<(), _>::Err(err).unwrap_or_abort();
    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn result_expect_or_abort(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    let err = syn::Error::new(span.into(), "Result::expect_or_abort() test");
    Result::<(), _>::Err(err).expect_or_abort("BOOM");
    quote!().into()
}


#[proc_macro]
#[proc_macro_error]
pub fn dummy(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    set_dummy(quote! {
        impl Default for NeedDefault {
            fn default() -> Self { NeedDefault::A }
        }
    });

    abort!(span, "set_dummy test")
}

#[proc_macro]
#[proc_macro_error]
pub fn unrelated_panic(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    panic!("unrelated panic test")
}


#[proc_macro]
#[proc_macro_error]
pub fn ok(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    quote!(fn #input() {}).into()
}