use std::env;

fn main() {
    // Only link simulator when not targeting wasm32
    if !env::var("TARGET").map(|t| t.contains("wasm32")).unwrap_or(false) {
        println!("cargo:rustc-link-search=native=../../target/debug");
        println!("cargo:rustc-link-lib=static=simulator");
        println!("cargo:rerun-if-changed=../../target/debug/libsimulator.a");
    }
}
