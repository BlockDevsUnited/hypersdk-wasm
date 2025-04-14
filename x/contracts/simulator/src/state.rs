// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::{collections::HashMap, ops::Deref};

type BoxedSlice = Box<[u8]>;

/// A simple key-value store representing the state of the simulated VM.
#[repr(transparent)]
#[derive(Debug)]
pub struct SimpleState {
    state: HashMap<BoxedSlice, BoxedSlice>,
}

impl SimpleState {
    pub fn new() -> SimpleState {
        SimpleState {
            state: HashMap::new(),
        }
    }

    pub fn get_value(&self, key: &[u8]) -> Option<&[u8]> {
        self.state.get(key).map(|v| v.deref())
    }

    pub fn insert(&mut self, key: BoxedSlice, value: BoxedSlice) {
        self.state.insert(key, value);
    }

    pub fn remove(&mut self, key: &[u8]) {
        self.state.remove(key);
    }
}

impl Default for SimpleState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Mutable<'a> {
    pub state: &'a mut SimpleState,
}

impl<'a> Mutable<'a> {
    pub fn new(state: &'a mut SimpleState) -> Self {
        Self { state }
    }
}
