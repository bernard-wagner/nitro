// Copyright 2021-2022, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE.md

#[cfg(feature = "c_brotli")]
fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    println!("cargo:rustc-link-search=../../target/lib-wasm/");
    println!("cargo:rustc-link-search=../target/lib/");
    println!("cargo:rustc-link-lib=static=brotlienc-static");
    println!("cargo:rustc-link-lib=static=brotlidec-static");
    println!("cargo:rustc-link-lib=static=brotlicommon-static");
}

#[cfg(not(feature = "c_brotli"))]
fn main() {
    // No-op for non-C Brotli builds
    // This is to ensure that the build script runs without errors
    println!("cargo:rerun-if-changed=build.rs");
}