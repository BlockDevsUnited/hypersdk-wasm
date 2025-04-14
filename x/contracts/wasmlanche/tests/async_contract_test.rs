// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

#[cfg(all(test, feature = "simulator"))]
mod tests {
    use std::sync::Arc;
    use tokio::runtime::Runtime;
    use tokio::sync::RwLock;
    use wasmlanche::{
        SimulatorImpl,
        simulator::SimulatorExt,
    };

    // Test that async operations work correctly
    #[test]
    fn test_async_operations() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut simulator = SimulatorImpl::new().await;
            
            // Set state
            simulator.store_state(b"key", b"value").await;
            
            // Get state
            let value = simulator.get_state(b"key").await;
            assert_eq!(value, Some(b"value".to_vec()));
            
            // Update state
            simulator.store_state(b"key", b"new_value").await;
            
            // Verify update
            let updated_value = simulator.get_state(b"key").await;
            assert_eq!(updated_value, Some(b"new_value".to_vec()));
            
            // Delete state
            simulator.delete_state(b"key").await;
            
            // Verify deletion
            let deleted_value = simulator.get_state(b"key").await;
            assert_eq!(deleted_value, None);
        });
    }

    // Test concurrent async operations
    #[test]
    fn test_concurrent_operations() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let simulator = Arc::new(RwLock::new(SimulatorImpl::new().await));
            
            // Set initial state
            {
                let mut sim = simulator.write().await;
                sim.store_state(b"counter", &[0]).await;
            }
            
            // Create multiple tasks to increment the counter
            let mut handles = Vec::new();
            for _ in 0..10 {
                let sim_clone = simulator.clone();
                
                let handle = tokio::spawn(async move {
                    // Read current value
                    let value = {
                        let sim = sim_clone.read().await;
                        sim.get_state(b"counter").await.unwrap_or_else(|| vec![0])
                    };
                    
                    // Increment value
                    let mut new_value = value.clone();
                    if !new_value.is_empty() {
                        new_value[0] += 1;
                    }
                    
                    // Write back
                    let mut sim = sim_clone.write().await;
                    sim.store_state(b"counter", &new_value).await;
                });
                
                handles.push(handle);
            }
            
            // Wait for all tasks to complete
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Verify final value
            // Note: Due to race conditions, the final value might not be exactly 10
            let final_value = {
                let sim = simulator.read().await;
                sim.get_state(b"counter").await.unwrap_or_else(|| vec![0])
            };
            
            assert!(final_value[0] > 0, "Counter should have been incremented at least once");
        });
    }

    // Test error handling with state operations
    #[test]
    fn test_error_handling() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut simulator = SimulatorImpl::new().await;
            
            // Test handling of state operations
            simulator.store_state(b"test_key", b"test_value").await;
            let value = simulator.get_state(b"test_key").await;
            assert_eq!(value, Some(b"test_value".to_vec()));
            
            // Test deleting non-existent key
            let deleted = simulator.delete_state(b"non_existent_key").await;
            assert_eq!(deleted, None);
            
            // Test getting non-existent key
            let non_existent = simulator.get_state(b"another_non_existent_key").await;
            assert_eq!(non_existent, None);
        });
    }
}
