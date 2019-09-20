//! Facility to stack and emit multiple errors.
//!
//!

use crate::{MacroError, AbortNow, check_correctness};

use std::cell::RefCell;

thread_local! {
    static ERR_STORAGE: RefCell<Vec<MacroError>> = RefCell::new(Vec::new());
}

#[macro_export]
macro_rules! emit_error {
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
macro_rules! emit_call_site_error {
    ($fmt:literal, $($args:expr),*) => {{
        use $crate::push_span_error;

        let span = $crate::proc_macro2::Span::call_site();
        push_span_error!(span, $fmt, $($args),*)
    }};

    ($msg:expr) => {{
        use $crate::emit_error;

        let span = $crate::proc_macro2::Span::call_site();
        emit_error!(span, $msg)
    }};
}

/// Clear the global error storage, returning the errors contained.
///
/// # Warning:
///
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
pub fn abort_if_dirty() {
    check_correctness();
    ERR_STORAGE.with(|storage| {
        if !storage.borrow().is_empty() {
            panic!(AbortNow)
        }
    });
}

/// Push the error into the global error storage.
///
/// Users are not supposed to use this function directly, use
/// [`push_span_error!`] instead.
#[doc(hidden)]
pub fn push_error(error: MacroError) {
    check_correctness();
    ERR_STORAGE.with(|storage| storage.borrow_mut().push(error))
}
