use wasmlanche::{
    simulator::{Simulator, SimulatorImpl},
    types::WasmlAddress,
};

#[tokio::test]
async fn test_counter() {
    let mut simulator = SimulatorImpl::new().await.expect("Failed to create simulator");
    let target = WasmlAddress::new([1; 32]);
    let wasm_bytes = include_bytes!("../examples/counter/target/wasm32-unknown-unknown/release/counter.wasm");

    // Deploy the contract
    let _result = simulator.execute(
        &target,
        &[],
        "deploy",
        wasm_bytes,
        0,
    ).await;

    // Initialize the counter
    let _result = simulator.execute(
        &target,
        &[],
        "initialize",
        &[],
        0,
    ).await;

    // Increment the counter
    let _result = simulator.execute(
        &target,
        &[],
        "increment",
        &[],
        0,
    ).await;

    // Increment the counter
    let _result = simulator.execute(
        &target,
        &[],
        "increment",
        &[],
        0,
    ).await;

    // Increment the counter
    let _result = simulator.execute(
        &target,
        &[],
        "increment",
        &[],
        0,
    ).await;

    // Decrement the counter
    let _result = simulator.execute(
        &target,
        &[],
        "decrement",
        &[],
        0,
    ).await;

    // Get the counter value
    let _result = simulator.execute(
        &target,
        &[],
        "get_count",
        &[],
        0,
    ).await;
}
