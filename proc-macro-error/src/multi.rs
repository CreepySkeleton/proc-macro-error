//! Facility to stack and emit multiple errors.
//!
//!
use crate::{MacroError, Payload};

use std::cell::RefCell;

use proc_macro2::TokenStream;
use quote::ToTokens;

thread_local! {
    static ERR_STORAGE: RefCell<Vec<MacroError>> = RefCell::new(Vec::new());
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

/// Clear the global error storage, returning the errors contained.
///
/// # Warning:
/// You **must** do it before macro execution completes
/// ([`filter_macro_errors!`] does it for you)! If the storage
/// is dirty at the end moment of macro execution `rustc` will fail with cryptic
///
/// ```text
/// thread 'rustc' panicked at 'use-after-free in `proc_macro` handle', src\libcore\option.rs:1166:5
/// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace.
/// ```
pub fn cleanup() -> Vec<MacroError> {
    ERR_STORAGE.with(|storage| storage.replace(Vec::new()))
}

/// Abort macro execution and show errors if global error storage is not empty.
pub fn trigger_if_dirty() {
    ERR_STORAGE.with(|storage| {
        if !storage.borrow().is_empty() {
            let errs = storage.replace(Vec::new());
            panic!(Payload(errs))
        }
    });
}

/// Push the error into the global error storage.
///
/// Users are not supposed to use this function directly, use
/// [`push_span_error!`] instead.
#[doc(hidden)]
pub fn push_error(error: MacroError) {
    ERR_STORAGE.with(|storage| storage.borrow_mut().push(error))
}

/// Exists because I can't (and shouldn't) implement
/// `ToTokens` for `Vec<MacroError>`
pub(crate) struct MultiMacroErrors(Vec<MacroError>);

impl ToTokens for MultiMacroErrors {
    fn to_tokens(&self, ts: &mut TokenStream) {
        for err in self.0.iter() {
            err.to_tokens(ts);
        }
    }
}
