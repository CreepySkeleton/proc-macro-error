use quote::ToTokens;

use std::cell::RefCell;

use crate::{MacroError, Payload};
use proc_macro2::TokenStream;

thread_local! {
    static ERR_STORAGE: RefCell<Vec<MacroError>> = RefCell::new(Vec::new());
}

pub(crate) struct MultiMacroErrors(Vec<MacroError>);

impl ToTokens for MultiMacroErrors {
    fn to_tokens(&self, ts: &mut TokenStream) {
        for err in self.0.iter() {
            err.to_tokens(ts);
        }
    }
}

pub fn push_error(error: MacroError) {
    ERR_STORAGE.with(|storage| storage.borrow_mut().push(error))
}

pub fn cleanup() -> Vec<MacroError> {
    ERR_STORAGE.with(|storage| storage.replace(Vec::new()))
}

pub fn trigger_on_error() {
    ERR_STORAGE.with(|storage| {
        if !storage.borrow().is_empty() {
            let errs = storage.replace(Vec::new());
            panic!(Payload(errs))
        }
    });
}

#[macro_export]
macro_rules! push_span_error {
    ($span:expr, $fmt:literal, $($args:expr),*) => {{
        let msg = format!($fmt, $($args),*);
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        let err = $crate::MacroError::new($span.into(), msg);
        $crate::multi::push_error(err);
    }};

    ($span:expr, $msg:expr) => {{
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        let err = $crate::MacroError::new($span.into(), $msg.to_string());
        $crate::multi::push_error(err);
    }};

    ($err:expr) => {{
        let err = $crate::MacroError::from($err);
        $crate::multi::push_error(err);
    }};
}

/// Shortcut for `span_error!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation-and-getting-started)
#[macro_export]
macro_rules! push_call_site_error {
    ($fmt:literal, $($args:expr),*) => {{
        use $crate::push_span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $fmt, $($args),*)
    }};

    ($msg:expr) => {{
        use $crate::push_span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $fmt, $($args),*)
    }};
}
