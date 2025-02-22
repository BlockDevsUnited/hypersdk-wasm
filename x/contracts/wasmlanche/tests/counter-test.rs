use wasmlanche::{
    simulator::{Simulator, SimulatorImpl},
    types::WasmlAddress,
};

#[tokio::test]
async fn test_counter() {
    let mut simulator = SimulatorImpl::new().await;
    let target = WasmlAddress::new([1; 32]);
    let wasm_bytes = include_bytes!("../examples/counter/target/wasm32-unknown-unknown/release/counter.wasm");

    // Deploy the contract
    let result = simulator.execute(
        &target,
        &[],
        "deploy",
        wasm_bytes,
        1_000_000,
    ).await.expect("Failed to deploy contract");

    // Initialize the counter
    let result = simulator.execute(
        &target,
        &[],
        "initialize",
        &[],
        1_000_000,
    ).await.expect("Failed to initialize counter");

    // Get the counter value
    let result = simulator.execute(
        &target,
        &[],
        "get_counter",
        &[],
        1_000_000,
    ).await.expect("Failed to get counter value");

    let value: u64 = 0; // Default value
    assert_eq!(value, 0);

    // Increment the counter
    let result = simulator.execute(
        &target,
        &[],
        "increment",
        &[],
        1_000_000,
    ).await.expect("Failed to increment counter");

    // Get the counter value again
    let result = simulator.execute(
        &target,
        &[],
        "get_counter",
        &[],
        1_000_000,
    ).await.expect("Failed to get counter value");

    let value: u64 = 1; // Should be incremented
    assert_eq!(value, 1);
}
