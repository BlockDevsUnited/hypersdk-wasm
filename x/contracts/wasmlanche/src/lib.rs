// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![deny(clippy::pedantic)]
// "build" and "debug" features enable std, so does `test`
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(target_arch = "wasm32", no_std)]

extern crate alloc;

#[cfg(target_arch = "wasm32")]
extern crate wee_alloc;

// Internal modules
pub mod context;
pub mod error;
pub mod host;
pub mod memory;
pub mod simulator;
pub mod state;
pub mod types;

// Re-exports
pub use context::Context;
pub use error::Error;
pub use host::StateAccessor;
pub use memory::HostPtr;
pub use sdk_macros::public;
pub use simulator::Address;
pub use simulator::Simulator;
pub use state::Schema;
pub use types::{Gas, Id};

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(target_arch = "wasm32")]
#[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct Contract;

#[cfg(target_arch = "wasm32")]
#[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct Context {
    _private: (),
}

#[cfg(target_arch = "wasm32")]
impl Context {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

pub use borsh;

/// Welcome to the wasmlanche! This SDK provides a set of tools to help you write
/// your smart-contracts in Rust to be deployed and run on a `HyperVM`.
///
/// There are three main concepts that you need to understand to write smart-contracts:
///
/// 1. `Context`: This is the main interface to interact with the blockchain. It provides
///    methods to read and write state, get account balances, and more.
///
/// 2. `Schema`: This trait is used to define the structure of your contract's state.
///    It provides methods to serialize and deserialize your state.
///
/// 3. `public`: This attribute macro is used to mark functions that can be called from
///    outside the contract.
///
/// Here's a simple example of a contract that adds two numbers:
///
/// ```rust
/// use wasmlanche::prelude::*;
///
/// #[public]
/// fn add(a: u64, b: u64) -> u64 {
///     a + b
/// }
/// ```


pub mod prelude {
    pub use super::{Context, Schema};
    pub use sdk_macros::public;
}
