// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use crate::error::Error;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Maximum allowed depth for nested contract calls
const MAX_CALL_DEPTH: u32 = 8;

/// Protocol version for compatibility checks
const PROTOCOL_VERSION: u32 = 1;

/// Tracks the call depth and other safety-related state
#[derive(Debug)]
pub struct SafetyContext {
    /// Current call depth for nested contract calls
    call_depth: u32,
    /// Map of actor addresses to their current nonces
    nonces: HashMap<Vec<u8>, u64>,
    /// Protocol version of the current contract
    protocol_version: u32,
}

impl SafetyContext {
    pub fn new() -> Self {
        Self {
            call_depth: 0,
            nonces: HashMap::new(),
            protocol_version: PROTOCOL_VERSION,
        }
    }

    /// Increment the call depth and check if it exceeds the maximum
    pub fn enter_call(&mut self) -> Result<(), Error> {
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(Error::MaxDepthExceeded(format!(
                "Call depth {} exceeds maximum of {}",
                self.call_depth + 1, MAX_CALL_DEPTH
            )));
        }
        self.call_depth += 1;
        Ok(())
    }

    /// Decrement the call depth
    pub fn exit_call(&mut self) {
        if self.call_depth > 0 {
            self.call_depth -= 1;
        }
    }

    /// Verify and increment the nonce for an actor
    pub fn verify_and_increment_nonce(&mut self, actor: &[u8], nonce: u64) -> Result<(), Error> {
        let current = match self.nonces.get(actor) {
            Some(n) => *n,
            None => {
                // First time seeing this actor, initialize their nonce to 0
                self.nonces.insert(actor.to_vec(), 0);
                0
            }
        };
        if nonce != current {
            return Err(Error::InvalidNonce(format!(
                "Invalid nonce {}, expected {}",
                nonce, current
            )));
        }
        self.nonces.insert(actor.to_vec(), current + 1);
        Ok(())
    }

    /// Get the current nonce for an actor
    pub fn get_nonce(&self, actor: &[u8]) -> u64 {
        *self.nonces.get(actor).unwrap_or(&0)
    }

    /// Check if the protocol version is compatible
    pub fn check_protocol_version(&self, version: u32) -> Result<(), Error> {
        if version != self.protocol_version {
            return Err(Error::InvalidProtocolVersion(format!(
                "Protocol version mismatch: expected {}, got {}",
                self.protocol_version, version
            )));
        }
        Ok(())
    }
}

/// Thread-safe wrapper around SafetyContext
#[derive(Debug)]
pub struct SafetyManager {
    context: Arc<RwLock<SafetyContext>>,
}

impl SafetyManager {
    pub fn new() -> Self {
        Self {
            context: Arc::new(RwLock::new(SafetyContext::new())),
        }
    }

    pub fn enter_call(&self) -> Result<(), Error> {
        self.context.write().unwrap().enter_call()
    }

    pub fn exit_call(&self) {
        self.context.write().unwrap().exit_call()
    }

    pub fn verify_and_increment_nonce(&self, actor: &[u8], nonce: u64) -> Result<(), Error> {
        let mut context = self.context.write().unwrap();
        let result = context.verify_and_increment_nonce(actor, nonce);
        drop(context);
        result
    }

    pub fn get_nonce(&self, actor: &[u8]) -> u64 {
        let context = self.context.read().unwrap();
        context.get_nonce(actor)
    }

    pub fn check_protocol_version(&self, version: u32) -> Result<(), Error> {
        self.context.read().unwrap().check_protocol_version(version)
    }
}

impl Default for SafetyManager {
    fn default() -> Self {
        Self::new()
    }
}
