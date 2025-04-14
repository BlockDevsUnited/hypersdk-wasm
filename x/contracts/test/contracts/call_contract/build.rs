fn main() {
    if let Err(e) = wasmlanche_build::build_wasm() {
        println!("cargo:warning=Failed to build WASM: {}", e);
        panic!("Failed to build WASM: {}", e);
    }
}
