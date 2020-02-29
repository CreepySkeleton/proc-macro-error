fn main() {
    let (version, _, _) = version_check::triple().unwrap();

    if version.at_most("1.36.0") {
        println!("cargo:rustc-cfg=always_assert_unwind");
    }
}
