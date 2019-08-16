extern crate proc_macro_error_test;

use proc_macro_error_test::make_fn;

make_fn!(it, _, works);

fn main() {
    it_works();
}
