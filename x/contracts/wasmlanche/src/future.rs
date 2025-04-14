// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

//! Future trait implementations for Wasmlanche async operations
//! This module provides Future trait implementations for various result types

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;

use crate::error::Error;

/// A future implementation for async operation results
#[pin_project]
pub struct AsyncResult<T> {
    pub result: Option<Result<T, Error>>,
}

impl<T> AsyncResult<T> {
    /// Create a new pending async result
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Create a new resolved async result
    pub fn with_result(result: Result<T, Error>) -> Self {
        Self { result: Some(result) }
    }

    /// Resolve this async result with a value
    pub fn resolve(&mut self, result: Result<T, Error>) {
        self.result = Some(result);
    }
}

impl<T> Future for AsyncResult<T> {
    type Output = Result<T, Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.result.take() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

/// A future implementation for async state operations
#[pin_project]
pub struct StateResult<T> {
    pub result: Option<Result<Option<T>, Error>>,
}

impl<T> StateResult<T> {
    /// Create a new pending state result
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Create a new resolved state result
    pub fn with_result(result: Result<Option<T>, Error>) -> Self {
        Self { result: Some(result) }
    }

    /// Resolve this state result with a value
    pub fn resolve(&mut self, result: Result<Option<T>, Error>) {
        self.result = Some(result);
    }
}

impl<T> Future for StateResult<T> {
    type Output = Result<Option<T>, Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.result.take() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

/// A future implementation for cross-contract async calls
#[pin_project]
pub struct ContractCallResult {
    pub result: Option<Result<Vec<u8>, String>>,
}

impl ContractCallResult {
    /// Create a new pending contract call result
    pub fn new() -> Self {
        Self { result: None }
    }

    /// Create a new resolved contract call result
    pub fn with_result(result: Result<Vec<u8>, String>) -> Self {
        Self { result: Some(result) }
    }

    /// Resolve this contract call result with a value
    pub fn resolve(&mut self, result: Result<Vec<u8>, String>) {
        self.result = Some(result);
    }
}

impl Future for ContractCallResult {
    type Output = Result<Vec<u8>, String>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.result.take() {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

/// A simple future implementation for operations that produce no result
#[pin_project]
pub struct UnitResult {
    pub completed: bool,
}

impl UnitResult {
    /// Create a new pending unit result
    pub fn new() -> Self {
        Self { completed: false }
    }

    /// Create a new completed unit result
    pub fn completed() -> Self {
        Self { completed: true }
    }

    /// Mark this unit result as completed
    pub fn complete(&mut self) {
        self.completed = true;
    }
}

impl Future for UnitResult {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.completed {
            *this.completed = false; // Reset for future polls
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
