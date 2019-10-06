//! # proc-macro-error
//!
//! This crate aims to make error reporting in proc-macros simple and easy to use.
//! Migrate from `panic!`-based errors for as little effort as possible!
//!
//! Also, there's ability to [append a dummy token stream][dummy] to your errors.
//!
//! ## Enticement
//!
//! Your errors look like this?
//! ```text
//! error: proc-macro derive panicked
//!   --> $DIR/bool_default_value.rs:11:10
//!    |
//! 11 | #[derive(StructOpt, Debug)]
//!    |          ^^^^^^^^^
//!    |
//!    = help: message: default_value is meaningless for bool
//! ```
//!
//! But you would like it to be like this!
//! ```text
//! error: default_value is meaningless for bool
//!   --> $DIR/bool_default_value.rs:14:24
//!    |
//! 14 |     #[structopt(short, default_value = true)]
//!    |                        ^^^^^^^^^^^^^
//! ```
//!
//! This is exactly what this crate is built for!!!
//!
//! ## Usage
//!
//! ### Panic-like usage
//!
//! ```rust
//! # fn some_logic(_input: &DeriveInput) -> Result<(), Dummy> { unimplemented!() }
//! # fn more_logic(_input: &DeriveInput) -> Result<(), Dummy> { unimplemented!() }
//! # fn more_logic_for_logic_god(_input: &DeriveInput) -> bool { unimplemented!() }
//! # struct Dummy {
//! #     span: proc_macro2::Span,
//! #     msg: String
//! # }
//! # impl Into<MacroError> for Dummy {
//! #     fn into(self) -> MacroError { unimplemented!() }
//! # }
//! use proc_macro_error::*;
//! use proc_macro::TokenStream;
//! use syn::{DeriveInput, parse_macro_input};
//! use quote::quote;
//!
//! # static _IGNORE: &str = "
//! // This is your main entry point
//! #[proc_macro]
//! // this attribute *MUST* be placed on top of the #[proc_macro] function
//! #[proc_macro_error]
//! # ";
//! pub fn make_answer(input: TokenStream) -> TokenStream {
//!     let input = parse_macro_input!(input as DeriveInput);
//!
//!     if let Err(err) = some_logic(&input) {
//!         // we've got a span to blame, let's use it
//!         // This immediately aborts the proc-macro and shows the error
//!         abort!(err.span, "You made an error, go fix it: {}", err.msg);
//!     }
//!
//!     // `Result` has some handy shortcuts if your error type implements
//!     // `Into<MacroError>`. `Option` has one unconditionally.
//!     more_logic(&input).expect_or_abort("What a careless user, behave!");
//!
//!     if !more_logic_for_logic_god(&input) {
//!         // We don't have an exact location this time,
//!         // so just highlight the proc-macro invocation itself
//!         abort_call_site!(
//!             "Bad, bad user! Now go stand in the corner and think about what you did!");
//!     }
//!
//!     // Now all the processing is done, return `proc_macro::TokenStream`
//!     quote!(/* stuff */).into()
//! }
//! ```
//!
//! ### Multiple errors
//!
//! ```rust
//! use proc_macro_error::*;
//! use proc_macro::TokenStream;
//! use syn::{spanned::Spanned, DeriveInput, ItemStruct, Fields, Attribute , parse_macro_input};
//! use quote::quote;
//!
//! # fn process_attr(_a: &Attribute) -> Result<Attribute, String> { unimplemented!() }
//! fn process_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
//!     attrs
//!         .iter()
//!         .filter_map(|attr| match process_attr(attr) {
//!             Ok(res) => Some(res),
//!             Err(msg) => {
//!                 emit_error!(attr.span(), "Invalid attribute: {}", msg);
//!                 None
//!             }
//!         })
//!         .collect()
//! }
//!
//! fn process_fields(_attrs: &Fields) -> Vec<TokenStream> {
//!     // processing fields in pretty much the same way as attributes
//!     unimplemented!()
//! }
//!
//! # static _IGNORE: &str = "
//! #[proc_macro]
//! #[proc_macro_error]
//! # ";
//! pub fn make_answer(input: TokenStream) -> TokenStream {
//!     let input = parse_macro_input!(input as ItemStruct);
//!     let attrs = process_attrs(&input.attrs);
//!
//!     // abort right now if some errors were encountered
//!     // at the attributes processing stage
//!     abort_if_dirty();
//!
//!     let fields = process_fields(&input.fields);
//!
//!     // no need to think about emitted errors
//!     // #[proc_macro_errors] will handle them for you
//!     //
//!     // just return a TokenStream as you normally would
//!     quote!(/* stuff */).into()
//! }
//! ```
//!
//! ## Limitations
//!
//! - No support for warnings.
//! - Very limited support for "help" suggestions.
//! - If a panic occurs somewhere in your macro no errors will be displayed.
//!
//! ## Motivation
//!
//! Error handling in proc-macros sucks. It's not much of a choice today:
//! you either "bubble up" the error up to the top-level of your macro and convert it to
//! a [`compile_error!`][compl_err] invocation or just use a good old panic. Both these ways suck:
//!
//! - Former sucks because it's quite redundant to unroll a proper error handling
//!     just for critical errors that will crash the macro anyway so people mostly
//!     choose not to bother with it at all and use panic. Almost nobody does it,
//!     simple `.expect` is too tempting.
//!
//! - Later sucks because there's no way to carry out span info via `panic!`. `rustc` will highlight
//!     the whole invocation itself but not some specific token inside it.
//!     Furthermore, panics aren't for error-reporting at all; panics are for bug-detecting
//!     (like unwrapping on `None` or out-of range indexing) or for early development stages
//!     when you need a prototype ASAP and error handling can wait. Mixing these usages only
//!     messes things up.
//!
//! - There is [`proc_macro::Diagnostics`] which is awesome but it has been experimental
//!     for more than a year and is unlikely to be stabilized any time soon.
//!
//!     This crate will be deprecated once `Diagnostics` is stable.
//!
//! That said, we need a solution, but this solution must meet these conditions:
//!
//! - It must be better than `panic!`. The main point: it must offer a way to carry span information
//!     over to user.
//! - It must require as little effort as possible to migrate from `panic!`. Ideally, a new
//!     macro with the same semantics plus ability to carry out span info. A support for
//!     emitting multiple errors would be great too.
//! - **It must be usable on stable**.
//!
//! This crate aims to provide such a mechanism. All you have to do is annotate your top-level
//! `#[proc_macro]` function with `#[proc_macro_errors]` attribute and change panics to
//! [`abort!`]/[`abort_call_site!`] where appropriate, see [**Usage**](#usage).
//!
//! ## Disclaimer
//! Please note that **this crate is not intended to be used in any other way
//! than a proc-macro error reporting**, use `Result` and `?` for anything else.
//!
//! [compl_err]: https://doc.rust-lang.org/std/macro.compile_error.html
//! [`proc_macro::Diagnostics`]: https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html

// reexports for use in macros
pub extern crate proc_macro;
pub extern crate proc_macro2;

pub mod dummy;
pub mod multi;
pub mod single;

pub use self::dummy::set_dummy;
pub use self::multi::abort_if_dirty;
pub use self::single::MacroError;
pub use proc_macro_error_attr::proc_macro_error;

use quote::quote;

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

/// This is the entry point for your proc-macro. It is **must** to be
/// used on the top level of the proc-macro (a function annotated with
/// `#[proc_macro*] attribute).
///
/// Typically, you use `#[proc_macro_error]` instead, see [module level docs][self].
pub fn entry_point<F>(f: F) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream,
    F: UnwindSafe,
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
