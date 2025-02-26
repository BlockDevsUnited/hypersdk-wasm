pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::env;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

pub const BUILD_DIR_NAME: &str = "target";
const WASM_TARGET: &str = "wasm32-unknown-unknown";
const RELEASE_PROFILE: &str = "release";

/// Put this in your build.rs file. It currently relies on `/build` directory to be in your crate root.
/// # Panics
/// Will panic when attempting to build the wasm file fails.
pub fn build_wasm() -> Result<()> {
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();

    if target != WASM_TARGET {
        let package_name = env::var("CARGO_PKG_NAME").unwrap();
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

        let profile = if profile == RELEASE_PROFILE {
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

        let mut build_cmd = Command::new("cargo");
        build_cmd
            .current_dir(&manifest_dir)
            .env("RUSTFLAGS", "-C target-feature=+crt-static")
            .arg("build")
            .arg("--target")
            .arg(WASM_TARGET)
            .arg("--profile")
            .arg(profile);

        if !features.is_empty() {
            build_cmd.arg("--features").arg(features.join(","));
        }

        let status = build_cmd.status()?;

        if !status.success() {
            anyhow::bail!("failed to build wasm file");
        }

        #[cfg(feature = "wasm-opt")]
        {
            use wasm_opt::OptimizationOptions;

            let target_dir = Path::new(&manifest_dir).join(BUILD_DIR_NAME);
            let wasm_file = target_dir
                .join(WASM_TARGET)
                .join(profile)
                .join(format!("{}.wasm", package_name));

            OptimizationOptions::new_optimize_for_size()
                .run(&wasm_file, &wasm_file)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = build_wasm();
        assert!(result.is_ok());
    }
}
