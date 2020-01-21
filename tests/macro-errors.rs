extern crate rustversion;
extern crate trybuild;

#[rustversion::attr(any(before(1.39)), ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
