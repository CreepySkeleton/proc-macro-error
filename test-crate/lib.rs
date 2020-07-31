#[macro_use]
extern crate proc_macro_error;
extern crate proc_macro;

use proc_macro2::{Span, TokenStream};
use proc_macro_error::{
    abort, abort_call_site, diagnostic, emit_call_site_warning, emit_error, emit_warning,
    proc_macro_error, set_dummy, Diagnostic, Level, OptionExt, ResultExt, SpanRange,
};

use syn::{parse_macro_input, spanned::Spanned};

// Macros and Diagnostic

#[proc_macro]
#[proc_macro_error]
pub fn abort_from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    abort!(
        span,
        syn::Error::new(Span::call_site(), "abort!(span, from) test")
    )
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
pub fn abort_call_site_test(_: proc_macro::TokenStream) -> proc_macro::TokenStream {
    abort_call_site!("abort_call_site! test")
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
    emit_error!(
        spans.next().unwrap(),
        syn::Error::new(Span::call_site(), "emit!(span, from) test")
    );
    emit_error!(
        spans.next().unwrap(),
        "emit!(span, expr1, {}) test",
        "expr2"
    );
    emit_error!(spans.next().unwrap(), "emit!(span, single_expr) test");
    Diagnostic::spanned(
        spans.next().unwrap().into(),
        Level::Error,
        "Diagnostic::emit() test".into(),
    )
    .emit();

    emit_call_site_error!("emit_call_site_error!(expr) test");

    // NOOP on stable, just checking that the macros themselves compile.
    emit_warning!(spans.next().unwrap(), "emit_warning! test");
    emit_call_site_warning!("emit_call_site_warning! test");

    quote!().into()
}

// Notes

#[proc_macro]
#[proc_macro_error]
pub fn abort_notes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut spans = input.into_iter().map(|s| s.span());
    let span = spans.next().unwrap();
    let span2 = spans.next().unwrap();

    let some_note = Some("Some note");
    let none_note: Option<&'static str> = None;

    abort! {
        span, "This is {} error", "an";

        note = "simple note";
        help = "simple help";
        hint = "simple hint";
        yay = "simple yay";

        note = "format {}", "note";

        note =? some_note;
        note =? none_note;

        note = span2 => "spanned simple note";
        note = span2 => "spanned format {}", "note";
        note =? span2 => some_note;
        note =? span2 => none_note;
    }
}

#[proc_macro]
#[proc_macro_error]
pub fn emit_notes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut spans = input.into_iter().step_by(2).map(|s| s.span());
    let span = spans.next().unwrap();
    let span2 = spans.next().unwrap();

    let some_note = Some("Some note");
    let none_note: Option<&'static str> = None;

    abort! {
        span, "This is {} error", "an";

        note = "simple note";
        help = "simple help";
        hint = "simple hint";
        yay = "simple yay";

        note = "format {}", "note";

        note =? some_note;
        note =? none_note;

        note = span2 => "spanned simple note";
        note = span2 => "spanned format {}", "note";
        note =? span2 => some_note;
        note =? span2 => none_note;
    }
}

// Extension traits

#[proc_macro]
#[proc_macro_error]
pub fn option_ext(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let none: Option<Diagnostic> = None;
    none.expect_or_abort("Option::expect_or_abort() test");
    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn result_unwrap_or_abort(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    let err = Diagnostic::spanned(
        span.into(),
        Level::Error,
        "Result::unwrap_or_abort() test".to_string(),
    );
    let res: Result<(), _> = Err(err);
    res.unwrap_or_abort();
    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn result_expect_or_abort(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    let err = Diagnostic::spanned(
        span.into(),
        Level::Error,
        "Result::expect_or_abort() test".to_string(),
    );
    let res: Result<(), _> = Err(err);
    res.expect_or_abort("BOOM");
    quote!().into()
}

// Dummy

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
pub fn append_dummy(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let span = input.into_iter().next().unwrap().span();
    set_dummy(quote! {
        impl Default for NeedDefault
    });

    proc_macro_error::append_dummy(quote!({
        fn default() -> Self {
            NeedDefault::A
        }
    }));

    abort!(span, "append_dummy test")
}

// Panic

#[proc_macro]
#[proc_macro_error]
pub fn unrelated_panic(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    panic!("unrelated panic test")
}

// Success

#[proc_macro]
#[proc_macro_error]
pub fn ok(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream::from(input);
    quote!(fn #input() {}).into()
}

// Multiple tokens

#[proc_macro_attribute]
#[proc_macro_error]
pub fn multiple_tokens(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    abort!(input, "...");
}

#[proc_macro]
#[proc_macro_error]
pub fn to_tokens_span(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ty = parse_macro_input!(input as syn::Type);
    emit_error!(ty, "whole type");
    emit_error!(ty.span(), "explicit .span()");
    quote!().into()
}

#[proc_macro]
#[proc_macro_error]
pub fn explicit_span_range(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut spans = input.into_iter().step_by(2).map(|s| s.span());
    let first = Span::from(spans.next().unwrap());
    let last = Span::from(spans.nth(1).unwrap());
    abort!(SpanRange { first, last }, "explicit SpanRange")
}

// Children messages

#[proc_macro]
#[proc_macro_error]
pub fn children_messages(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut spans = input.into_iter().step_by(2).map(|s| s.span());
    diagnostic!(spans.next().unwrap(), Level::Error, "main macro message")
        .span_error(spans.next().unwrap().into(), "child message".into())
        .emit();

    let mut main = syn::Error::new(spans.next().unwrap().into(), "main syn::Error");
    let child = syn::Error::new(spans.next().unwrap().into(), "child syn::Error");
    main.combine(child);
    Diagnostic::from(main).abort()
}
