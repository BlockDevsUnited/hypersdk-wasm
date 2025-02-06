// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

extern crate alloc;

use std::collections::HashMap;
use std::io;
use std::mem;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::Error as WasmlError,
    host::StateAccessor,
};

#[derive(Debug)]
pub enum Error {
    /// borsh error
    BorshError(io::Error),
    /// state error
    StateError(String),
}

/// A trait for defining the associated value for a given state-key.
/// This trait is not meant to be implemented manually but should instead be implemented with the [`state_schema!`](crate::state_schema) macro.
/// # Safety
/// Do not implement this trait manually. Use the [`state_schema`](crate::state_schema) macro instead.
pub trait Schema {
    type Value: BorshSerialize + BorshDeserialize;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self::Value, Error>;
}

#[derive(Debug)]
pub(crate) struct PrefixedKey<K> {
    prefix: Vec<u8>,
    key: K,
}

impl<K> PrefixedKey<K> {
    fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<Self>()) }
    }
}

#[derive(Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Cache {
    host: StateAccessor,
    cache: HashMap<Vec<u8>, Vec<u8>>,
    deleted: Vec<Vec<u8>>,
}

impl Cache {
    pub fn new(host: StateAccessor) -> Self {
        Self {
            host,
            cache: HashMap::new(),
            deleted: Vec::new(),
        }
    }

    pub fn get<K: Schema>(&mut self, key: &K) -> Result<Option<K::Value>, Error> {
        let key_bytes = key.to_bytes();
        
        if self.deleted.contains(&key_bytes) {
            return Ok(None);
        }

        if let Some(value_bytes) = self.cache.get(&key_bytes) {
            let value = K::from_bytes(value_bytes)?;
            return Ok(Some(value));
        }

        if let Some(value_bytes) = self.host.get(key_bytes) {
            let value = K::from_bytes(&value_bytes)?;
            return Ok(Some(value));
        }

        Ok(None)
    }

    pub fn store<K: Schema>(&mut self, key: K, value: K::Value) -> Result<(), Error> {
        let key_bytes = key.to_bytes();
        let value_bytes = borsh::to_vec(&value).map_err(|e| Error::BorshError(e))?;
        self.cache.insert(key_bytes, value_bytes);
        Ok(())
    }

    pub fn delete(&mut self, key_bytes: Vec<u8>) {
        self.deleted.push(key_bytes);
    }

    pub fn flush(&mut self) {
        for key in &self.deleted {
            self.host.delete(key.clone());
        }

        for (key, value) in &self.cache {
            self.host.store(key.clone(), value.clone());
        }

        self.cache.clear();
        self.deleted.clear();
    }
}

pub(crate) fn to_key<K: Schema>(key: K) -> PrefixedKey<K> {
    PrefixedKey {
        prefix: vec![0; mem::size_of::<K>()],
        key,
    }
}

pub trait IntoPairs: private::Sealed {
    fn into_pairs(self) -> Vec<(Vec<u8>, Vec<u8>)>;
}

mod private {
    pub trait Sealed {}
}

impl<K, V> private::Sealed for ((K, V),)
where
    K: Schema<Value = V>,
    V: BorshSerialize,
{
}

impl<K, V> IntoPairs for ((K, V),)
where
    K: Schema<Value = V>,
    V: BorshSerialize,
{
    fn into_pairs(self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let ((key, value),) = self;
        let mut pairs = Vec::with_capacity(1);
        let mut value_bytes = Vec::new();
        value.serialize(&mut value_bytes).expect("Failed to serialize value");
        let prefixed_key = to_key(key);
        pairs.push((prefixed_key.as_bytes().to_vec(), value_bytes));
        pairs
    }
}
