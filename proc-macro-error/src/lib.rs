//! # proc-macro-error
//!
//! This crate aims to make error reporting in proc-macros simple and easy to use.
//! Migrate from `panic!`-based errors for as little effort as possible!
//!
//! Also, there's [ability to append a dummy token stream][dummy] to your errors.
//!
//! ## Enticement
//!
//! Your errors look like
//! ```text
//!
//! ### Singe error usage
//! ```rust,ignore
//! extern crate proc_macro_error;
//! use proc_macro_error::{
//!     filter_macro_errors,
//!     span_error,
//!     call_site_error,
//!     ResultExt,
//!     OptionExt
//! };
//!
//! // This is your main entry point
//! #[proc_macro_]
//! pub fn make_answer(input: TokenStream) -> TokenStream {
//!     // This macro **must** be placed at the top level.
//!     // No need to touch the code inside though.
//!     filter_macro_errors! {
//!         // `parse_macro_input!` and friends work just fine inside this macro
//!         let input = parse_macro_input!(input as MyParser);
//!
//!         if let Err(err) = some_logic(&input) {
//!             // we've got a span to blame, let's use it
//!             let span = err.span_should_be_highlighted();
//!             let msg = err.message();
//!             // This call jumps directly to the end of `filter_macro_errors!` invocation
//!             span_error!(span, "You made an error, go fix it: {}", msg);
//!         }
//!
//!         // `Result` gets some handy shortcuts if your error type implements
//!         // `Into<``MacroError``>`. `Option` has one unconditionally
//!         use proc_macro_error::ResultExt;
//!         more_logic(&input).expect_or_exit("What a careless user, behave!");
//!
//!         if !more_logic_for_logic_god!(&input) {
//!             // We don't have an exact location this time,
//!             // so just highlight the proc-macro invocation itself
//!             call_site_error!(
//!                 "Bad, bad user! Now go stand in the corner and think about what you did!");
//!         }
//!
//!         // Now all the processing is done, return `proc_macro::TokenStream`
//!         quote!(/* stuff */).into()
//!     }
//!
//!     // At this point we have a new shining `proc_macro::TokenStream`!
//! }
//! ```
//!
//!
//! ## Motivation and Getting started
//!
//! Error handling in proc-macros sucks. It's not much of a choice today:
//! you either "bubble up" the error up to the top-level of your macro and convert it to
//! a [`compile_error!`][compl_err] invocation or just use a good old panic. Both these ways suck:
//!
//! - Former sucks because it's quite redundant to unroll a proper error handling
//!     just for critical errors that will crash the macro anyway so people mostly
//!     choose not to bother with it at all and use panic. Almost nobody does it,
//!     simple `.expect` is too tempting.
//! - Later sucks because there's no way to carry out span info via `panic!`. `rustc` will highlight
//!     the whole invocation itself but not some specific token inside it.
//!     Furthermore, panics aren't for error-reporting at all; panics are for bug-detecting
//!     (like unwrapping on `None` or out-of range indexing) or for early development stages
//!     when you need a prototype ASAP and error handling can wait. Mixing these usages only
//!     messes things up.
//! - There is [`proc_macro::Diagnostics`](https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html)
//!     but it's experimental. (This crate will be deprecated once `Diagnostics` is stable.)
//!
//! That said, we need a solution, but this solution must meet these conditions:
//!
//! - It must be better than `panic!`. The main point: it must offer a way to carry span information
//!     over to user.
//! - It must require as little effort as possible to migrate from `panic!`. Ideally, a new
//!     macro with the same semantics plus ability to carry out span info.
//! - It must be usable on stable.
//!
//! This crate aims to provide such a mechanism. All you have to do is enclose all
//! the code inside your top-level `#[proc_macro]` function in [`filter_macro_errors!`]
//! invocation and change panics to [`span_error!`]/[`call_site_error!`] where appropriate,
//! see [Usage](#usage)
//!
//! # How it works
//! Effectively, it emulates try-catch mechanism on the top of panics.
//!
//! Essentially, the [`filter_macro_errors!`] macro is (C++ like pseudo-code)
//!
//! ```C++
//! try {
//!     /* your code */
//! } catch (MacroError) {
//!     /* conversion to compile_error! */
//! } catch (MultiMacroErrors) {
//!     /* conversion to multiple compile_error! invocations */
//! }
//! ```
//!
//! [`span_error!`] and co are
//!
//! ```C++
//! throw MacroError::new(span, format!(msg...));
//! ```
//!
//! By calling [`span_error!`] you trigger panic that will be caught by [`filter_macro_errors!`]
//! and converted to [`compile_error!`][compl_err] invocation.
//! All the panics that weren't triggered by [`span_error!`] and co will be resumed as is.
//!
//! Panic catching is indeed *slow* but the macro is about to abort anyway so speed is not
//! a concern here. Please note that **this crate is not intended to be used in any other way
//! than a proc-macro error reporting**, use `Result` and `?` instead.
//!
//! [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
//! [`proc_macro::Diagnostics`](https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html)

// reexports for use in macros
pub extern crate proc_macro;
pub extern crate proc_macro2;

pub mod dummy;
pub mod multi;
pub mod single;

pub use self::dummy::set_dummy;
pub use self::single::MacroError;
pub use self::multi::abort_if_dirty;
pub use proc_macro_error_attr::proc_macro_error;

use quote::{quote};

use std::panic::{catch_unwind, resume_unwind, UnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};


/// This traits expands [`Result<T, Into<MacroError>>`](std::result::Result) with some handy shortcuts.
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

/// This macro **MUST** be placed at the top level of your procedural macro,
/// inside the function marked with a `#[proc_macro*]` attribute. All the
/// error emitting related API can be used only inside this macro invocation.
///
/// See the [module-level documentation](self) for usage example
pub fn entry_point<F>(f: F) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream,
    F: UnwindSafe
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
            },
            Err(boxed) => resume_unwind(boxed),
        }
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
