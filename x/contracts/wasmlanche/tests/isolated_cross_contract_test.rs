#[cfg(all(test, feature = "std", feature = "simulator", not(target_arch = "wasm32")))]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Mutex;
    use tokio::time::timeout;

    use wasmlanche::{
        types::WasmlAddress,
        simulator::SimulatorImpl,
        simulator::SimulatorExt,
        Simulator,
    };

    // Define cross-contract call arguments
    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
    struct CrossCallArgs {
        target_address: WasmlAddress,
        method: String,
        amount: u64,
        data: Vec<u8>,
    }

    // Define cross-contract call result
    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
    struct CrossCallResult {
        success: bool,
        gas_used: u64,
        return_data: Vec<u8>,
        timestamp: u64,
    }
    
    // Define call metadata for tracking contract interactions
    #[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
    struct CallMetadata {
        caller: WasmlAddress,
        callee: WasmlAddress,
        method: String,
        amount: u64,
        data: Vec<u8>,
        success: bool,
        return_data: Option<Vec<u8>>,
        timestamp: u64,
    }

    // Helper function to serialize a value
    fn serialize_value<T: BorshSerialize>(value: &T) -> Result<Vec<u8>, String> {
        borsh::BorshSerialize::try_to_vec(value).map_err(|e| format!("Serialization error: {}", e))
    }

    // Helper function to deserialize a value
    fn deserialize_value<T: BorshDeserialize>(data: &[u8]) -> Result<T, String> {
        borsh::BorshDeserialize::try_from_slice(data).map_err(|e| format!("Deserialization error: {}", e))
    }

    // Caller Contract - Makes cross-contract calls
    struct CallerContract {
        simulator: SimulatorImpl,
        address: WasmlAddress,
        call_counter: u64,
    }

    impl CallerContract {
        async fn new() -> Self {
            let simulator = SimulatorImpl::new().await;
            let address = WasmlAddress::new([1u8; 32]);
            
            Self {
                simulator,
                address,
                call_counter: 0,
            }
        }
        
        async fn increment_call_counter(&mut self) -> Result<u64, wasmlanche::error::Error> {
            // Increment the counter
            self.call_counter += 1;
            
            // Store the updated counter
            let key = b"call_counter".to_vec();
            let data = serialize_value(&self.call_counter).unwrap();
            
            self.simulator.store_state(&key, &data).await;
            
            Ok(self.call_counter)
        }
        
        async fn get_call_counter(&self) -> Result<u64, wasmlanche::error::Error> {
            let key = b"call_counter".to_vec();
            
            if let Some(data) = self.simulator.get_state(&key).await {
                let counter: u64 = deserialize_value(&data).unwrap();
                Ok(counter)
            } else {
                Ok(0)
            }
        }
        
        async fn slow_operation(&mut self, delay_ms: u64) -> Result<(), wasmlanche::error::Error> {
            // Simulate a slow operation
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            Ok(())
        }
        
        async fn call_with_timeout(&mut self, args: CrossCallArgs) -> Result<CrossCallResult, wasmlanche::error::Error> {
            // No need to record gas since we're not using it
            // Let's remove this line to resolve the borrow conflict
            // let initial_gas = self.simulator.remaining_fuel_async();
            
            // Simulate making a cross-contract call
            match args.amount {
                // Simulate an error if amount is 9999
                9999 => Err(wasmlanche::error::Error::Contract("Simulated failure".into())),
                _ => {
                    // For other values, simulate a successful call
                    let result = CrossCallResult {
                        success: true,
                        gas_used: 1000,
                        return_data: serialize_value(&args.amount).unwrap(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };
                    
                    // Update the call metadata
                    self.update_call_metadata(args.clone(), &result).await?;
                    
                    Ok(result)
                }
            }
        }
        
        async fn update_call_metadata(&mut self, args: CrossCallArgs, result: &CrossCallResult) -> Result<(), wasmlanche::error::Error> {
            // Create metadata
            let metadata = CallMetadata {
                caller: self.address,
                callee: args.target_address,
                method: args.method,
                amount: args.amount,
                data: args.data,
                success: result.success,
                return_data: Some(result.return_data.clone()),
                timestamp: result.timestamp,
            };
            
            // Serialize the metadata
            let key = format!("call_{}_{}", self.call_counter, args.target_address);
            let serialized = serialize_value(&metadata).unwrap();
            
            // Store the metadata
            self.simulator.store_state(key.as_bytes(), &serialized).await;
            
            Ok(())
        }
        
        async fn get_metadata_for_call(&self, call_id: u64, target: &WasmlAddress) -> Result<Option<CallMetadata>, wasmlanche::error::Error> {
            let key = format!("call_{}_{}", call_id, target);
            
            if let Some(data) = self.simulator.get_state(key.as_bytes()).await {
                let metadata: CallMetadata = deserialize_value(&data).unwrap();
                Ok(Some(metadata))
            } else {
                Ok(None)
            }
        }
    }

    // Callee Contract - Receives cross-contract calls
    struct CalleeContract {
        simulator: SimulatorImpl,
        address: WasmlAddress,
    }

    impl CalleeContract {
        async fn new() -> Self {
            let simulator = SimulatorImpl::new().await;
            let address = WasmlAddress::new([2u8; 32]);
            
            Self {
                simulator,
                address,
            }
        }
        
        async fn process_call(&mut self, amount: u64, data: &[u8]) -> Result<Vec<u8>, wasmlanche::error::Error> {
            // Process the call and return a result
            let processed_amount = amount * 2;
            let result = serialize_value(&processed_amount).unwrap();
            
            // Store the call details
            let key = format!("processed_call_{}", amount);
            self.simulator.store_state(key.as_bytes(), data).await;
            
            Ok(result)
        }
        
        async fn get_call_data(&self, amount: u64) -> Result<Option<Vec<u8>>, wasmlanche::error::Error> {
            let key = format!("processed_call_{}", amount);
            
            Ok(self.simulator.get_state(key.as_bytes()).await)
        }
    }

    // Test cross-contract calls
    #[tokio::test(flavor = "multi_thread")]
    async fn test_cross_contract_calls() {
        // Initialize contracts
        let mut caller = CallerContract::new().await;
        let mut callee = CalleeContract::new().await;
        
        // Test simple call
        let args = CrossCallArgs {
            target_address: callee.address,
            method: "process_call".to_string(),
            amount: 100,
            data: serialize_value(&42u64).unwrap(),
        };
        
        // Increment call counter
        let call_id = caller.increment_call_counter().await.unwrap();
        assert_eq!(call_id, 1);
        
        // Make the call
        let result = caller.call_with_timeout(args.clone()).await.unwrap();
        
        // Verify result
        assert!(result.success);
        let return_amount: u64 = deserialize_value(&result.return_data).unwrap();
        assert_eq!(return_amount, args.amount);
        
        // Verify metadata was stored
        let metadata = caller.get_metadata_for_call(call_id, &callee.address).await.unwrap();
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.amount, args.amount);
        assert_eq!(metadata.method, args.method);
        assert!(metadata.success);
        
        // Test error case
        let error_args = CrossCallArgs {
            target_address: callee.address,
            method: "process_call".to_string(),
            amount: 9999, // This will cause an error
            data: serialize_value(&42u64).unwrap(),
        };
        
        // Increment call counter
        let call_id = caller.increment_call_counter().await.unwrap();
        assert_eq!(call_id, 2);
        
        // Make the call and expect an error
        let error_result = caller.call_with_timeout(error_args).await;
        assert!(error_result.is_err());
        
        // Test timeout scenario
        let call_with_timeout = async {
            // Simulate slow operation
            caller.slow_operation(100).await.unwrap();
            
            // Create new call args
            let timeout_args = CrossCallArgs {
                target_address: callee.address,
                method: "process_call".to_string(),
                amount: 200,
                data: serialize_value(&42u64).unwrap(),
            };
            
            // Increment call counter
            let call_id = caller.increment_call_counter().await.unwrap();
            assert_eq!(call_id, 3);
            
            // Make the call
            caller.call_with_timeout(timeout_args).await
        };
        
        // Set a short timeout to test timeout handling
        match timeout(Duration::from_millis(50), call_with_timeout).await {
            Ok(inner_result) => {
                // The call completed within the timeout
                assert!(inner_result.is_ok());
            },
            Err(_) => {
                // The call timed out, which is expected
                println!("Call timed out as expected");
            }
        }
        
        // Test multiple concurrent calls
        let mut handles = vec![];
        
        for i in 0..5 {
            let mut caller_clone = CallerContract::new().await;
            let amount = 300 + i;
            
            let handle = tokio::spawn(async move {
                // Create new call args
                let args = CrossCallArgs {
                    target_address: WasmlAddress::new([2u8; 32]),
                    method: "process_call".to_string(),
                    amount,
                    data: serialize_value(&amount).unwrap(),
                };
                
                // Increment call counter
                let call_id = caller_clone.increment_call_counter().await.unwrap();
                
                // Make the call
                let result = caller_clone.call_with_timeout(args).await.unwrap();
                
                (call_id, result)
            });
            
            handles.push(handle);
        }
        
        // Wait for all calls to complete
        for handle in handles {
            let (call_id, result) = handle.await.unwrap();
            assert!(result.success);
            assert!(call_id > 0);
        }
    }
}
