#[cfg(all(test, feature = "std", feature = "simulator", not(target_arch = "wasm32")))]
mod tests {
    use borsh::{BorshDeserialize, BorshSerialize};
    use wasmlanche::{
        simulator::SimulatorImpl,
        simulator::SimulatorExt,
    };

    // Define a simple test structure
    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Clone)]
    struct TestValue {
        pub id: u64,
        pub name: String,
    }

    // Helper function to serialize a value
    fn serialize_value<T: BorshSerialize>(value: &T) -> Result<Vec<u8>, String> {
        borsh::BorshSerialize::try_to_vec(value).map_err(|e| format!("Serialization error: {}", e))
    }

    // Helper function to deserialize a value
    fn deserialize_value<T: BorshDeserialize>(data: &[u8]) -> Result<T, String> {
        borsh::BorshDeserialize::try_from_slice(data).map_err(|e| format!("Deserialization error: {}", e))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_non_blocking_operations() {
        println!("Creating simulator...");
        // Initialize simulator
        let mut simulator = SimulatorImpl::new().await;
        
        println!("Preparing test value...");
        // Create test value
        let test_value = TestValue {
            id: 42,
            name: "Test Item".to_string(),
        };
        
        // Serialize the value
        let serialized = serialize_value(&test_value).expect("Failed to serialize");
        
        println!("Storing value...");
        // Store the value in state storage (using the async version)
        simulator.store_state(b"test_key", &serialized).await;
        
        println!("Retrieving value...");
        // Retrieve the value (using the async version)
        let retrieved = simulator.get_state(b"test_key").await;
        
        // Verify retrieval
        assert!(retrieved.is_some(), "Failed to retrieve the stored value");
        
        // Deserialize and verify the value
        if let Some(bytes) = retrieved {
            let deserialized: TestValue = deserialize_value(&bytes).expect("Failed to deserialize");
            assert_eq!(deserialized, test_value, "Retrieved value doesn't match the original");
            println!("Deserialized value: {:?}", deserialized);
        }
        
        println!("Test completed successfully!");
    }
}
