#[cfg(all(test, feature = "simulator"))]
mod tests {
    use wasmlanche::{
        SimulatorImpl,
        simulator::SimulatorExt,
    };
    use tokio::runtime::Runtime;

    #[test]
    fn test_async_produce() {
        let rt = Runtime::new().unwrap();
        
        rt.block_on(async {
            let mut simulator = SimulatorImpl::new().await;
            
            // Test the produce function
            let result = simulator.call_function::<_, i64>("produce", &[]).await;
            assert_eq!(result, Ok(42));
            
            // Verify state was set
            let value = simulator.get_state(b"shared_value").await;
            assert!(value.is_some());
            
            // Test the async produce function
            let result = simulator.call_function::<_, i64>("async_ops::produce_async", &[]).await;
            assert_eq!(result, Ok(1337));
            
            // Verify state was updated
            let value = simulator.get_state(b"shared_value").await;
            assert!(value.is_some());
        });
    }
}
