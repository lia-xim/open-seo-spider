fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // The debug CLI dispatch and server initialization need more than the
    // Windows linker default. Match the usual 8 MiB Unix main-thread reserve;
    // this reserves address space, not 8 MiB of committed memory per request.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            println!("cargo:rustc-link-arg-bin=open-seo-spider=/STACK:8388608");
        } else {
            println!("cargo:rustc-link-arg-bin=open-seo-spider=-Wl,--stack,8388608");
        }
    }
}
