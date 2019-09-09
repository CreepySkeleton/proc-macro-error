//! This module contains data types and functions to be used for single-error reporting.
//!
//! These are supposed to be used through [`span_error!`] and [`call_site_error!`],
//! see [crate level documentation](crate).

use crate::Payload;
use crate::ResultExt;
use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use quote::ToTokens;
use std::convert::{AsMut, AsRef};
use std::fmt::{Display, Formatter};

/// Makes a [`MacroError`] instance from provided arguments (`panic!`-like)
/// and triggers panic in hope it will be caught by [`filter_macro_errors!`].
///
/// # Syntax
///
/// This macro is meant to be a `panic!` drop-in replacement so its syntax is very similar to `panic!`,
/// but it has three forms instead of two:
///
/// 1. "panic-format-like" form: span, formatting [`str`] literal, comma-separated list of args.
///     First argument is a span, all the rest gets passed to [`format!`] to build the error message.
/// 2. "panic-single-arg-like" form: span, expr, no comma at the end.
///     First argument is a span, the second is our error message, it must implement [`ToString`].
/// 3. "MacroError::trigger-like" form: single expr.
///     Literally `MacroError::from(arg).trigger()`. It's here just for convenience so [`span_error!`]
///     can be used with instances of [`syn::Error`], [`MacroError`], [`&str`], [`String`] and so on...
///
#[macro_export]
macro_rules! span_error {
    ($span:expr, $fmt:literal, $($args:expr),*) => {{
        let msg = format!($fmt, $($args),*);
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        $crate::MacroError::new($span.into(), msg).trigger()
    }};

    ($span:expr, $msg:expr) => {{
        // we use $span.into() so it would work with proc_macro::Span and
        // proc_macro2::Span all the same
        $crate::MacroError::new($span.into(), $msg.to_string()).trigger()
    }};

    ($err:expr) => { $crate::MacroError::from($err).trigger() };
}

/// Shortcut for `span_error!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation-and-getting-started)
#[macro_export]
macro_rules! call_site_error {
    ($fmt:literal, $($args:expr),*) => {{
        use $crate::span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $fmt, $($args),*)
    }};

    ($msg:expr) => {{
        use $crate::span_error;

        let span = $crate::proc_macro2::Span::call_site();
        span_error!(span, $msg)
    }};
}

/// An single error in a proc-macro. This struct preserves
/// the given span so `rustc` can highlight the exact place in user code
/// responsible for the error.
///
/// You're not supposed to use this type directly, use [`span_error!`] and [`call_site_error!`].
#[derive(Debug)]
pub struct MacroError {
    pub(crate) span: Span,
    pub(crate) msg: String,
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

    /// Abandon the old span and replace it with the given one.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    /// Get the span contained.
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    /// Trigger single error, aborting the proc-macro's execution.
    ///
    /// You're not supposed to use this function directly.
    /// Use [`span_error!`] or [`call_site_error!`] instead.
    pub fn trigger(self) -> ! {
        panic!(Payload(self))
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

impl ToTokens for MacroError {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let MacroError { ref msg, ref span } = *self;
        let msg = syn::LitStr::new(msg, span.clone());
        ts.extend(quote_spanned!(*span=> compile_error!(#msg); ));
    }
}

impl Display for MacroError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Display::fmt(&self.msg, f)
    }
}

impl<T, E: Into<MacroError>> ResultExt for Result<T, E> {
    type Ok = T;

    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => e.into().trigger(),
        }
    }

    fn expect_or_exit(self, message: &str) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                let MacroError { msg, span } = e.into();
                let msg = format!("{}: {}", message, msg);
                MacroError::new(span, msg).trigger()
            }
        }
    }
}

impl AsRef<String> for MacroError {
    fn as_ref(&self) -> &String {
        &self.msg
    }
}

impl AsMut<String> for MacroError {
    fn as_mut(&mut self) -> &mut String {
        &mut self.msg
    }
}
