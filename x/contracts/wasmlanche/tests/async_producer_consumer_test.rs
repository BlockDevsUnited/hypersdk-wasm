#[cfg(all(test, feature = "simulator"))]
mod tests {
    use tokio::runtime::Runtime;
    use wasmlanche::{
        SimulatorImpl,
        simulator::SimulatorExt,
        types::WasmlAddress,
    };

    // Test async state operations between producer and consumer
    #[test]
    fn test_async_producer_consumer() {
        // Create a new runtime
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            // Create a new simulator
            let mut simulator = SimulatorImpl::new().await;
            
            // Sample contract addresses
            let producer_addr = WasmlAddress::new([1; 32]);
            let consumer_addr = WasmlAddress::new([2; 32]);
            
            // Set up account balances
            simulator.set_balance_async(&producer_addr, 1000).await;
            simulator.set_balance_async(&consumer_addr, 1000).await;
            
            // Producer writes to state
            let shared_key = b"shared_value";
            let value: i32 = 42;
            let bytes = value.to_le_bytes().to_vec();
            
            // Simulate producer contract storing a value
            simulator.store_state(shared_key, &bytes).await;
            
            // Simulate consumer contract reading a value
            let result = simulator.get_state(shared_key).await;
            
            // Check if consumer fetched the correct value
            assert!(result.is_some(), "Consumer should be able to read producer's state");
            let bytes = result.unwrap();
            assert_eq!(bytes.len(), 4, "Value should be 4 bytes for i32");
            
            let mut array = [0u8; 4];
            array.copy_from_slice(&bytes[0..4]);
            let consumer_value = i32::from_le_bytes(array);
            
            assert_eq!(consumer_value, 42, "Consumer received wrong value");
        });
    }
}
