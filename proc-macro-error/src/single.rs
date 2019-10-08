//! This module contains data types and functions to be used for single-error reporting.
//!
//! These are supposed to be used through [`abort!`] and [`abort_call_site!`],
//! see [crate level documentation](crate).

use crate::{
    multi::{abort_now, push_error},
    ResultExt,
};

use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};

use std::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

/// Shortcut for `MacroError::new($span.into(), format!($fmt, $args...))`
#[macro_export]
macro_rules! macro_error {
    ($span:expr, $fmt:expr, $($args:expr),+) => {{
        let msg = format!($fmt, $($args),*);
        let span = $span.into();
        $crate::MacroError::new(span, msg)
    }};

    ($span:expr, $msg:expr) => {{
        $crate::MacroError::new($span.into(), $msg.to_string())
    }};
}

/// Makes a [`MacroError`] instance from provided arguments and aborts showing it.
///
/// # Syntax
///
/// This macro is meant to be a `panic!` drop-in replacement so its
/// syntax is very similar to `panic!`, but it has three forms instead of two:
///
/// 1. "panic-format-like" form: `abort!(span_expr, format_str_literal [, format_args...])
///
///     First argument is a span, all the rest is passed to [`format!`] to build the error message.
///
/// 2. "panic-single-arg-like" form: `abort!(span_expr, error_expr)`
///
///     First argument is a span, the second is the error message, it must implement [`ToString`].
///
/// 3. `MacroError::abort`-like form: `abort!(error_expr)`
///
///     Literally `MacroError::from(arg).abort()`. It's here just for convenience so [`abort!`]
///     can be used with instances of [`syn::Error`], [`MacroError`], [`&str`], [`String`]
///     and so on...
///
#[macro_export]
macro_rules! abort {
    ($span:expr, $fmt:expr, $($args:expr),*) => {{
        use $crate::macro_error;
        macro_error!($span, $fmt, $($args),*).abort()
    }};

    ($span:expr, $msg:expr) => {{
        use $crate::macro_error;
        macro_error!($span, $msg).abort()
    }};

    ($err:expr) => { $crate::MacroError::from($err).abort() };
}

/// Shortcut for `abort!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! abort_call_site {
    ($fmt:expr, $($args:expr),*) => {{
        use $crate::abort;

        let span = $crate::proc_macro2::Span::call_site();
        abort!(span, $fmt, $($args),*)
    }};

    ($msg:expr) => {{
        use $crate::abort;

        let span = $crate::proc_macro2::Span::call_site();
        abort!(span, $msg)
    }};
}

/// An single error message in a proc macro with span info attached.
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

    /// Replace the span info with `span`. Returns old span.
    pub fn set_span(&mut self, span: Span) -> Span {
        std::mem::replace(&mut self.span, span)
    }

    /// Get the span contained.
    pub fn span(&self) -> Span {
        self.span
    }

    /// Abort the proc-macro's execution and show the error.
    ///
    /// You're not supposed to use this function directly.
    /// Use [`abort!`] instead.
    pub fn abort(self) -> ! {
        push_error(self);
        abort_now()
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
        let msg = syn::LitStr::new(msg, *span);
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

    fn unwrap_or_abort(self) -> T {
        match self {
            Ok(res) => res,
            Err(e) => e.into().abort(),
        }
    }

    fn expect_or_abort(self, message: &str) -> T {
        match self {
            Ok(res) => res,
            Err(e) => {
                let MacroError { msg, span } = e.into();
                abort!(span, "{}: {}", message, msg);
            }
        }
    }
}

impl Deref for MacroError {
    type Target = str;

    fn deref(&self) -> &str {
        &self.msg
    }
}

impl DerefMut for MacroError {
    fn deref_mut(&mut self) -> &mut str {
        &mut self.msg
    }
}
