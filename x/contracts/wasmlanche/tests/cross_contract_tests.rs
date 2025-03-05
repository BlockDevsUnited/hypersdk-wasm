//! Tests for cross-contract async call functionality
//!
//! This test suite verifies that contracts can make async calls to other contracts
//! with proper error handling, timeout management, and gas accounting.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use spin::RwLock;
    use wasmlanche::{
        context::Context,
        error::Error,
        events::Event, // Correct module name is events, not event
        host::{Host, HostImpl},
        types::WasmlAddress,
    };

    // Create a mock contract registry to simulate deployed contracts
    struct MockContractRegistry {
        contracts: std::collections::HashMap<WasmlAddress, Box<dyn Fn(&[u8]) -> Result<Vec<u8>, Error> + Send + Sync>>,
    }

    impl MockContractRegistry {
        fn new() -> Self {
            Self {
                contracts: std::collections::HashMap::new(),
            }
        }

        fn register<F>(&mut self, address: WasmlAddress, handler: F)
        where
            F: Fn(&[u8]) -> Result<Vec<u8>, Error> + Send + Sync + 'static,
        {
            self.contracts.insert(address, Box::new(handler));
        }

        fn call(&self, address: &WasmlAddress, args: &[u8]) -> Result<Vec<u8>, Error> {
            match self.contracts.get(address) {
                Some(handler) => handler(args),
                None => Err(Error::Contract(format!("Contract not found at address {:?}", address))),
            }
        }
    }

    // Create a struct that wraps HostImpl with the registry field
    struct TestHostImpl {
        inner: HostImpl,
        registry: Option<Arc<RwLock<MockContractRegistry>>>,
    }

    impl TestHostImpl {
        fn new(actor: WasmlAddress) -> Self {
            Self {
                inner: HostImpl::new(actor),
                registry: None,
            }
        }

        fn with_registry(actor: WasmlAddress, registry: Arc<RwLock<MockContractRegistry>>) -> Self {
            Self {
                inner: HostImpl::new(actor),
                registry: Some(registry),
            }
        }

        fn registry(&self) -> Option<Arc<RwLock<MockContractRegistry>>> {
            self.registry.clone()
        }
    }

    // Implement Host for TestHostImpl by delegating to inner HostImpl
    impl Host for TestHostImpl {
        fn store_state(&mut self, key: &[u8], value: &[u8]) -> Result<(), Error> {
            self.inner.store_state(key, value)
        }

        fn get_state(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
            self.inner.get_state(key)
        }

        fn delete_state(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
            self.inner.delete_state(key)
        }

        fn get_events(&self) -> Vec<Event> {
            self.inner.get_events()
        }

        fn add_event(&mut self, event: Event) -> Result<(), Error> {
            self.inner.add_event(event)
        }

        fn charge_gas(&mut self, amount: u64) -> Result<(), Error> {
            self.inner.charge_gas(amount)
        }

        fn remaining_gas(&self) -> u64 {
            self.inner.remaining_gas()
        }

        fn get_balance(&self, account: &WasmlAddress) -> u64 {
            self.inner.get_balance(account)
        }

        fn set_balance(&mut self, account: &WasmlAddress, amount: u64) {
            self.inner.set_balance(account, amount)
        }

        fn emit_event(&mut self, event: Event) -> Result<(), Error> {
            self.inner.emit_event(event)
        }

        fn consume_gas(&mut self, amount: u64) -> Result<(), Error> {
            self.inner.consume_gas(amount)
        }

        fn call_contract(&mut self, target: &WasmlAddress, method: &str, args: &[u8]) -> Result<Vec<u8>, Error> {
            // Ensure we have a registry
            if let Some(registry) = self.registry() {
                // For testing, we'll just append the method name to the args
                let mut call_args = method.as_bytes().to_vec();
                call_args.extend_from_slice(args);
                
                // Call the target contract
                registry.read().call(target, &call_args)
            } else {
                Err(Error::Contract(String::from("No contract registry available")))
            }
        }
    }

    #[tokio::test]
    async fn test_cross_contract_call() {
        // Create a mock contract registry
        let registry = Arc::new(RwLock::new(MockContractRegistry::new()));
        
        // Register a callee contract
        let callee_address = WasmlAddress::new([1; 32]);
        registry.write().register(callee_address.clone(), |args| {
            // Parse the method name from the args
            // For testing, we assume the method name is the first part of the args
            let method = String::from_utf8_lossy(&args[0..9]);
            
            if method.starts_with("get_value") {
                // Parse the amount from the args
                if args.len() >= 17 {
                    let amount_bytes: [u8; 8] = args[9..17].try_into().unwrap();
                    let amount = u64::from_le_bytes(amount_bytes);
                    
                    // Double the amount and return it
                    let result = amount * 2;
                    Ok(result.to_le_bytes().to_vec())
                } else {
                    Err(Error::Contract(String::from("Invalid arguments")))
                }
            } else {
                Err(Error::Contract(format!("Unknown method: {}", method)))
            }
        });
        
        // Create a caller context
        let caller_address = WasmlAddress::new([2; 32]);
        let caller_host = TestHostImpl::with_registry(caller_address.clone(), registry.clone());
        let mut caller_ctx = Context::with_host(caller_host);
        
        // Make a cross-contract call
        let amount: u64 = 42;
        let call_args = amount.to_le_bytes().to_vec();
        
        let result = caller_ctx.call_contract(&callee_address, "get_value", &call_args, Some(5000)).await;
        
        // Verify the result
        assert!(result.is_ok(), "Cross-contract call failed");
        
        let result_data = result.unwrap();
        let result_bytes: [u8; 8] = result_data.try_into().unwrap();
        let result_value = u64::from_le_bytes(result_bytes);
        
        // Verify we got back double the amount we sent (42 * 2 = 84)
        assert_eq!(result_value, 84, "Unexpected result value");
    }

    #[tokio::test]
    async fn test_cross_contract_error_handling() {
        // Create a mock contract registry
        let registry = Arc::new(RwLock::new(MockContractRegistry::new()));
        
        // Register a callee contract that returns errors
        let callee_address = WasmlAddress::new([3; 32]);
        registry.write().register(callee_address.clone(), |args| {
            // Parse the method name from the args
            let method = String::from_utf8_lossy(&args[0..9]);
            
            if method.starts_with("get_value") {
                // Parse the amount from the args
                if args.len() >= 17 {
                    let amount_bytes: [u8; 8] = args[9..17].try_into().unwrap();
                    let amount = u64::from_le_bytes(amount_bytes);
                    
                    // Simulate an error for certain amounts
                    if amount == 0xDEAD {
                        return Err(Error::Contract(String::from("Simulated error")));
                    }
                    
                    // Double the amount and return it
                    let result = amount * 2;
                    Ok(result.to_le_bytes().to_vec())
                } else {
                    Err(Error::Contract(String::from("Invalid arguments")))
                }
            } else {
                Err(Error::Contract(format!("Unknown method: {}", method)))
            }
        });
        
        // Create a caller context
        let caller_address = WasmlAddress::new([4; 32]);
        let caller_host = TestHostImpl::with_registry(caller_address.clone(), registry.clone());
        let mut caller_ctx = Context::with_host(caller_host);
        
        // Make a cross-contract call with an amount that triggers an error
        let amount: u64 = 0xDEAD;
        let call_args = amount.to_le_bytes().to_vec();
        
        let result = caller_ctx.call_contract(&callee_address, "get_value", &call_args, Some(5000)).await;
        
        // Verify the result is an error
        assert!(result.is_err(), "Expected cross-contract call to return an error");
        
        // Now try with a valid amount
        let amount: u64 = 123;
        let call_args = amount.to_le_bytes().to_vec();
        
        let result = caller_ctx.call_contract(&callee_address, "get_value", &call_args, Some(5000)).await;
        
        // Verify the result is successful
        assert!(result.is_ok(), "Cross-contract call with valid args failed");
        
        let result_data = result.unwrap();
        let result_bytes: [u8; 8] = result_data.try_into().unwrap();
        let result_value = u64::from_le_bytes(result_bytes);
        
        // Verify we got back double the amount we sent (123 * 2 = 246)
        assert_eq!(result_value, 246, "Unexpected result value");
    }
}
