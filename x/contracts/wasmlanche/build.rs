fn main() {
    println!("cargo:rustc-link-search=native=../../target/debug");
    println!("cargo:rustc-link-lib=static=simulator");
    println!("cargo:rerun-if-changed=../../target/debug/libsimulator.a");
}
