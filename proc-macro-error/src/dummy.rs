//! Facility to emit dummy implementations (or whatever) in case
//! an error happen.
//!
//! `compile_error!` does not interrupt compilation right away. This means
//! `rustc` doesn't just show you the error and abort, it carries on the
//! compilation process, looking for other errors to report.
//!
//! Let's consider an example:
//!
//! ```rust,ignore
//! trait MyTrait {
//!     fn do_thing();
//! }
//!
//! // this proc macro is supposed to generate MyTrait impl
//! #[proc_macro_derive(MyTrait)]
//! fn example(input: TokenStream) -> TokenStream {
//!     // somewhere deep inside
//!     span_error!(span, "something's wrong");
//!
//!     // this implementation will be generated if no error happened
//!     quote! {
//!         impl MyTrait for #name {
//!             fn do_thing() {/* whatever */}
//!         }
//!     }
//! }
//!
//! // ================
//! // in main.rs
//!
//! // this derive triggers an error
//! #[derive(MyTrait)] // first BOOM!
//! struct Foo;
//!
//! fn main() {
//!     Foo::do_thing(); // second BOOM!
//! }
//! ```
//!
//! The problem is: the generated token stream contains only `compile_error!`
//! invocation, the impl was not generated. That means user will see two compilation
//! errors:
//!
//! ```text
//! error: set_dummy test
//!  --> $DIR/probe.rs:9:10
//!   |
//! 9 |#[proc_macro_derive(MyTrait)]
//!   |                    ^^^^^^^
//!
//! error[E0599]: no function or associated item named `do_thing` found for type `Foo` in the current scope
//!  --> src\main.rs:3:10
//!   |
//! 1 | struct Foo;
//!   | ----------- function or associated item `do_thing` not found for this
//! 2 | fn main() {
//! 3 |     Foo::do_thing(); // second BOOM!
//!   |          ^^^^^^^^ function or associated item not found in `Foo`
//! ```
//!
//! But the second error is meaningless! We definitely need to fix this.
//!
//! Most used approach in cases like this is "dummy implementation" -
//! omit `impl MyTrait for #name` and fill functions bodies with `unimplemented!()`.
//!
//! This is how you do it:
//!
//! ```rust,ignore
//!  trait MyTrait {
//!      fn do_thing();
//!  }
//!
//!  // this proc macro is supposed to generate MyTrait impl
//!  #[proc_macro_derive(MyTrait)]
//!  fn example(input: TokenStream) -> TokenStream {
//!      // first of all - we set a dummy impl which will be appended to
//!      // `compile_error!` invocations in case a trigger does happen
//!      proc_macro_error::set_dummy(Some(quote! {
//!          impl MyTrait for #name {
//!              fn do_thing() { unimplemented!() }
//!          }
//!      }));
//!
//!      // somewhere deep inside
//!      span_error!(span, "something's wrong");
//!
//!      // this implementation will be generated if no error happened
//!      quote! {
//!          impl MyTrait for #name {
//!              fn do_thing() {/* whatever */}
//!          }
//!      }
//!  }
//!
//!  // ================
//!  // in main.rs
//!
//!  // this derive triggers an error
//!  #[derive(MyTrait)] // first BOOM!
//!  struct Foo;
//!
//!  fn main() {
//!      Foo::do_thing(); // no more errors!
//!  }
//! ```

use proc_macro2::TokenStream;
use std::cell::Cell;

thread_local! {
    pub(crate) static DUMMY_IMPL: Cell<Option<TokenStream>> = Cell::new(None);
}

/// Sets dummy token stream which will be appended to `compile_error!(msg);...`
/// invocations, should a trigger happen and/or global error storage would
/// appear not to be empty. Returns an old dummy, if set.
///
/// # Warning:
/// If you do `set_dummy(ts)` you **must** do `cleanup()`
/// before macro execution completes ([`filer_macro_errors!`] does it for you)!
/// Otherwise `rustc` will fail with cryptic
/// ```text
/// thread 'rustc' panicked at 'use-after-free in `proc_macro` handle', src\libcore\option.rs:1166:5
/// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace.
/// ```
pub fn set_dummy(dummy: TokenStream) -> Option<TokenStream> {
    DUMMY_IMPL.with(|old_dummy| old_dummy.replace(dummy))
}

/// Clear the global error storage, returning the old dummy, if present.
///
/// # Warning:
/// You **must** do it before macro execution completes
/// ([`filter_macro_errors!`] does it for you)! If dummy
/// is set at the end moment of macro execution `rustc` will fail with cryptic
///
/// ```text
/// thread 'rustc' panicked at 'use-after-free in `proc_macro` handle', src\libcore\option.rs:1166:5
/// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace.
/// ```
pub fn cleanup() -> Option<TokenStream> {
    DUMMY_IMPL.with(|old_dummy| old_dummy.replace(None))
}
