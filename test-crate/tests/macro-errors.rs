#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    let version = version_check::Version::read().unwrap();

    if version.at_least("1.39.0") {
        t.compile_fail("tests/ui-post_1.39/*.rs");
    } else {
        t.compile_fail("tests/ui-pre_1.39/*.rs");
    }
}
