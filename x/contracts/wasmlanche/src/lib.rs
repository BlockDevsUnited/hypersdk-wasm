// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#![deny(clippy::pedantic)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]
#![cfg_attr(target_arch = "wasm32", no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod build;
pub mod context;
pub mod error;
pub mod events;
pub mod gas;
pub mod host;
pub mod memory;
pub mod safety;
pub mod simulator;
pub mod state;
pub mod types;

pub use crate::{
    context::Context,
    error::Error,
    events::{Event, EventLog},
    gas::GasCounter,
    host::Host,
    memory::Memory,
    simulator::Simulator,
    state::StateAccess,
    types::WasmlAddress,
};

pub const ID_LEN: usize = 32;

/// Welcome to the wasmlanche! This SDK provides a set of tools to help you write
/// your smart-contracts in Rust to be deployed and run on a `HyperVM`.
/// 
/// # Getting Started
/// 
/// To get started, create a new Rust project and add the following to your
/// `Cargo.toml`:
/// 
/// ```toml
/// [dependencies]
/// wasmlanche = { git = "https://github.com/hyperledger/wasmlanche" }
/// ```
/// 
/// Then, create a new file called `lib.rs` and add the following:
/// 
/// ```rust
/// use wasmlanche::prelude::*;
/// 
/// #[public]
/// fn init() {
///     // Your initialization code here
/// }
/// 
/// #[public]
/// fn handle() {
///     // Your contract code here
/// }
/// ```
/// 
/// # Features
/// 
/// The wasmlanche SDK provides the following features:
/// 
/// - `std` - Enable standard library features
/// - `no_std` - Disable standard library features
/// 
/// By default, the `std` feature is enabled.
/// 
/// # Examples
/// 
/// For more examples, see the `examples` directory in the repository.
/// 
/// # License
/// 
/// This project is licensed under the Apache License, Version 2.0.
/// 
/// # Contributing
/// 
/// We welcome contributions! Please see the `CONTRIBUTING.md` file in the
/// repository for more information.

/// Re-exports commonly used types and traits.
pub mod prelude {
    pub use super::{Context, Error, Event, EventLog, GasCounter};
    pub use sdk_macros::public;
}

pub use borsh;

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(target_arch = "wasm32")]
#[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct Contract;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_context() {
        let state = Arc::new(RwLock::new(host::HostState::default()));
        let host = Arc::new(RwLock::new(Host::new(state)));
        let mut context = Context::new(
            WasmlAddress::new(vec![1, 2, 3]),
            0,
            0,
            host,
            None,
        );

        // Test event handling
        let event = Event::StateChange {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        };
        context.add_event(event).await.unwrap();
        let events = context.get_events().await;
        assert_eq!(events.len(), 1);
    }
}
