fn main() {
    println!("cargo:rerun-if-env-changed=CFG_RELEASE_CHANNEL");
    if option_env!("CFG_RELEASE_CHANNEL").map_or(true, |c| c == "nightly" || c == "dev") {
        println!("cargo:rustc-cfg=nightly");
    }
}
