#[rustversion::attr(not(all(stable, since(1.36))), ignore)]
#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
