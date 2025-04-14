// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{env, path::PathBuf, process::Command};

fn main() {
    // Only run when std feature is enabled and not targeting wasm32
    if std::env::var("CARGO_FEATURE_STD").is_ok() && env::var("TARGET").map(|t| !t.contains("wasm32")).unwrap_or(true) {
        let crate_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let profile = env::var("PROFILE").unwrap();

        let target_dir = env::var("CARGO_TARGET_DIR")
            .or_else(|_| -> Result<_, Box<dyn std::error::Error>> {
                let json = Command::new("cargo").arg("metadata").output()?.stdout;
                let json = serde_json::from_slice::<serde_json::Value>(&json)?;
                Ok(json["target_directory"].as_str().unwrap().to_string())
            })
            .expect("Failed to get target directory");
        let target_dir = PathBuf::from(target_dir).join(&profile);

        let state_package = crate_root.join("state");
        let ffi_package = crate_root.join("ffi");
        let common_path = crate_root.join("common");
        let rust_src = crate_root.join("src");

        // rerun the build script if go files change
        println!("cargo:rerun-if-changed={}", state_package.to_string_lossy());
        println!("cargo:rerun-if-changed={}", ffi_package.to_string_lossy());
        println!("cargo:rerun-if-changed={}", common_path.to_string_lossy());
        println!("cargo:rerun-if-changed={}", rust_src.to_string_lossy());

        let output = target_dir.join("libsimulator.a");
        let go_file = ffi_package.join("ffi.go");

        // Compile callbacks.c
        let callbacks_c = common_path.join("callbacks.c");
        let callbacks_o = target_dir.join("callbacks.o");
        let status = Command::new("clang")
            .args(["-c", "-o"])
            .arg(&callbacks_o)
            .arg(&callbacks_c)
            .arg("-I")
            .arg(&common_path)
            .status()
            .expect("Failed to compile callbacks.c");

        if !status.success() {
            panic!("Failed to compile callbacks.c");
        }

        // Build the Go library
        let status = Command::new("go")
            .args(["build", "-buildmode=c-archive", "-tags=debug", "-o"])
            .arg(&output)
            .arg(&go_file)
            .env("CGO_LDFLAGS", format!("-Wl,-force_load,{}", callbacks_o.display()))
            .status()
            .expect("Failed to execute Go build command");

        if !status.success() {
            panic!("Go build command failed");
        }

        println!("cargo:rustc-link-search=native={}", target_dir.to_string_lossy());
        println!("cargo:rustc-link-lib=static=simulator");

        // Generate bindings
        let bindings = bindgen::Builder::default()
            .header(common_path.join("types.h").to_string_lossy())
            .header(common_path.join("callbacks.h").to_string_lossy())
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .generate()
            .expect("Unable to generate bindings");

        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    } else {
        // For wasm32 targets, we don't need to link against the native library
        // Just create a dummy bindings file
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        std::fs::write(
            out_path.join("bindings.rs"),
            "// Dummy bindings for wasm32 target\n",
        ).expect("Couldn't write dummy bindings!");
    }
}
