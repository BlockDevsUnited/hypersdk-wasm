// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

use std::collections::HashMap;
use borsh::{BorshSerialize, BorshDeserialize};
use cfg_if::cfg_if;

use crate::{
    error::Error,
    types::{Address, Gas},
    memory::HostPtr,
};

#[derive(Debug, Default, Clone, BorshSerialize, BorshDeserialize)]
pub struct StateAccessor {
    // We'll use an in-memory store for now
    store: HashMap<Vec<u8>, Vec<u8>>,
}

impl StateAccessor {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn get(&self, key: Vec<u8>) -> Option<Vec<u8>> {
        self.store.get(&key).cloned()
    }

    pub fn store(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.store.insert(key, value);
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.store.remove(&key);
    }

    pub fn get_balance(&self, _addr: &[u8]) -> u64 {
        // For now, return a default balance
        1000
    }
}

pub struct CallContractArgs<'a> {
    pub contract: &'a [u8],
    pub method: &'a str,
    pub args: &'a [u8],
    pub gas: u64,
}

impl BorshSerialize for CallContractArgs<'_> {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&self.contract.len(), writer)?;
        writer.write_all(self.contract)?;
        BorshSerialize::serialize(&self.method, writer)?;
        BorshSerialize::serialize(&self.args.len(), writer)?;
        writer.write_all(self.args)?;
        BorshSerialize::serialize(&self.gas, writer)?;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;

    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn get_balance(args_ptr: *const u8) -> u64;
        pub fn get_bytes(args_ptr: *const u8) -> HostPtr;
        pub fn put(args_ptr: *const u8);
        pub fn delete(args_ptr: *const u8);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod test {
    use super::*;

    pub fn get_balance(_args: &[u8]) -> u64 {
        1000
    }

    pub fn get_bytes(_args: &[u8]) -> HostPtr {
        HostPtr::null()
    }

    pub fn put(_args: &[u8]) {
        // No-op for tests
    }

    pub fn delete(_args: &[u8]) {
        // No-op for tests
    }
}

#[cfg(feature = "test")]
mod test_wrappers {
    use super::CallContractArgs;
    use crate::{host::StateAccessor, Address, Gas, HostPtr};
    use core::cell::{Cell, RefCell};

    pub const BALANCE_PREFIX: u8 = 0;
    pub const SEND_PREFIX: u8 = 1;
    pub const CALL_FUNCTION_PREFIX: u8 = 2;
    pub const DEPLOY_PREFIX: u8 = 3;

    impl StateAccessor {
        pub fn put(_args: &[u8]) {
            // happens on context drop() -> cache drop() -> flush()
            // this means this function wont do anything
        }

        pub fn get_bytes(_args: &[u8]) -> HostPtr {
            // if calling get_bytes, not found in cache
            HostPtr::null()
        }
    }

    #[derive(Clone)]
    #[cfg_attr(feature = "debug", derive(Debug))]
    pub struct Accessor {
        state: MockState,
    }

    impl Default for Accessor {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Accessor {
        pub fn new() -> Self {
            Accessor {
                state: MockState::new(),
            }
        }

        pub fn state(&self) -> &MockState {
            &self.state
        }

        pub fn new_deploy_address(&self) -> Address {
            let address: [u8; 33] = [self.state().deploy(); 33];
            Address::new(address)
        }

        pub fn deploy(&self, key: &[u8]) -> HostPtr {
            let key = [DEPLOY_PREFIX]
                .iter()
                .chain(key.iter())
                .copied()
                .collect::<Vec<u8>>();
            let val = self.state.get(&key);

            assert!(
                !val.is_null(),
                "Deploy function not mocked. Please mock the function call."
            );

            val
        }

        pub fn call_contract(&self, args: &CallContractArgs) -> HostPtr {
            let key = {
                // same default as borsh::to_vec uses
                let mut key = Vec::with_capacity(1024);
                key.push(CALL_FUNCTION_PREFIX);
                borsh::to_writer(&mut key, args).expect("failed to serialize call-contract args");
                key
            };

            let val = self.state.get(&key);

            assert!(
                !val.is_null(),
                "Call function not mocked. Please mock the function call."
            );

            val
        }

        pub fn get_balance(&self, args: &[u8]) -> HostPtr {
            // balance prefix + key
            let key = [BALANCE_PREFIX]
                .iter()
                .chain(args.iter())
                .copied()
                .collect::<Vec<u8>>();

            let host_ptr = self.state.get(&key);
            assert!(
                !host_ptr.is_null(),
                "get_balance not mocked. Please mock the function call."
            );

            host_ptr
        }

        pub fn set_balance(&self, account: Address, balance: u64) {
            let address_bytes = borsh::to_vec(&account).expect("failed to serialize");
            let key = [BALANCE_PREFIX]
                .iter()
                .chain(address_bytes.iter())
                .copied()
                .collect::<Vec<u8>>();

            let balance_bytes = borsh::to_vec(&balance).expect("failed to serialize");

            self.state.put(&key, balance_bytes);
        }

        pub fn get_remaining_fuel(&self) -> HostPtr {
            self.state().get_fuel()
        }

        pub fn send_value(&self, args: &[u8]) -> HostPtr {
            // send prefix + key
            let key = [SEND_PREFIX]
                .iter()
                .chain(args.iter())
                .copied()
                .collect::<Vec<u8>>();

            let host_ptr = self.state.get(&key);
            assert!(
                !host_ptr.is_null(),
                "send_value not mocked. Please mock the function call."
            );

            host_ptr
        }
    }

    impl Default for MockState {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Clone, Debug)]
    pub struct MockState {
        state: RefCell<hashbrown::HashMap<Vec<u8>, Vec<u8>>>,
        deploys: Cell<u8>,
        fuel: Gas,
    }

    impl MockState {
        pub fn new() -> Self {
            Self {
                state: RefCell::new(hashbrown::HashMap::new()),
                deploys: Cell::new(0),
                fuel: u64::MAX,
            }
        }

        pub fn get(&self, key: &[u8]) -> HostPtr {
            match self.state.borrow().get(key) {
                Some(val) => {
                    let ptr = crate::memory::alloc(val.len());
                    unsafe {
                        std::ptr::copy(val.as_ptr(), ptr.as_ptr().cast_mut(), val.len());
                    }
                    ptr
                }
                None => HostPtr::null(),
            }
        }

        pub fn put(&self, key: &[u8], value: Vec<u8>) {
            self.state.borrow_mut().insert(key.into(), value);
        }

        pub fn deploy(&self) -> u8 {
            self.deploys.set(self.deploys.get() + 1);
            self.deploys.get()
        }

        pub fn get_fuel(&self) -> HostPtr {
            let fuel_bytes = borsh::to_vec(&self.fuel).expect("failed to serialize");
            let ptr = crate::memory::alloc(fuel_bytes.len());
            unsafe {
                std::ptr::copy(
                    fuel_bytes.as_ptr(),
                    ptr.as_ptr().cast_mut(),
                    fuel_bytes.len(),
                );
            }
            ptr
        }
    }
}

#[cfg(not(feature = "test"))]
mod external_wrappers {
    use super::CallContractArgs;
    use crate::host::StateAccessor;
    use crate::memory::HostPtr;

    impl StateAccessor {
        #[inline]
        pub fn put(args: &[u8]) {
            #[link(wasm_import_module = "state")]
            extern "C" {
                #[link_name = "put"]
                fn put(ptr: *const u8, len: usize);
            }

            unsafe {
                put(args.as_ptr(), args.len());
            }
        }

        #[inline]
        pub fn get_bytes(args: &[u8]) -> HostPtr {
            #[link(wasm_import_module = "state")]
            extern "C" {
                #[link_name = "get"]
                fn get_bytes(ptr: *const u8, len: usize) -> u32;
            }

            let ptr = unsafe { get_bytes(args.as_ptr(), args.len()) };
            HostPtr::from_raw(ptr as *const u8)
        }
    }

    #[derive(Clone)]
    #[cfg_attr(feature = "debug", derive(Debug))]
    pub struct Accessor;

    impl Accessor {
        #![allow(clippy::unused_self)]

        pub fn new() -> Self {
            Accessor
        }

        #[inline]
        pub fn deploy(&self, args: &[u8]) -> HostPtr {
            use crate::memory::HostPtr;
            #[link(wasm_import_module = "contract")]
            extern "C" {
                #[link_name = "deploy"]
                fn deploy(ptr: *const u8, len: usize) -> u32;
            }

            let ptr = unsafe { deploy(args.as_ptr(), args.len()) };
            HostPtr::from_raw(ptr as *const u8)
        }

        #[inline]
        pub fn call_contract(&self, args: &CallContractArgs) -> HostPtr {
            #[link(wasm_import_module = "contract")]
            extern "C" {
                #[link_name = "call_contract"]
                fn call_contract(ptr: *const u8, len: usize) -> u32;
            }

            let args = borsh::to_vec(args).expect("failed to serialize args");

            let ptr = unsafe { call_contract(args.as_ptr(), args.len()) };
            HostPtr::from_raw(ptr as *const u8)
        }

        #[inline]
        pub fn get_balance(&self, args: &[u8]) -> HostPtr {
            #[link(wasm_import_module = "balance")]
            extern "C" {
                #[link_name = "get"]
                fn get(ptr: *const u8, len: usize) -> u32;
            }

            let ptr = unsafe { get(args.as_ptr(), args.len()) };
            HostPtr::from_raw(ptr as *const u8)
        }

        #[inline]
        pub fn get_remaining_fuel(&self) -> HostPtr {
            #[link(wasm_import_module = "contract")]
            extern "C" {
                #[link_name = "remaining_fuel"]
                fn get_remaining_fuel() -> u32;
            }

            let ptr = unsafe { get_remaining_fuel() };
            HostPtr::from_raw(ptr as *const u8)
        }

        #[inline]
        pub fn send_value(&self, args: &[u8]) -> HostPtr {
            #[link(wasm_import_module = "balance")]
            extern "C" {
                #[link_name = "send"]
                fn send_value(ptr: *const u8, len: usize) -> u32;
            }

            let ptr = unsafe { send_value(args.as_ptr(), args.len()) };
            HostPtr::from_raw(ptr as *const u8)
        }
    }
}
