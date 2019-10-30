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

use std::fmt::{Display, Formatter};

// FIXME: this can be greatly simplified via $()?
// as soon as MRSV hits 1.32

/// Shortcut for `MacroError::new($span.into(), format!($fmt, $args...))`
#[macro_export(local_inner_macros)]
macro_rules! diagnostic {
    // from alias
    ($err:expr) => {{
        $crate::Diagnostic::from($err)
    }};

    // span, message, help
    ($span:expr, $fmt:expr, $($args:expr),+ ; $($rest:tt)+) => {{
        let diag = $crate::Diagnostic::span_error(
            $span.into(),
            __pme__format!($fmt, $($args),*)
        );
        __pme__suggestions!(diag $($rest)*);
        diag
    }};

    ($span:expr, $msg:expr ; $($rest:tt)+) => {{
        let diag = $crate::Diagnostic::span_error($span.into(), $msg.to_string());
        __pme__suggestions!(diag $($rest)*);
        diag
    }};

    // span, message, no help
    ($span:expr, $fmt:expr, $($args:expr),+) => {{
        $crate::Diagnostic::span_error(
            $span.into(),
            __pme__format!($fmt, $($args),*)
        )
    }};

    ($span:expr, $msg:expr) => {{
        $crate::Diagnostic::span_error($span.into(), $msg.to_string())
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __pme__format {
    ($($args:tt)*) => {format!($($args)*)};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __pme__suggestions {
    ($var:ident $help:ident =? $msg:expr) => {
        let $var = if let Some(msg) = $msg {
            $var.suggestion(stringify!($help), msg.to_string())
        } else {
            $var
        };
    };
    ($var:ident $help:ident =? $span:expr => $msg:expr) => {
        let $var = if let Some(msg) = $msg {
            $var.span_suggestion($span.into(), stringify!($help), msg.to_string())
        } else {
            $var
        };
    };

    ($var:ident $help:ident =? $msg:expr ; $($rest:tt)*) => {
        __pme__suggestions!($var $help =? $msg);
        __pme__suggestions!($var $($rest)*);
    };
    ($var:ident $help:ident =? $span:expr => $msg:expr ; $($rest:tt)*) => {
        __pme__suggestions!($var $help =? $span => $msg);
        __pme__suggestions!($var $($rest)*);
    };

    ($var:ident $help:ident = $msg:expr) => {
        let $var = $var.suggestion(stringify!($help), $msg.to_string());
    };
    ($var:ident $help:ident = $fmt:expr, $($args:expr),*) => {
        let $var = $var.suggestion(
            stringify!($help),
            format!($fmt, $($args),*)
        );
    };
    ($var:ident $help:ident = $span:expr => $msg:expr) => {
        let $var = $var.span_suggestion($span.into(), stringify!($help), $msg.to_string());
    };
    ($var:ident $help:ident = $span:expr => $fmt:expr, $($args:expr),*) => {
        let $var = $var.span_suggestion(
            $span.into(),
            stringify!($help),
            format!($fmt, $($args),*)
        );
    };

    ($var:ident $help:ident = $msg:expr ; $($rest:tt)*) => {
        __pme__suggestions!($var $help = $msg);
        __pme__suggestions!($var $($rest)*);
    };
    ($var:ident $help:ident = $fmt:expr, $($args:expr),* ; $($rest:tt)*) => {
        __pme__suggestions!($var $help = $fmt, $($args),*);
        __pme__suggestions!($var $($rest)*);
    };
    ($var:ident $help:ident = $span:expr => $msg:expr ; $($rest:tt)*) => {
        __pme__suggestions!($var $help = $span => $msg);
        __pme__suggestions!($var $($rest)*);
    };
    ($var:ident $help:ident = $span:expr => $fmt:expr, $($args:expr),* ; $($rest:tt)*) => {
        __pme__suggestions!($var $help = $span => $fmt, $($args),*);
        __pme__suggestions!($var $($rest)*);
    };
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
        $crate::diagnostic!($($tts)*).abort()
    }};
}

/// Shortcut for `abort!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
#[macro_export]
macro_rules! abort_call_site {
    ($($tts:tt)*) => {{
        let span = $crate::proc_macro2::Span::call_site();
        $crate::diagnostic!(span, $($tts)*).abort()
    }};
}

/// A structure representing a single diagnostic message
pub struct Diagnostic {
    span: Span,
    msg: String,
    suggestions: Vec<(SuggestionKind, String, Span)>,
}

enum SuggestionKind {
    Help,
    Note,
}

impl SuggestionKind {
    fn name(&self) -> &'static str {
        match self {
            SuggestionKind::Note => "note",
            SuggestionKind::Help => "help",
        }
    }
}

impl Diagnostic {
    /// Create new error message to be emited later
    pub fn span_error(span: Span, msg: String) -> Self {
        Diagnostic {
            span,
            msg,
            suggestions: vec![],
        }
    }

    pub fn error(msg: String) -> Self {
        Diagnostic::span_error(Span::call_site(), msg)
    }

    pub fn span_help(mut self, span: Span, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Help, msg, span));
        self
    }

    pub fn help(mut self, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Help, msg, self.span));
        self
    }

    pub fn span_note(mut self, span: Span, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Note, msg, span));
        self
    }

    pub fn note(mut self, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Note, msg, self.span));
        self
    }

    #[doc(hidden)]
    pub fn span_suggestion(self, span: Span, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.span_help(span, msg),
            _ => self.span_note(span, msg),
        }
    }

    #[doc(hidden)]
    pub fn suggestion(self, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.help(msg),
            _ => self.note(msg),
        }
    }

    /// Abort the proc-macro's execution and show the error.
    ///
    /// You're not supposed to use this function directly.
    /// Use [`abort!`] instead.
    pub fn abort(self) -> ! {
        self.emit();
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

impl From<String> for Diagnostic {
    fn from(msg: String) -> Self {
        Diagnostic::error(msg)
    }
}

impl From<&str> for Diagnostic {
    fn from(msg: &str) -> Self {
        Diagnostic::error(msg.into())
    }
}

impl From<syn::Error> for Diagnostic {
    fn from(e: syn::Error) -> Self {
        Diagnostic::span_error(e.span(), e.to_string())
    }
}

impl ToTokens for Diagnostic {
    fn to_tokens(&self, ts: &mut TokenStream) {
        let span = &self.span;
        let msg = syn::LitStr::new(&self.to_string(), *span);
        ts.extend(quote_spanned!(*span=> compile_error!(#msg); ));
    }
}

impl Display for Diagnostic {
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

        let Diagnostic {
            ref msg,
            ref suggestions,
            ..
        } = *self;

        if suggestions.is_empty() {
            Display::fmt(msg, f)
        } else {
            ensure_double_lf(f, msg)?;
            for suggestion in suggestions {
                write!(f, "  {}: ", suggestion.0.name())?;
                ensure_double_lf(f, &suggestion.1)?;
            }

            Ok(())
        }
    }
}

impl<T, E: Into<Diagnostic>> ResultExt for Result<T, E> {
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
                abort!(span, "{}: {}", message, msg)
            }
        }
    }
}
