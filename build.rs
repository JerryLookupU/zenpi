fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("PROFILE").as_deref() == Ok("release")
    {
        // Strip at link time, before the UUID is computed; post-link stripping
        // can leave a UUID derived from transient object paths. Keep signatures.
        println!("cargo:rustc-link-arg-bin=zenpi=-Wl,-S,-x,-reproducible");
    }
}
