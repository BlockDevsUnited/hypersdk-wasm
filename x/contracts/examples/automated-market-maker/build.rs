// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasmlanche_build::build_wasm()?;
    Ok(())
}
