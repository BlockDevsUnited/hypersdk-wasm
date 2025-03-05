#[cfg(all(test, feature = "std", feature = "simulator", not(target_arch = "wasm32")))]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use std::time::SystemTime;
    
    use wasmlanche::{
        types::WasmlAddress,
        simulator::SimulatorImpl,
        simulator::SimulatorExt,
        Simulator,
    };
    
    // Define cross-contract call metadata
    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
    struct CrossCallResult {
        pub success: bool,
        pub return_data: Vec<u8>,
        pub gas_used: u64,
        pub timestamp: u64,
    }

    // Mock contract implementation
    struct MockContract {
        simulator: SimulatorImpl,
    }

    impl MockContract {
        async fn new() -> Self {
            let simulator = SimulatorImpl::new().await;
            Self { simulator }
        }
        
        async fn store_value(&mut self, key: &[u8], value: u64) -> Result<(), wasmlanche::error::Error> {
            // Serialize the value
            let serialized = value.to_le_bytes().to_vec();
            
            // Store the value
            self.simulator.store_state(key, &serialized).await;
            
            Ok(())
        }
        
        async fn get_value(&self, key: &[u8]) -> Result<Option<u64>, wasmlanche::error::Error> {
            // Get the value from the state
            if let Some(bytes) = self.simulator.get_state(key).await {
                if bytes.len() >= 8 {
                    let mut buf = [0u8; 8];
                    buf.copy_from_slice(&bytes[0..8]);
                    return Ok(Some(u64::from_le_bytes(buf)));
                }
            }
            
            Ok(None)
        }
        
        fn get_address(&self) -> WasmlAddress {
            WasmlAddress::new([1u8; 32])
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_simple_contract_operations() {
        // Create a contract instance
        let mut contract = MockContract::new().await;
        
        // Test storing and retrieving a value
        contract.store_value(b"test_key", 42).await.expect("Failed to store value");
        
        // Read the value back
        let value = contract.get_value(b"test_key").await.expect("Failed to get value");
        
        // Verify the value
        assert_eq!(value, Some(42));
        
        // Test with a different value
        contract.store_value(b"another_key", 100).await.expect("Failed to store second value");
        let second_value = contract.get_value(b"another_key").await.expect("Failed to get second value");
        assert_eq!(second_value, Some(100));
        
        // Key that doesn't exist
        let missing_value = contract.get_value(b"missing_key").await.expect("Failed to query missing key");
        assert_eq!(missing_value, None);
    }
}
