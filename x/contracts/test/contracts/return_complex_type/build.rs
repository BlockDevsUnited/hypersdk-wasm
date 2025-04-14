fn main() {
    // Handle any potential errors from the build process
    if let Err(err) = wasmlanche_build::build_wasm() {
        // Print the error and exit with a non-zero code
        eprintln!("Failed to build WASM: {}", err);
        std::process::exit(1);
    }
}
