#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
mod tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use wasmlanche::{
        context::WasmlAddress,
        future::{AsyncResult, StateResult, UnitResult},
        simulator::{SimulatorImpl, SimulatorExt},
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_async_state_operations() {
        // Create a simple simulator state
        let state = Arc::new(RwLock::new(HashMap::new()));
        
        // Create a simulator with an empty module
        let wasm = r#"
            (module
                (memory (export "memory") 1)
                (func (export "allocate") (param i32) (result i32)
                    i32.const 0  ;; Just return 0 for simplicity
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wasm).expect("Failed to parse WAT");
        
        let mut simulator = SimulatorImpl::new(&wasm_bytes, Arc::new(RwLock::new(Default::default())))
            .expect("Failed to create simulator");
        
        // Test addresses for our operations
        let address = WasmlAddress::from_bytes(&[1u8; 32]).unwrap();
        
        // Test 1: Store and retrieve state
        let key = b"test_key";
        let value = b"test_value";
        
        // Store the state
        simulator.store_state(key, value).await.expect("Failed to store state");
        
        // Retrieve the state
        let retrieved = simulator.get_state(key).await.expect("Failed to get state");
        assert_eq!(retrieved, Some(value.to_vec()));
        
        // Test 2: Delete state
        let removed = simulator.delete_state(key).await.expect("Failed to delete state");
        assert_eq!(removed, Some(value.to_vec()));
        
        // Verify it's gone
        let retrieved_after_delete = simulator.get_state(key).await.expect("Failed to get state");
        assert_eq!(retrieved_after_delete, None);
        
        // Test 3: Balance operations
        simulator.set_balance_async(&address, 1000).await;
        let balance = simulator.get_balance_async(&address).await;
        assert_eq!(balance, 1000);
    }
}
