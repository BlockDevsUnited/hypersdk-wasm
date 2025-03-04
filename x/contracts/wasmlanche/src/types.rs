#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize};
use borsh::maybestd::io::{self, Write, Read, Result as IoResult};
use bytemuck::{Pod, Zeroable};
use core::fmt;
use hex;

/// Byte length of an action ID.
pub const ID_LEN: usize = 32;

/// An action ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct Id {
    bytes: [u8; ID_LEN],
}

impl Id {
    pub fn new(bytes: [u8; ID_LEN]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; ID_LEN] {
        &self.bytes
    }
}

impl BorshSerialize for Id {
    fn serialize<W: Write>(&self, writer: &mut W) -> IoResult<()> {
        writer.write_all(&self.bytes)
    }
}

impl BorshDeserialize for Id {
    fn deserialize(buf: &mut &[u8]) -> IoResult<Self> {
        if buf.len() < ID_LEN {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
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

    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
}

impl BorshSerialize for ContractId {
    fn serialize<W: Write>(&self, writer: &mut W) -> IoResult<()> {
        writer.write_all(&self.bytes)
    }
}

impl BorshDeserialize for ContractId {
    fn deserialize(buf: &mut &[u8]) -> IoResult<Self> {
        if buf.len() < 32 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
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

/// Contract address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct WasmlAddress([u8; 32]);

impl Default for WasmlAddress {
    fn default() -> Self {
        Self([0; 32])
    }
}

impl WasmlAddress {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for WasmlAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex_str = hex::encode(&self.0);
        f.write_str("0x")?;
        f.write_str(&hex_str)
    }
}

impl From<Vec<u8>> for WasmlAddress {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 32);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self(arr)
    }
}

impl From<&[u8]> for WasmlAddress {
    fn from(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 32);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Self(arr)
    }
}

#[derive(Debug, Clone, BorshSerialize)]
#[derive(BorshDeserialize)]
pub struct ContractInput {
    pub method: Vec<u8>,
    pub params: Vec<u8>,
}

impl ContractInput {
    pub fn new(method: &[u8], params: &[u8]) -> Self {
        Self { method: method.to_vec(), params: params.to_vec() }
    }
    
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, borsh::maybestd::io::Error>
    {
        borsh::BorshDeserialize::try_from_slice(slice)
    }
}

#[derive(Debug, Clone, BorshSerialize)]
#[derive(BorshDeserialize)]
pub struct ContractOutput {
    pub data: Vec<u8>,
}

impl ContractOutput {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
    
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, borsh::maybestd::io::Error>
    {
        borsh::BorshDeserialize::try_from_slice(slice)
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
pub struct Address {
    bytes: [u8; 33],
}

impl Address {
    pub const LEN: usize = 33;

    pub fn new(bytes: [u8; Self::LEN]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Default for Address {
    fn default() -> Self {
        Self { bytes: [0; Self::LEN] }
    }
}

impl IntoIterator for Address {
    type Item = u8;
    type IntoIter = core::array::IntoIter<Self::Item, { Address::LEN }>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(self.bytes)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<[u8; 33]> for Address {
    fn from(bytes: [u8; 33]) -> Self {
        Self { bytes }
    }
}

impl From<&[u8; 33]> for Address {
    fn from(bytes: &[u8; 33]) -> Self {
        Self { bytes: *bytes }
    }
}

// Address is a transparent wrapper around a fixed-size byte array, which is safe to implement Pod and Zeroable for
unsafe impl Zeroable for Address {}
unsafe impl Pod for Address {}
