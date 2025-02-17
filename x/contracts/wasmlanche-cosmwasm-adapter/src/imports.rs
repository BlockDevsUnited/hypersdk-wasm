use anyhow::Result;
use cosmwasm_std::{Api, CanonicalAddr, ContractResult, Querier, Storage, SystemResult};
use wasmtime::{Caller, Linker, Module, Store};

use crate::host::HostEnv;

fn read_region<S, A, Q>(
    caller: &mut Caller<'_, HostEnv<S, A, Q>>,
    ptr: u32,
) -> Result<Vec<u8>>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    let memory = caller.get_export("memory")
        .ok_or_else(|| anyhow::anyhow!("no memory export"))?.into_memory()
        .ok_or_else(|| anyhow::anyhow!("export is not memory"))?;

    // Read length prefix (4 bytes)
    let mut len_bytes = [0u8; 4];
    memory.read(&caller, ptr as usize, &mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes);

    // Read the actual data
    let mut data = vec![0u8; len as usize];
    memory.read(&caller, (ptr + 4) as usize, &mut data)?;
    Ok(data)
}

fn write_region<S, A, Q>(
    caller: &mut Caller<'_, HostEnv<S, A, Q>>,
    ptr: u32,
    data: &[u8],
) -> Result<i32>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    let len = data.len() as u32;
    let memory = caller.get_export("memory")
        .ok_or_else(|| anyhow::anyhow!("no memory export"))?.into_memory()
        .ok_or_else(|| anyhow::anyhow!("export is not memory"))?;
    
    // Write length prefix
    memory.write(&mut *caller, ptr as usize, &len.to_be_bytes())?;
    
    // Write data
    memory.write(&mut *caller, (ptr + 4) as usize, data)?;
    
    Ok(0)
}

pub fn define_imports<S, A, Q>(
    linker: &mut Linker<HostEnv<S, A, Q>>,
    store: &mut Store<HostEnv<S, A, Q>>,
    module: Module,
) -> Result<()>
where
    S: Storage + Clone + 'static,
    A: Api + Clone + 'static,
    Q: Querier + Clone + 'static,
{
    // Debug function
    linker.func_wrap("env", "debug",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, message_ptr: i32| -> Result<()> {
            let message = read_region(&mut _caller, message_ptr as u32)?;
            let message_str = String::from_utf8(message)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in debug message: {}", e))?;
            println!("Debug: {}", message_str);
            Ok(())
        }
    )?;

    // Abort function
    linker.func_wrap("env", "abort",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, message_ptr: i32| -> Result<()> {
            let message = read_region(&mut _caller, message_ptr as u32)?;
            let message_str = String::from_utf8(message)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in abort message: {}", e))?;
            anyhow::bail!("Contract aborted: {}", message_str);
        }
    )?;

    // Storage functions
    linker.func_wrap("env", "db_read", 
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, key_ptr: u32| -> Result<u32> {
            let key = read_region(&mut _caller, key_ptr)?;
            let data = _caller.data().storage.get(&key);
            match data {
                Some(value) => {
                    let output_ptr = _caller.data().next_ptr.borrow().checked_add(8)
                        .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
                    write_region(&mut _caller, output_ptr, &value)?;
                    Ok(output_ptr)
                },
                None => Ok(0), // Return 0 for non-existent keys
            }
        }
    )?;

    linker.func_wrap("env", "db_write",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, key_ptr: u32, value_ptr: u32| -> Result<()> {
            let key = read_region(&mut _caller, key_ptr)?;
            let value = read_region(&mut _caller, value_ptr)?;
            _caller.data_mut().storage.set(&key, &value);
            Ok(())
        }
    )?;

    linker.func_wrap("env", "db_remove",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, key_ptr: u32| -> Result<()> {
            let key = read_region(&mut _caller, key_ptr)?;
            _caller.data_mut().storage.remove(&key);
            Ok(())
        }
    )?;

    // Add db_scan function
    linker.func_wrap("env", "db_scan",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, _start_ptr: u32, _end_ptr: u32, _order: i32| -> Result<u32> {
            // For now, return 0 since we don't support scanning yet
            Ok(0)
        }
    )?;

    // Add db_next function
    linker.func_wrap("env", "db_next",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, _iterator_id: u32| -> Result<u32> {
            // For now, return 0 since we don't support iteration yet
            Ok(0)
        }
    )?;

    // Address functions
    linker.func_wrap("env", "addr_validate",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, addr_ptr: i32| -> Result<i32> {
            let addr_raw = read_region(&mut _caller, addr_ptr as u32)?;
            let addr_str = String::from_utf8(addr_raw)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in address: {}", e))?;
            match _caller.data().api.addr_validate(&addr_str) {
                Ok(_) => Ok(0),
                Err(_) => Ok(1),
            }
        }
    )?;

    linker.func_wrap("env", "addr_canonicalize",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, addr_ptr: i32, canonical_ptr: i32| -> Result<i32> {
            let addr_raw = read_region(&mut _caller, addr_ptr as u32)?;
            let addr_str = String::from_utf8(addr_raw)
                .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in address: {}", e))?;
            match _caller.data().api.addr_canonicalize(&addr_str) {
                Ok(canon_addr) => {
                    // Write the canonical address to the provided pointer
                    let canon_bytes = canon_addr.as_slice();
                    write_region(&mut _caller, canonical_ptr as u32, canon_bytes)?;
                    Ok(0)
                },
                Err(_) => Ok(1),
            }
        }
    )?;

    linker.func_wrap("env", "addr_humanize",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, canonical_ptr: i32, human_ptr: i32| -> Result<i32> {
            let canon_addr = read_region(&mut _caller, canonical_ptr as u32)?;
            let canonical = CanonicalAddr::from(canon_addr);
            match _caller.data().api.addr_humanize(&canonical) {
                Ok(human_addr) => {
                    // Write the human address to the provided pointer
                    write_region(&mut _caller, human_ptr as u32, human_addr.as_str().as_bytes())?;
                    Ok(0)
                },
                Err(_) => Ok(1),
            }
        }
    )?;

    // Crypto functions
    linker.func_wrap("env", "secp256k1_verify",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, hash_ptr: i32, sig_ptr: i32, pubkey_ptr: i32| -> Result<i32> {
            let hash = read_region(&mut _caller, hash_ptr as u32)?;
            let sig = read_region(&mut _caller, sig_ptr as u32)?;
            let pubkey = read_region(&mut _caller, pubkey_ptr as u32)?;
            match _caller.data().api.secp256k1_verify(&hash, &sig, &pubkey) {
                Ok(true) => Ok(0),
                Ok(false) => Ok(1),
                Err(_) => Ok(2),
            }
        }
    )?;

    linker.func_wrap("env", "secp256k1_recover_pubkey",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, hash_ptr: u32, sig_ptr: u32, recovery_param: u32| -> Result<u64> {
            let hash = read_region(&mut _caller, hash_ptr)?;
            let sig = read_region(&mut _caller, sig_ptr)?;
            match _caller.data().api.secp256k1_recover_pubkey(&hash, &sig, recovery_param as u8) {
                Ok(pubkey) => {
                    let output_ptr = _caller.data().next_ptr.borrow().checked_add(8)
                        .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
                    write_region(&mut _caller, output_ptr, &pubkey)?;
                    Ok(((output_ptr as u64) << 32) | (pubkey.len() as u64))
                },
                Err(_) => Ok(0),
            }
        }
    )?;

    linker.func_wrap("env", "ed25519_verify",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, msg_ptr: i32, sig_ptr: i32, pubkey_ptr: i32| -> Result<i32> {
            let msg = read_region(&mut _caller, msg_ptr as u32)?;
            let sig = read_region(&mut _caller, sig_ptr as u32)?;
            let pubkey = read_region(&mut _caller, pubkey_ptr as u32)?;
            match _caller.data().api.ed25519_verify(&msg, &sig, &pubkey) {
                Ok(true) => Ok(0),
                Ok(false) => Ok(1),
                Err(_) => Ok(2),
            }
        }
    )?;

    linker.func_wrap("env", "ed25519_batch_verify",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, messages_ptr: i32, signatures_ptr: i32, public_keys_ptr: i32| -> Result<i32> {
            let messages = read_region(&mut _caller, messages_ptr as u32)?;
            let signatures = read_region(&mut _caller, signatures_ptr as u32)?;
            let public_keys = read_region(&mut _caller, public_keys_ptr as u32)?;
            
            // Split the input data into slices of slices
            let messages_slices: Vec<&[u8]> = messages.split(|&x| x == 0).collect();
            let signatures_slices: Vec<&[u8]> = signatures.split(|&x| x == 0).collect();
            let public_keys_slices: Vec<&[u8]> = public_keys.split(|&x| x == 0).collect();
            
            match _caller.data().api.ed25519_batch_verify(&messages_slices, &signatures_slices, &public_keys_slices) {
                Ok(true) => Ok(0),
                Ok(false) => Ok(1),
                Err(_) => Ok(2),
            }
        }
    )?;

    // Query function
    linker.func_wrap("env", "query_chain",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, query_ptr: u32| -> Result<u32> {
            let query_raw = read_region(&mut _caller, query_ptr)?;
            let querier_result = _caller.data().querier.raw_query(&query_raw);
            match querier_result {
                SystemResult::Ok(contract_result) => {
                    match contract_result {
                        ContractResult::Ok(binary) => {
                            let output_ptr = _caller.data().next_ptr.borrow().checked_add(8)
                                .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
                            write_region(&mut _caller, output_ptr, binary.as_slice())?;
                            Ok(output_ptr)
                        },
                        ContractResult::Err(err) => {
                            let output_ptr = _caller.data().next_ptr.borrow().checked_add(8)
                                .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;
                            write_region(&mut _caller, output_ptr, err.as_bytes())?;
                            Ok(output_ptr)
                        }
                    }
                },
                SystemResult::Err(_) => Ok(0),
            }
        }
    )?;

    // Memory functions
    linker.func_wrap("env", "allocate",
        |mut caller: Caller<'_, HostEnv<S, A, Q>>, size: u32| -> Result<u32> {
            let memory = caller.get_export("memory")
                .ok_or_else(|| anyhow::anyhow!("no memory export"))?.into_memory()
                .ok_or_else(|| anyhow::anyhow!("export is not memory"))?;
    
            // Allocate memory starting from 64KB to avoid conflicts with other regions
            let mut next_ptr = caller.data().next_ptr.borrow_mut();
            let ptr = *next_ptr;
            
            // Calculate total size needed
            let total_size = size;
            
            *next_ptr = next_ptr.checked_add(total_size)
                .ok_or_else(|| anyhow::anyhow!("Memory size overflow"))?;

            // Ensure we have enough memory
            let required_pages = (u64::from(*next_ptr) + 65535) / 65536;
            let current_pages = memory.size(&caller);
            drop(next_ptr); // Release the borrow
            
            if required_pages > current_pages {
                memory.grow(&mut caller, required_pages - current_pages)?;
            }

            // Initialize memory region with zeros
            let data = vec![0u8; total_size as usize];
            memory.write(&mut caller, ptr as usize, &data)?;

            Ok(ptr)
        }
    )?;

    linker.func_wrap("env", "deallocate",
        |mut _caller: Caller<'_, HostEnv<S, A, Q>>, ptr: u32| -> Result<()> {
            // For now, we don't actually deallocate memory since we're using a simple bump allocator
            Ok(())
        }
    )?;

    Ok(())
}
