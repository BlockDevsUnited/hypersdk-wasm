fn main() {
    // Call build_wasm and explicitly handle the Result
    if let Err(err) = wasmlanche_build::build_wasm() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}
