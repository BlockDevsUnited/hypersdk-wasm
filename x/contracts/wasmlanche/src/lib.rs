// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![deny(clippy::pedantic)]
// "build" and "debug" features enable std, so does `test`
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]

//! Welcome to the wasmlanche! This SDK provides a set of tools to help you write
//! your smart-contracts in Rust to be deployed and run on a `HyperVM`.
//!
//! There are three main concepts that you need to understand to write a smart-contract:
//! 1. **State**  
//!    State is the data that is stored on the blockchain. It also follows a schema that you specify with the [`state_schema!`] macro.
//!    <br><br>
//!
//! 2. **Public Functions**  
//!    These are the entry-points of your contract. They are annotated with the [`#[public]`](crate::public) attribute.
//!    <br><br>
//!
//! 3. **Context**  
//!    The [Context] provides all access to the outer context of the execution. It is also used to access and set state with the keys defined by your schema.
//!    <br><br>
//! ## Example
//! ```
//! use wasmlanche::{public, state_schema, Address, Context};
//!
//! type Count = u64;
//!
//! state_schema! {
//!     /// Counter for each address.
//!     Counter(Address) => Count,
//! }
//!
//! /// Gets the count at the address.
//! #[public]
//! pub fn get_value(context: &mut Context, of: Address) -> Count {
//!     context
//!         .get(Counter(of))
//!         .expect("state corrupt")
//!         .unwrap_or_default()
//! }
//!
//! /// Increments the count at the address by the amount.
//! #[public]
//! pub fn inc(context: &mut Context, to: Address, amount: Count) {
//!     let counter = amount + get_value(context, to);
//!
//!     context
//!         .store_by_key(Counter(to), counter)
//!         .expect("serialization failed");
//! }
//!
//! # fn main() {}
//! ```
//!
//! ## Hint
//! Use the [dbg!] macro when testing your contract, along with the `-- --nocapture` argument to your `cargo test` command.

#[cfg(feature = "build")]
pub mod build;

#[cfg(all(feature = "simulator", not(target_arch = "wasm32")))]
pub mod simulator;

#[cfg(not(target_arch = "wasm32"))]
mod context;
#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
mod memory;
#[cfg(not(target_arch = "wasm32"))]
mod state;
mod types;

#[cfg(feature = "debug")]
mod logging;
#[cfg(not(feature = "debug"))]
mod logging {
    #[macro_export]
    macro_rules! dbg {
        // match anything
        ($($token:tt)*) => {};
    }

    pub fn log(_msg: &str) {}
    pub fn register_panic() {}
}

#[cfg(all(feature = "bindings", not(target_arch = "wasm32")))]
pub use self::context::ExternalCallContext;

#[cfg(not(target_arch = "wasm32"))]
pub use self::{
    context::{Context, ExternalCallArgs, ExternalCallError},
    state::{macro_types, Error},
};

pub use self::types::{Address, ContractId, Gas, Id, ID_LEN};

#[doc(hidden)]
#[cfg(not(target_arch = "wasm32"))]
pub use self::memory::HostPtr;

// For wasm32 target, provide dummy types
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

#[cfg(target_arch = "wasm32")]
pub type HostPtr = u32;

#[cfg(target_arch = "wasm32")]
pub use self::logging::{log, register_panic};

pub use sdk_macros::{public, state_schema};

// re-exports
pub use borsh;
pub use bytemuck;
