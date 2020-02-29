fn main() {
    let (version, channel, _) = version_check::triple().unwrap();

    if !channel.is_nightly() {
        println!("cargo:rustc-cfg=use_fallback");
    }

    if version.at_most("1.38.0") || !channel.is_stable() {
        println!("cargo:rustc-cfg=skip_ui_tests");
    }
}
