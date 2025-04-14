fn main() {
    if let Err(e) = wasmlanche_build::build_wasm() {
        eprintln!("Error in build.rs: {}", e);
        std::process::exit(1);
    }
}
