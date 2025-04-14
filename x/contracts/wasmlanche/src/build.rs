// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#[cfg(feature = "build")]
use std::env;
#[cfg(feature = "build")]
use std::path::Path;
#[cfg(feature = "build")]
use std::process::Command;

pub const BUILD_DIR_NAME: &str = "target";

#[cfg(feature = "build")]
#[allow(clippy::module_name_repetitions)]
/// Put this in your build.rs file. It currently relies on `/build` directory to be in your crate root.
/// # Panics
/// Will panic when attempting to build the wasm file fails.
pub fn build_wasm() -> Result<(), Box<dyn std::error::Error>> {
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();

    if target != "wasm32-unknown-unknown" {
        let package_name = env::var("CARGO_PKG_NAME").unwrap();
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

        let profile = if profile == "release" {
            &profile
        } else {
            "test"
        };

        let features = env::vars()
            .filter_map(|(key, value)| {
                if key.starts_with("CARGO_FEATURE_") && value == "1" {
                    let feature = key.trim_start_matches("CARGO_FEATURE_").to_lowercase();

                    match feature.as_str() {
                        "bindings" | "test" => None,
                        _ => Some(feature),
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let target_dir = format!("{manifest_dir}/{BUILD_DIR_NAME}");
        let mut command = Command::new("cargo");
        command
            .arg("rustc")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--target-dir")
            .arg(&target_dir);

        if profile == "release" {
            command.arg("--release");
        }

        if !features.is_empty() {
            command.arg("--features").arg(features.join(","));
        }

        command.arg("--crate-type").arg("cdylib");

        let cargo_build_output = command
            .output()
            .expect("command should execute even if it fails");

        let profile = if profile == "release" {
            "release"
        } else {
            "debug"
        };

        if !cargo_build_output.status.success() {
            let stdout = String::from_utf8_lossy(&cargo_build_output.stdout);
            let stderr = String::from_utf8_lossy(&cargo_build_output.stderr);

            println!("cargo:warning=stdout:");

            for line in stdout.lines() {
                println!("cargo:warning={line}");
            }

            println!("cargo:warning=stderr:");

            for line in stderr.lines() {
                println!("cargo:warning={line}");
            }

            println!("cargo:warning=exit-status={}", cargo_build_output.status);

            return Err("failed to build wasm file".into());
        }

        let target_dir = Path::new(&target_dir)
            .join("wasm32-unknown-unknown")
            .join(profile)
            .join(format!("{}.wasm", package_name.replace('-', "_")));

        let target_dir = match target_dir.canonicalize() {
            Ok(target_dir) => target_dir,
            err @ Err(_) => {
                println!("cargo:warning= not found -> {target_dir:?}");
                err.expect("failed to canonicalize wasm file path")
            }
        };

        println!("cargo:warning=`.wasm` file at {target_dir:?}");

        let target_dir = target_dir
            .to_str()
            .expect("crate name must not contain any non-utf8 characters");
        println!("cargo:rustc-env=CONTRACT_PATH={target_dir}");

        println!(
            r#"cargo:warning=If the simulator fails to find the "{package_name}" contract, try running `cargo clean -p {package_name}` followed by `cargo test` again."#
        );
    }

    Ok(())
}

#[cfg(feature = "build")]
fn main() {
    build_wasm().expect("build_wasm failed");
}
