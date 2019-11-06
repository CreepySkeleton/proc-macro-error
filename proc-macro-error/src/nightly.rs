use std::sync::atomic::{AtomicBool, Ordering};

use proc_macro::{Diagnostic as PDiag, Level as PLevel};

use crate::{abort_now, check_correctness, Diagnostic, Level, SuggestionKind};

pub fn abort_if_dirty() {
    check_correctness();
    if IS_DIRTY.load(Ordering::SeqCst) {
        abort_now()
    }
}

pub(crate) fn cleanup() -> Vec<Diagnostic> {
    vec![]
}

pub(crate) fn emit_diagnostic(diag: Diagnostic) {
    IS_DIRTY.store(true, Ordering::SeqCst);

    #[allow(unused)]
    let Diagnostic {
        level,
        span,
        msg,
        suggestions,
    } = diag;

    let mut res = PDiag::spanned(span.unwrap(), PLevel::from(level), msg);

    for (kind, msg, span) in suggestions {
        res = match (kind, span) {
            (SuggestionKind::Note, Some(span)) => res.span_note(span.unwrap(), msg),
            (SuggestionKind::Help, Some(span)) => res.span_help(span.unwrap(), msg),
            (SuggestionKind::Note, None) => res.note(msg),
            (SuggestionKind::Help, None) => res.help(msg),
        }
    }

    res.emit()
}

impl From<Level> for PLevel {
    fn from(level: Level) -> PLevel {
        match level {
            Level::Warning => PLevel::Warning,
            Level::Error => PLevel::Error,
            _ => unreachable!(),
        }
    }
}

static IS_DIRTY: AtomicBool = AtomicBool::new(false);
