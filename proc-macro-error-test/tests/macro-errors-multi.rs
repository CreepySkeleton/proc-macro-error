extern crate proc_macro_error_test;
extern crate trybuild;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui-multi/*.rs");
}
