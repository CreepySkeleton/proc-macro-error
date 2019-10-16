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
    // from alias

    ($err:expr) => {{
        $crate::MacroError::from($err)
    }};

    // span, message, help

    ($span:expr, $fmt:expr, $($args:expr),+ ; help = $help_fmt:expr, $($help_args:expr),+) => {{
        let msg = format!($fmt, $($args),*);
        let help = format!($help_fmt, $($help_args),*);
        let span = $span.into();
        $crate::MacroError::with_help(span, msg, help)
    }};

    ($span:expr, $fmt:expr, $($args:expr),+ ; help = $help_msg:expr) => {{
        let msg = format!($fmt, $($args),*);
        let help = $help_msg.to_string();
        let span = $span.into();
        $crate::MacroError::with_help(span, msg, help)
    }};

    ($span:expr, $msg:expr ; help = $help_msg:expr, $($help_args:expr),+) => {{
        let msg = $msg.to_string();
        let help = format!($help_fmt, $($help_args),*);
        let span = $span.into();
        $crate::MacroError::with_help(span, msg, help)
    }};

    ($span:expr, $msg:expr ; help = $help_msg:expr) => {{
        let msg = $msg.to_string();
        let help = $help_msg.to_string();
        let span = $span.into();
        $crate::MacroError::with_help(span, msg, help)
    }};

    // span, message, no help

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
    ($($tts:tt)*) => {{
        $crate::macro_error!($($tts)*).abort()
    }};
}

/// Shortcut for `abort!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! abort_call_site {
    ($($tts:tt)*) => {{
        let span = $crate::proc_macro2::Span::call_site();
        $crate::macro_error!(span, $($tts)*).abort()
    }};
}

/// An single error message in a proc macro with span info attached.
#[derive(Debug)]
pub struct MacroError {
    pub(crate) span: Span,
    pub(crate) msg: String,
    pub(crate) help: Option<String>,
}

impl MacroError {
    /// Create an error with the span and message provided.
    pub fn new(span: Span, msg: String) -> Self {
        MacroError {
            span,
            msg,
            help: None,
        }
    }

    /// Create an error with the span, the message, and the help message provided.
    pub fn with_help(span: Span, msg: String, help: String) -> Self {
        MacroError {
            span,
            msg,
            help: Some(help),
        }
    }

    /// A shortcut for `MacroError::new(Span::call_site(), message)`
    pub fn call_site(msg: String) -> Self {
        MacroError::new(Span::call_site(), msg)
    }

    /// A shortcut for `MacroError::with_help(Span::call_site(), message, help)`
    pub fn call_site_help(msg: String, help: String) -> Self {
        MacroError::with_help(Span::call_site(), msg, help)
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

    /// Display the error while not aborting macro execution.
    ///
    /// You're not supposed to use this function directly.
    /// Use [`emit_error!`] instead.
    pub fn emit(self) {
        push_error(self);
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
        let span = &self.span;
        let msg = syn::LitStr::new(&self.to_string(), *span);
        ts.extend(quote_spanned!(*span=> compile_error!(#msg); ));
    }
}

impl Display for MacroError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        fn ensure_double_lf(f: &mut Formatter, s: &str) -> std::fmt::Result {
            if s.ends_with("\n\n") {
                Display::fmt(s, f)
            } else if s.ends_with('\n') {
                write!(f, "{}\n", s)
            } else {
                write!(f, "{}\n\n", s)
            }
        }

        let MacroError {
            ref msg, ref help, ..
        } = *self;
        if let Some(help) = help {
            ensure_double_lf(f, msg)?;
            write!(f, "  help: ")?;
            ensure_double_lf(f, help)
        } else {
            Display::fmt(msg, f)
        }
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
                let e = e.into();
                let span = e.span;
                let msg = e.to_string();
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
