// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(feature = "std")]
use std::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use core::mem::size_of;
use std::io::{Read, Result as IoResult};
use std::fmt;

/// Byte length of an action ID.
pub const ID_LEN: usize = 32;

/// An action ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct Id {
    bytes: [u8; ID_LEN],
}

impl Id {
    /// Create a new ID from bytes.
    pub fn new(bytes: [u8; ID_LEN]) -> Self {
        Self { bytes }
    }

    /// Get the bytes of the ID.
    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.bytes
    }
}

impl BorshSerialize for Id {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.bytes)
    }
}

impl BorshDeserialize for Id {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        if buf.len() < ID_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "buffer too short for Id",
            ));
        }
        let mut bytes = [0u8; ID_LEN];
        bytes.copy_from_slice(&buf[..ID_LEN]);
        *buf = &buf[ID_LEN..];
        Ok(Self { bytes })
    }

    fn deserialize_reader<R: Read>(reader: &mut R) -> IoResult<Self> {
        let mut bytes = [0u8; ID_LEN];
        reader.read_exact(&mut bytes)?;
        Ok(Self { bytes })
    }
}

/// The ID bytes of a contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractId {
    bytes: [u8; 32]
}

impl ContractId {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl BorshSerialize for ContractId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.bytes)
    }
}

impl BorshDeserialize for ContractId {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        if buf.len() < 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "buffer too short for ContractId",
            ));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&buf[..32]);
        *buf = &buf[32..];
        Ok(Self { bytes })
    }

    fn deserialize_reader<R: Read>(reader: &mut R) -> IoResult<Self> {
        let mut bytes = [0u8; 32];
        reader.read_exact(&mut bytes)?;
        Ok(Self { bytes })
    }
}

/// A wrapper around u64 for gas values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gas(pub(crate) u64);

impl Gas {
    /// Create a new Gas value.
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the gas value.
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl From<u64> for Gas {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Gas> for u64 {
    fn from(gas: Gas) -> Self {
        gas.0
    }
}

/// Represents an address where a smart contract is deployed.
#[derive(Clone, Copy, Ord, PartialOrd, PartialEq, Eq, BorshSerialize, BorshDeserialize, Hash, Debug)]
#[repr(transparent)]
pub struct Address([u8; 33]);

// Address is a transparent wrapper around a fixed-size byte array, which is safe to implement Pod and Zeroable for
// Safety: Address is a transparent wrapper around [u8; 33] which is both Pod and Zeroable
unsafe impl Zeroable for Address {}
unsafe impl Pod for Address {}

impl Address {
    pub const LEN: usize = 33;
    pub const ZERO: Self = Self([0; Self::LEN]);

    // Constructor function for Address
    #[must_use]
    pub fn new(bytes: [u8; Self::LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Default for Address {
    fn default() -> Self {
        Self([0; Self::LEN])
    }
}

impl IntoIterator for Address {
    type Item = u8;
    type IntoIter = core::array::IntoIter<Self::Item, { Address::LEN }>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self.0)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 33]> for Address {
    fn from(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }
}

impl From<&[u8; 33]> for Address {
    fn from(bytes: &[u8; 33]) -> Self {
        Self(*bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WasmlAddress {
    bytes: Vec<u8>
}

impl WasmlAddress {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl BorshSerialize for WasmlAddress {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.bytes, writer)
    }
}

impl BorshDeserialize for WasmlAddress {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let bytes = Vec::deserialize(buf)?;
        Ok(Self { bytes })
    }

    fn deserialize_reader<R: Read>(reader: &mut R) -> IoResult<Self> {
        let bytes = Vec::deserialize_reader(reader)?;
        Ok(Self { bytes })
    }
}

impl fmt::Display for WasmlAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", hex::encode(&self.bytes))
    }
}

impl From<Vec<u8>> for WasmlAddress {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl From<&[u8]> for WasmlAddress {
    fn from(bytes: &[u8]) -> Self {
        Self { bytes: bytes.to_vec() }
    }
}

#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "std")]
use std::string::String;
