#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
mod tests {
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use wasmlanche::{
        context::WasmlAddress,
        future::AsyncResult,
        simulator::{Simulator, SimulatorExt, SimulatorImpl},
    };
    use borsh::{BorshDeserialize, BorshSerialize};

    #[derive(BorshSerialize, BorshDeserialize)]
    struct CallArgs {
        pub target: WasmlAddress,
        pub amount: u64,
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
    struct CallResult {
        pub success: bool,
        pub value: u64,
    }

    #[tokio::test]
    async fn test_cross_contract_calls() {
        // Create addresses for the contracts
        let caller_address = WasmlAddress::from_bytes(&[1u8; 32]).unwrap();
        let callee_address = WasmlAddress::from_bytes(&[2u8; 32]).unwrap();
        
        // Compile the contracts (simplified for test)
        let caller_wasm = include_bytes!("../../../target/wasm32-unknown-unknown/debug/cross_contract_caller.wasm");
        let callee_wasm = include_bytes!("../../../target/wasm32-unknown-unknown/debug/cross_contract_callee.wasm");
        
        // Create simulators for both contracts
        let mut caller_simulator = SimulatorImpl::new(caller_wasm, Arc::new(RwLock::new(Default::default()))).unwrap();
        let mut callee_simulator = SimulatorImpl::new(callee_wasm, Arc::new(RwLock::new(Default::default()))).unwrap();
        
        // Set initial balances
        caller_simulator.set_balance(&caller_address, 1000);
        callee_simulator.set_balance(&callee_address, 500);
        
        // Prepare call arguments
        let call_args = CallArgs {
            target: callee_address.clone(),
            amount: 42,
        };
        let serialized_args = call_args.try_to_vec().unwrap();
        
        // Execute the caller contract
        let result = caller_simulator.execute(
            &caller_address,
            &callee_address.to_bytes(),
            "make_cross_contract_call",
            &serialized_args,
            1000000,
        ).await.unwrap();
        
        // Deserialize and check the result
        let call_result = CallResult::try_from_slice(&result).unwrap();
        assert_eq!(call_result, CallResult { success: true, value: 84 }); // 42 * 2
        
        // Check that state was properly stored in the caller contract
        let last_called_result = caller_simulator.execute(
            &caller_address,
            &[],
            "get_last_called",
            &[],
            1000000,
        ).await.unwrap();
        
        let last_called = WasmlAddress::try_from_slice(&last_called_result).unwrap();
        assert_eq!(last_called, callee_address);
        
        // Check the callee contract state
        let amount_bytes = 42u64.to_le_bytes().to_vec();
        let last_request_result = callee_simulator.execute(
            &callee_address,
            &[],
            "get_last_request",
            &[],
            1000000,
        ).await.unwrap();
        
        let last_request = u64::from_le_bytes(last_request_result.try_into().unwrap());
        assert_eq!(last_request, 42);
    }
}
