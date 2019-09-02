# v0.2.6 (2019-09-02)
* Introduce support for dummy implementations via `dummy::set_dummy`
* `multi::*` is now deprecated, will be completely rewritten in v0.3

# v0.2.0 (2019-08-15)

## Breaking changes
* `trigger_error` replaced with `MacroError::trigger` and `filter_macro_error_panics`
  is hidden from docs.
  This is not quite a breaking change since users weren't supposed to use these functions directly anyway.
* All dependencies are updated to `v1.*`.

## New features
* Ability to stack multiple errors via `multi::MultiMacroErrors` and emit them at once.

## Improvements
* Now `MacroError` implements `std::fmt::Display` instead of `std::string::ToString`.
* `MacroError::span` inherent method.
* `From<MacroError> for proc_macro/proc_macro2::TokenStream` implementations.
* `AsRef/AsMut<String> for MacroError` implementations.

# v0.1.x (2019-07-XX)

## New features
* An easy way to report errors inside within a proc-macro via `span_error`,
  `call_site_error` and `filter_macro_errors`.
