
//! # proc-macro-error
//!
//! This crate aims to provide an error reporting mechanism that is usable inside
//! `proc-macros`, can highlight a specific span, and can be migrated from
//! `panic!`-based errors with minimal efforts.
//!
//! ## Usage
//!
//! In your `Cargo.toml`:
//!
//! ```toml
//! proc-macro-error = "0.1"
//! ```
//!
//! In `lib.rs`:
//!
//!
//! ```rust, ignore
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
//! #[proc_macro]
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
//! you either "bubble up" the error up to top-level of you macro and convert it to
//! a [`compile_error!`][compl_err] invocation or just use a good old panic. Both these ways suck:
//!
//! - Former sucks because it's quite redundant to unroll a proper error handling
//!     just for critical errors that will crash the macro anyway so people mostly
//!     choose not to bother with it at all and use panic. Almost nobody does it,
//!     simple `.expect` is too tempting.
//! - Later sucks because there's no way to carry out span info via panic. `rustc` will highlight
//!     the whole invocation itself but not some specific token inside it.
//!     Furthermore, panics aren't for error-reporting at all; panics are for bug-detecting
//!     (like unwrapping on `None` or out-of range indexing) or for early development stages
//!     when you need a prototype ASAP and error handling can wait. Mixing these usages only
//!     messes things up.
//! - There is [`proc_macro::Diagnostics`](https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html)
//!     but it's experimental.
//!
//! That said, we need a solution, but this solution must meet these conditions:
//!
//! - It must be better than panics. The main point: it must offer a way to carry span information
//!     over to user.
//! - It must require as little effort as possible to migrate from panic. Ideally, a new
//!     macro with the same semantics plus ability to carry out span info.
//! - It must be usable on stable.
//!
//! This crate aims to provide such a mechanism. All you have to do is enclose all
//! the code inside your top-level `#[proc_macro]` function in [`filter_macro_errors!`]
//! invocation and change panics to [`span_error!`]/[`call_site_error!`] where appropriate,
//! see [Usage](#usage)
//!
//! # How it works
//! Effectively, it emulates try-catch mechanism on top of panics.
//!
//! Essentially, the [`filter_macro_errors!`] macro is a
//! ```C++
//! try {
//!     /* your code */
//! } catch (MacroError) {
//!     /* conversion to compile_error! */
//! }
//! ```
//!
//! [`span_error!`] and co are
//! ```C++
//! throw MacroError::new(span, format!(msg...));
//! ```
//!
//! By calling [`span_error!`] you trigger panic that will be caught by [`filter_macro_errors!`]
//! and converted to [`compile_error!`][compl_err] invocation.
//! All the panics that wasn't triggered by [`span_error!`] and co but any other reason
//! will be resumed as is.
//!
//! Panic catching is indeed *slow* but the macro is about to abort anyway so speed is not
//! a concern here. Please note that this crate is not intended to be used in any other way
//! than a proc-macro error reporting, use `Result` and `?` instead.
//!
//!
//! [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
//!

// reexports for use in macros
pub extern crate proc_macro;
pub extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;

/// Makes a [`MacroError`] instance from provided arguments (`panic!`-like)
/// and triggers panic in hope it will be caught by [`filter_macro_errors!`].
///
/// # Syntax
///
/// This macro is meant to be a `panic!` drop-in replacement so its syntax is very similar to `panic!`,
/// but it has three forms instead of two:
/// 1. "panic-format-like" form: span, formatting [`str`] literal, comma-separated list of args.
///     First argument is a span, all the rest gets passed to [`format!`] to build the error message.
/// 2. "panic-single-arg-like" form: span, expr, no comma at the end.
///     First argument is a span, the second is our error message, it must implement [`ToString`].
/// 3. "trigger_error-like" form: single expr.
///     Literally `trigger_error(arg)`. It's here just for convenience so [`span_error!`] can be used
///     with instances of [`syn::Error`], [`MacroError`], [`&str`], [`String`] and so on...
///
#[macro_export]
macro_rules! span_error {
    ($span:expr, $fmt:literal, $($args:expr),*) => {{
        let msg = format!($fmt, $($args),*);
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        let err = $crate::MacroError::new($span.into(), msg);
        $crate::trigger_error(err)
    }};

    ($span:expr, $msg:expr) => {{
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        let err = $crate::MacroError::new($span.into(), $msg.to_string());
        $crate::trigger_error(err)
    }};

    ($err:expr) => { $crate::trigger_error($err) };
}

/// Shortcut for `span_error!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! call_site_error {
    ($fmt:literal, $($args:expr),*) => {{
        use $crate::span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $fmt, $($args),*)
    }};

    ($fmt:expr) => {{
        use $crate::span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $fmt)
    }};
}

/// This macro is supposed to be used at the top level of your `proc-macro`,
/// the function marked with a `#[proc_macro*]` attribute. It catches all the
/// errors triggered by [`span_error!`], [`call_site_error!`], and [`trigger_error`].
/// Once caught, it converts it to a [`proc_macro::TokenStream`]
/// containing a [`compile_error!`][compl_err] invocation.
///
/// See the [module-level documentation](self) for usage example
///
/// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
#[macro_export]
macro_rules! filter_macro_errors {
    ($($code:tt)*) => {
        let f = move || -> $crate::proc_macro::TokenStream { $($code)* };
        $crate::filter_macro_error_panics(f)
    };
}

/// An error in a proc-macro. This struct preserves
/// the given span so `rustc` can highlight the exact place in user code
/// responsible for the error.
///
/// You're not supposed to use this type directly, use [`span_error!`] and [`call_site_error!`].
#[derive(Debug)]
pub struct MacroError {
    span: Span,
    msg: String,
}

impl MacroError {
    /// Create an error with the span and message provided.
    pub fn new(span: Span, msg: String) -> Self {
        MacroError { span, msg }
    }

    /// A shortcut for `MacroError::new(Span::call_site(), message)`
    pub fn call_site(msg: String) -> Self {
        MacroError::new(Span::call_site(), msg)
    }

    /// Convert this error into a [`TokenStream`] containing these tokens: `compiler_error!(<message>);`.
    /// All these tokens carry the span this error contains attached.
    ///
    /// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
    pub fn into_compile_error(self) -> TokenStream {
        let MacroError { span, msg } = self;
        quote_spanned! { span=> compile_error!(#msg); }
    }

    /// Abandon the old span and replace it with the given one.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}

/// This traits expands [`Result<T, Into<MacroError>>`](std::result::Result) with some handy shortcuts.
pub trait ResultExt {
    type Ok;

    /// Behaves like [`Result::unwrap`]: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via [`span_error!`].
    fn unwrap_or_exit(self) -> Self::Ok;

    /// Behaves like [`Result::expect`]: if self is `Ok` yield the contained value,
    /// otherwise abort macro execution via [`span_error!`].
    /// If it aborts then resulting message will be preceded with `message`.
    fn expect_or_exit(self, msg: &str) -> Self::Ok;
}

/// This traits expands [`Option<T>`][std::option::Option] with some handy shortcuts.
pub trait OptionExt {
    type Some;

    /// Behaves like [`Option::expect`]: if self is `Some` yield the contained value,
    /// otherwise abort macro execution via [`call_site_error!`].
    /// If it aborts the `message` will be used for [`compile_error!`][compl_err] invocation.
    ///
    /// [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
    fn expect_or_exit(self, msg: &str) -> Self::Some;
}

impl<T, E: Into<MacroError>> ResultExt for Result<T, E> {
    type Ok = T;

    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => trigger_error(e),
        }
    }

    fn expect_or_exit(self, message: &str) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                let MacroError { msg, span } = e.into();
                let msg = format!("{}: {}", message, msg);
                trigger_error(MacroError::new(span, msg))
            }
        }
    }
}

impl<T> OptionExt for Option<T> {
    type Some = T;

    fn expect_or_exit(self, message: &str) -> T {
        match self {
            Some(res) => res,
            None => call_site_error!(message),
        }
    }
}

impl From<syn::Error> for MacroError {
    fn from(e: syn::Error) -> Self {
        MacroError::new(e.span(), e.to_string())
    }
}

impl From<String> for MacroError {
    fn from(msg: String) -> Self {
        MacroError::call_site(msg)
    }
}

impl From<&str> for MacroError {
    fn from(msg: &str) -> Self {
        MacroError::call_site(msg.into())
    }
}

impl ToString for MacroError {
    fn to_string(&self) -> String {
        self.msg.clone()
    }
}

/// Trigger error, aborting the proc-macro's execution.
///
/// You're not supposed to use this function directly.
/// Use [`span_error!`] or [`call_site_error!`] instead.
pub fn trigger_error<T: Into<MacroError>>(err: T) -> ! {
    panic!(Payload(err.into()))
}

/// Execute the closure and catch all the panics triggered by
/// [`trigger_error`], converting them to [`proc_macro::TokenStream`].
/// All the other panics will be passed through as is.
///
/// You're not supposed to use this function directly, use [`filter_macro_errors!`]
/// instead.
pub fn filter_macro_error_panics<F>(f: F) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream,
{
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(stream) => stream,
        Err(boxed) => match boxed.downcast::<Payload>() {
            Ok(err) => err.0.into_compile_error().into(),
            Err(other) => resume_unwind(other),
        },
    }
}

struct Payload(MacroError);

// SAFE: Payload is private, a user can't use it to make any harm.
unsafe impl Send for Payload {}
unsafe impl Sync for Payload {}
