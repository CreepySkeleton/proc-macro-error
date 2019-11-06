//! Facility for stacking and emitting multiple errors.
//!
//! [`abort!`] macro stops a proc-macro *right away*, much like in a panic-like
//! fashion. But sometimes you *do not* want to stop right there, for example you're
//! processing a list of attributes and want to *emit* a separate error for every
//! mis-built attribute.
//!
//! The [`emit_error!`] and [`emit_call_site_error!`] macros are just for it!

