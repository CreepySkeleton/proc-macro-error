//! Facility to stack and emit multiple errors.
//!
//! [`abort!`] macro stops a proc-macro *right away*, much like in a panic-like
//! fashion. But sometimes you *do not* want to stop right there, for example you're
//! processing a list of attributes and want to *emit* a separate error for every
//! mis-built attribute.
//!
//! The [`emit_error!`] and [`emit_call_site_error!`] macros are just for it!

use crate::{MacroError, AbortNow, check_correctness};

use std::cell::RefCell;

thread_local! {
    static ERR_STORAGE: RefCell<Vec<MacroError>> = RefCell::new(Vec::new());
}

/// Emit an error while not aborting the proc-macro right away.
///
/// The emitted errors will be converted to a `TokenStream` sequence
/// of `compile_error!` invocations after the execution hits the end
/// of the function marked with `[proc_macro_error]` or the lambda passed to [`entry_point`].
///
/// # Note:
/// If a panic occurs somewhere in your macro no errors will be shown.
#[macro_export]
macro_rules! emit_error {
    ($span:expr, $fmt:expr, $($args:expr),*) => {{
        use $crate::macro_error;

        let err = macro_error!($span, $fmt, $($args),*);
        $crate::multi::push_error(err);
    }};

    ($span:expr, $msg:expr) => {{
        let err = $crate::MacroError::new($span.into(), $msg.to_string());
        $crate::multi::push_error(err);
    }};

    ($err:expr) => {{
        let err = $crate::MacroError::from($err);
        $crate::multi::push_error(err);
    }};
}

/// Shortcut for `emit_error!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! emit_call_site_error {
    ($fmt:expr, $($args:expr),*) => {{
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

/// Abort macro execution and display all the emitted errors, if any.
///
/// Does nothing if no errors were emitted.
pub fn abort_if_dirty() {
    check_correctness();
    ERR_STORAGE.with(|storage| {
        if !storage.borrow().is_empty() {
            panic!(AbortNow)
        }
    });
}

/// Clear the global error storage, returning the errors contained.
pub(crate) fn cleanup() -> Vec<MacroError> {
    ERR_STORAGE.with(|storage| storage.replace(Vec::new()))
}

/// Push the error into the global error storage.
///
/// **Not public API.**
#[doc(hidden)]
pub fn push_error(error: MacroError) {
    check_correctness();
    ERR_STORAGE.with(|storage| storage.borrow_mut().push(error))
}
