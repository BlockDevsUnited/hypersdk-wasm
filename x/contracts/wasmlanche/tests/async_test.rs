#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
mod tests {
    use wasmlanche::{
        types::WasmlAddress,
        simulator::{SimulatorImpl, SimulatorExt},
    };

    #[tokio::test]
    async fn test_async_state_operations() {
        // Create a simulator with default state - must be awaited
        let mut simulator = SimulatorImpl::new().await;
        
        // Test addresses for our operations
        let address = WasmlAddress::new([1u8; 32]);
        
        // Test 1: Store and retrieve state
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        
        // Store the state using the async methods from SimulatorExt
        // Important: Use trait methods directly, not through Simulator trait
        simulator.store_state(&key, &value).await;
        
        // Retrieve the state - must use async methods
        let retrieved = simulator.get_state(&key).await;
        assert_eq!(retrieved, Some(value.clone()));
        
        // Test 2: Delete state
        let removed = simulator.delete_state(&key).await;
        assert_eq!(removed, Some(value.to_vec()));
        
        // Verify it's gone
        let retrieved_after_delete = simulator.get_state(&key).await;
        assert_eq!(retrieved_after_delete, None);
        
        // Test 3: Balance operations
        simulator.set_balance_async(&address, 1000).await;
        let balance = simulator.get_balance_async(&address).await;
        assert_eq!(balance, 1000);
        
        // Don't call blocking methods like simulator.remaining_fuel()
        // Instead use async alternatives where available
    }
}
