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

#![cfg_attr(pme_nightly, feature(proc_macro_diagnostic))]

// reexports for use in macros
#[doc(hidden)]
pub extern crate proc_macro;
#[doc(hidden)]
pub extern crate proc_macro2;

pub use self::dummy::set_dummy;
pub use proc_macro_error_attr::proc_macro_error;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use quote::{quote_spanned, ToTokens};
use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, UnwindSafe};

pub mod dummy;

#[cfg(not(pme_nightly))]
#[path = "stable.rs"]
mod imp;

#[cfg(any(pme_nightly, nightly_fmt))]
#[path = "nightly.rs"]
mod imp;

// FIXME: this can be greatly simplified via $()?
// as soon as MRSV hits 1.32

/// Build [`Diagnostic`] instance from provided arguments.
///
/// # Syntax
///
/// See [the guide][].
///
#[macro_export(local_inner_macros)]
macro_rules! diagnostic {
    // from alias
    ($err:expr) => { $crate::Diagnostic::from($err) };

    // span, message, help
    ($span:expr, $level:expr, $fmt:expr, $($args:expr),+ ; $($rest:tt)+) => {{
        let diag = $crate::Diagnostic::spanned(
            $span.into(),
            $level,
            __pme__format!($fmt, $($args),*)
        );
        __pme__suggestions!(diag $($rest)*);
        diag
    }};

    ($span:expr, $level:expr, $msg:expr ; $($rest:tt)+) => {{
        let diag = $crate::Diagnostic::spanned($span.into(), $level, $msg.to_string());
        __pme__suggestions!(diag $($rest)*);
        diag
    }};

    // span, message, no help
    ($span:expr, $level:expr, $fmt:expr, $($args:expr),+) => {{
        $crate::Diagnostic::spanned(
            $span.into(),
            $level,
            __pme__format!($fmt, $($args),*)
        )
    }};

    ($span:expr, $level:expr, $msg:expr) => {{
        $crate::Diagnostic::spanned($span.into(), $level, $msg.to_string())
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
    ($var:ident ;) => ();

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

/// Abort proc-macro execution right now and display the error.
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
    ($err:expr) => {
        $crate::diagnostic!($err).abort()
    };

    ($span:expr, $($tts:tt)*) => {{
        $crate::diagnostic!($span, $crate::Level::Error, $($tts)*).abort()
    }};
}

/// Shortcut for `abort!(Span::call_site(), msg...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation)
///
/// # Syntax
///
/// See [the guide][].
///
#[macro_export]
macro_rules! abort_call_site {
    ($($tts:tt)*) => {{
        let span = $crate::proc_macro2::Span::call_site();
        $crate::diagnostic!(span, $crate::Level::Error, $($tts)*).abort()
    }};
}

/// Emit an error while not aborting the proc-macro right away.
///
/// The emitted errors will be converted to a `TokenStream` sequence
/// of `compile_error!` invocations after the execution hits the end
/// of the function marked with `[proc_macro_error]`.
///
/// # Syntax
///
/// See [the guide][].
///
#[macro_export]
macro_rules! emit_error {
    ($err:expr) => {
        $crate::diagnostic!($err).emit()
    };

    ($span:expr, $($tts:tt)*) => {
        $crate::diagnostic!($span, $crate::Level::Error, $($tts)*).emit()
    };
}

/// Shortcut for `emit_error!(Span::call_site(), ...)`. This macro
/// is still preferable over plain panic, see [Motivation](#motivation).
///
/// # Syntax
///
/// See [the guide][].
///
#[macro_export]
macro_rules! emit_call_site_error {
    ($($tts:tt)*) => {{
        let span = $crate::proc_macro2::Span()::call_site();
        $crate::diagnostic!(span, $crate::Level::Error, $($tts)*).emit()
    }};
}

/// Represents a diagnostic level
///
/// # Warnings
///
/// Warnings are ignored on stable/beta
#[derive(Debug, PartialEq)]
pub enum Level {
    Error,
    Warning,
    #[doc(hidden)]
    NonExhaustive,
}

/// Represents a single diagnostic message
#[derive(Debug)]
pub struct Diagnostic {
    level: Level,
    span: Span,
    msg: String,
    suggestions: Vec<(SuggestionKind, String, Option<Span>)>,
}

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

impl Diagnostic {
    /// Create a new diagnostic message that points to `Span::call_site()`
    pub fn new(level: Level, message: String) -> Self {
        Diagnostic::spanned(Span::call_site(), level, message)
    }

    /// Create a new diagnostic message that points to the `span`
    pub fn spanned(span: Span, level: Level, message: String) -> Self {
        Diagnostic {
            level,
            span,
            msg: message,
            suggestions: vec![],
        }
    }

    /// Attach a "help" note to your main message, note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_help(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Help, msg, Some(span)));
        self
    }

    /// Attach a "help" note to your main message,
    pub fn help(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Help, msg, None));
        self
    }

    /// Attach a note to your main message, note will have it's own span on nightly.
    ///
    /// # Span
    ///
    /// The span is ignored on stable, the note effectively inherits its parent's (main message) span
    pub fn span_note(mut self, span: Span, msg: String) -> Self {
        self.suggestions
            .push((SuggestionKind::Note, msg, Some(span)));
        self
    }

    /// Attach a note to your main message
    pub fn note(mut self, msg: String) -> Self {
        self.suggestions.push((SuggestionKind::Note, msg, None));
        self
    }

    /// The message of main warning/error (no notes attached)
    pub fn message(&self) -> &str {
        &self.msg
    }

    /// Abort the proc-macro's execution and display the diagnostic.
    ///
    /// # Warnings
    ///
    /// Warnings do not get emitted on stable/beta but this function will abort anyway.
    pub fn abort(self) -> ! {
        self.emit();
        abort_now()
    }

    /// Display the diagnostic while not aborting macro execution.
    ///
    /// # Warnings
    ///
    /// Warnings are ignored on stable/beta
    pub fn emit(self) {
        imp::emit_diagnostic(self);
    }
}

/// Abort macro execution and display all the emitted errors, if any.
///
/// Does nothing if no errors were emitted.
pub fn abort_if_dirty() {
    imp::abort_if_dirty();
}

#[doc(hidden)]
impl Diagnostic {
    pub fn span_suggestion(self, span: Span, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.span_help(span, msg),
            _ => self.span_note(span, msg),
        }
    }

    pub fn suggestion(self, suggestion: &str, msg: String) -> Self {
        match suggestion {
            "help" | "hint" => self.help(msg),
            _ => self.note(msg),
        }
    }
}

impl ToTokens for Diagnostic {
    fn to_tokens(&self, ts: &mut TokenStream) {
        use std::borrow::Cow;

        fn ensure_lf(buf: &mut String, s: &str) {
            if s.ends_with('\n') {
                buf.push_str(s);
            } else {
                buf.push_str(s);
                buf.push('\n');
            }
        }

        let Diagnostic {
            ref msg,
            ref suggestions,
            ref level,
            ..
        } = *self;

        if *level == Level::Warning {
            return;
        }

        let message = if suggestions.is_empty() {
            Cow::Borrowed(msg)
        } else {
            let mut message = String::new();
            ensure_lf(&mut message, msg);
            message.push('\n');

            for (kind, note, _span) in suggestions {
                message.push_str("  = ");
                message.push_str(kind.name());
                message.push_str(": ");
                ensure_lf(&mut message, note);
            }
            message.push('\n');

            Cow::Owned(message)
        };

        let span = &self.span;
        let msg = syn::LitStr::new(&*message, *span);
        ts.extend(quote_spanned!(*span=> compile_error!(#msg); ));
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
                let mut e = e.into();
                e.msg = format!("{}: {}", message, e.msg);
                e.abort()
            }
        }
    }
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

#[derive(Debug)]
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

impl From<syn::Error> for Diagnostic {
    fn from(e: syn::Error) -> Self {
        Diagnostic::spanned(e.span(), Level::Error, e.to_string())
    }
}

/// This is the entry point for a proc-macro.
///
/// **NOT PUBLIC API, SUBJECT TO CHANGE WITHOUT NOTICE**
#[doc(hidden)]
pub fn entry_point<F>(f: F, proc_macro_hack: bool) -> proc_macro::TokenStream
where
    F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
{
    ENTERED_ENTRY_POINT.with(|flag| flag.set(true));
    let caught = catch_unwind(f);
    let dummy = dummy::cleanup();
    let err_storage = imp::cleanup();
    ENTERED_ENTRY_POINT.with(|flag| flag.set(false));

    let mut appendix = TokenStream::new();
    if proc_macro_hack {
        appendix.extend(quote! {
            #[allow(unused)]
            macro_rules! proc_macro_call {
                () => ( unimplemented!() )
            }
        });
    }

    match caught {
        Ok(ts) => {
            if err_storage.is_empty() {
                ts
            } else {
                quote!( #(#err_storage)* #dummy #appendix ).into()
            }
        }

        Err(boxed) => match boxed.downcast::<AbortNow>() {
            Ok(_) => quote!( #(#err_storage)* #dummy #appendix ).into(),
            Err(boxed) => resume_unwind(boxed),
        },
    }
}

fn abort_now() -> ! {
    check_correctness();
    panic!(AbortNow)
}

thread_local! {
    static ENTERED_ENTRY_POINT: Cell<bool> = Cell::new(false);
}

struct AbortNow;

fn check_correctness() {
    if !ENTERED_ENTRY_POINT.with(|flag| flag.get()) {
        panic!(
            "proc-macro-error API cannot be used outside of `entry_point` invocation, \
             perhaps you forgot to annotate your #[proc_macro] function with `#[proc_macro_error]"
        );
    }
}
