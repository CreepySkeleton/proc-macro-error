//! Facility for stacking and emitting multiple errors.
//!
//! [`abort!`] macro stops a proc-macro *right away*, much like in a panic-like
//! fashion. But sometimes you *do not* want to stop right there, for example you're
//! processing a list of attributes and want to *emit* a separate error for every
//! mis-built attribute.
//!
//! The [`emit_error!`] and [`emit_call_site_error!`] macros are just for it!

use crate::{check_correctness, single::Diagnostic, AbortNow};

use std::cell::RefCell;

thread_local! {
    static ERR_STORAGE: RefCell<Vec<Diagnostic>> = RefCell::new(Vec::new());
}

/// Emit an error while not aborting the proc-macro right away.
///
/// The emitted errors will be converted to a `TokenStream` sequence
/// of `compile_error!` invocations after the execution hits the end
/// of the function marked with `[proc_macro_error]` or the lambda passed to [`entry_point`].
///
/// # Syntax
///
/// The same as [`abort!`].
///
/// # Note:
/// If a panic occurs somewhere in your macro no errors will be shown.
#[macro_export]
macro_rules! emit_error {
    ($($tts:tt)*) => {{
        $crate::diagnostic!($($tts)*).emit()
    }};
}

/// Shortcut for `emit_error!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! emit_call_site_error {
    ($($tts:tt)*) => {{
        let span = $crate::proc_macro2::Span::call_site();
        $crate::diagnostic!(span, $($tts)*).emit()
    }};
}

/// Abort macro execution and display all the emitted errors, if any.
///
/// Does nothing if no errors were emitted.
pub fn abort_if_dirty() {
    check_correctness();
    ERR_STORAGE.with(|storage| {
        if !storage.borrow().is_empty() {
            abort_now()
        }
    });
}

/// Clear the global error storage, returning the errors contained.
pub(crate) fn cleanup() -> Vec<Diagnostic> {
    ERR_STORAGE.with(|storage| storage.replace(Vec::new()))
}

/// Abort right now.
pub(crate) fn abort_now() -> ! {
    check_correctness();
    panic!(AbortNow)
}

/// Push the error into the global error storage.
pub(crate) fn push_error(error: Diagnostic) {
    check_correctness();
    ERR_STORAGE.with(|storage| storage.borrow_mut().push(error))
}
