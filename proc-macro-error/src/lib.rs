//! # proc-macro-error
//!
//! This crate aims to make error reporting in proc-macros simple and easy to use.
//! Migrate from `panic!`-based errors for as little effort as possible!
//!
//! Also, there's ability to [append a dummy token stream][dummy] to your errors.
//!
//! ## Limitations
//!
//! - Warnings get emitted only on nightly, they're ignored on stable.
//! - "help" suggestions cannot have their own span info on stable.
//! - If a panic occurs somewhere in your macro no errors will be displayed.
//!
//! ## Guide
//!
//! ### Table of contents
//!
//! ### Introduction
//!
//!

// reexports for use in macros
pub extern crate proc_macro;
pub extern crate proc_macro2;

pub mod dummy;
pub mod multi;
pub mod single;

pub use self::dummy::set_dummy;
pub use self::multi::abort_if_dirty;
pub use self::single::Diagnostic;
pub use proc_macro_error_attr::proc_macro_error;

use quote::quote;

use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};

/// This traits expands [`Result<T, Into<Diagnostic>>`](Diagnostic) with some handy shortcuts.
pub trait ResultExt {
    type Ok;

    /// Behaves like [`Result::unwrap`]: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via [`abort!`].
    fn unwrap_or_abort(self) -> Self::Ok;

    /// Behaves like [`Result::expect`]: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via [`abort!`].
    /// If it aborts then resulting error message will be preceded with `message`.
    fn expect_or_abort(self, msg: &str) -> Self::Ok;
}

/// This traits expands [`Option<T>`][std::option::Option] with some handy shortcuts.
pub trait OptionExt {
    type Some;

    /// Behaves like [`Option::expect`]: if self is `Some` yield the contained value,
    /// otherwise abort macro execution via [`abort_call_site!`].
    /// If it aborts the `message` will be used for [`compile_error!`][compl_err] invocation.
    ///
    /// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
    fn expect_or_abort(self, msg: &str) -> Self::Some;
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Some(res) => res,
            None => abort_call_site!(message),
        }
    }
}

/// This is the entry point for your proc-macro.
///
/// Typically, you use `#[proc_macro_error]` instead, see [module level docs][self].
pub fn entry_point<F>(f: F) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
{
    ENTERED_ENTRY_POINT.with(|flag| flag.store(true, Ordering::SeqCst));
    let caught = catch_unwind(f);
    let dummy = dummy::cleanup();
    let err_storage = multi::cleanup();
    ENTERED_ENTRY_POINT.with(|flag| flag.store(false, Ordering::SeqCst));

    match caught {
        Ok(ts) => {
            if err_storage.is_empty() {
                ts
            } else {
                quote!( #(#err_storage)* #dummy ).into()
            }
        }

        Err(boxed) => match boxed.downcast::<AbortNow>() {
            Ok(_) => {
                assert!(!err_storage.is_empty());
                quote!( #(#err_storage)* #dummy ).into()
            }
            Err(boxed) => resume_unwind(boxed),
        },
    }
}

thread_local! {
    static ENTERED_ENTRY_POINT: AtomicBool = AtomicBool::new(false);
}

struct AbortNow;

fn check_correctness() {
    if !ENTERED_ENTRY_POINT.with(|flag| flag.load(Ordering::SeqCst)) {
        panic!("proc-macro-error API cannot be used outside of `entry_point` invocation. Perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]");
    }
}
