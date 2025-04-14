# Wasmlanche Smart Contract SDK

This SDK provides a framework for writing WebAssembly-based smart contracts with asynchronous capabilities for the Hyper blockchain platform.

## Core Features

- **Async/Await Execution Model**: Write non-blocking contracts with natural async/await syntax
- **Cross-Contract Communication**: Call other contracts and await their results
- **Robust State Management**: Thread-safe, concurrent state operations
- **Error Handling**: Clear error propagation with Result types
- **TEE Integration**: Ready for Trusted Execution Environments (SGX, SEV)

## Getting Started

### Prerequisites

- Install Rust (https://www.rust-lang.org/tools/install)
- Add WebAssembly target: `rustup target add wasm32-unknown-unknown`

### Creating Your First Contract

1. Create a new Rust project:
```bash
cargo new --lib my_contract
cd my_contract
```

2. Configure your Cargo.toml:
```toml
[package]
name = "my_contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasmlanche = { path = "../../wasmlanche", default-features = false }
borsh = "0.10.0"
```

3. Write your contract in src/lib.rs:
```rust
use borsh::{BorshDeserialize, BorshSerialize};
use wasmlanche::{
    context::Context,
    future::AsyncResult,
    error::Error,
};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddArgs {
    pub a: i32,
    pub b: i32,
}

// Public async function - can be called from outside
pub async fn add(ctx: &mut Context, args: AddArgs) -> AsyncResult<i32> {
    // Log for debugging
    ctx.log(&format!("Adding {} + {}", args.a, args.b));
    
    // Store result in state
    let result = args.a + args.b;
    let key = b"last_result";
    
    match ctx.store_by_key(key, result.to_le_bytes().to_vec()) {
        Ok(_) => AsyncResult::with_result(Ok(result)),
        Err(e) => AsyncResult::with_result(Err(e)),
    }
}

// Required by the runtime
fn main() {}
```

4. Build your contract:
```bash
cargo build --target wasm32-unknown-unknown --release
```

## Async Programming Model

### Why Async Contracts?

- **Concurrent Execution**: Non-conflicting contracts execute in parallel
- **Cross-Contract Calls**: Make calls to other contracts without blocking
- **Improved Throughput**: Better resource utilization during I/O operations
- **Natural Error Handling**: Clear propagation with Result types

### Basic Async Patterns

```rust
// Define an async function
pub async fn my_function(ctx: &mut Context) -> AsyncResult<T> {
    // Your implementation
    AsyncResult::with_result(Ok(result))
}

// Call another contract asynchronously
pub async fn call_other(ctx: &mut Context, target: WasmlAddress) -> AsyncResult<Vec<u8>> {
    // The await keyword suspends execution until the call completes
    let result = ctx.call_contract(&target, "method_name", &args, None).await?;
    AsyncResult::with_result(Ok(result))
}
```

### State Management

```rust
// Store state asynchronously
ctx.store_by_key_async(key, value).await?;

// Retrieve state asynchronously
let value = ctx.get_by_key_async::<YourType>(key).await?;
```

## Parameter Handling

Wasmlanche supports two parameter passing patterns:

### 1. Length-Prefixed Format

```
[4 bytes length prefix][actual data]
```

- First 4 bytes are a little-endian u32 representing data length
- Common WebAssembly convention

### 2. Direct Data Format

```
[raw data without prefix]
```

- No length prefix, data passed directly
- Used primarily for fixed-size data like contract IDs

### Best Practices for Parameter Handling

Always implement robust parameter validation:

```rust
pub fn read_params(data: &[u8]) -> Vec<u8> {
    // Check if we have at least 4 bytes for length prefix
    if data.len() >= 4 {
        // Try reading as length-prefixed format
        let length_bytes = &data[0..4];
        let length = u32::from_le_bytes(length_bytes.try_into().unwrap_or([0; 4]));
        
        // Validate reasonable length (e.g., 0 < len <= 1024)
        if length > 0 && length <= 1024 && data.len() >= (4 + length as usize) {
            return data[4..(4 + length as usize)].to_vec();
        }
    }
    
    // If length prefix is invalid or not present, assume direct data format
    // (e.g., for 32-byte contract IDs)
    data.to_vec()
}
```

## Advanced Patterns

### Cross-Contract Communication

```rust
pub async fn make_cross_contract_call(
    ctx: &mut Context, 
    target: WasmlAddress,
    method: &str,
    args: Vec<u8>,
    timeout_ms: Option<u64>,
) -> AsyncResult<Vec<u8>> {
    let result = ctx.call_contract(&target, method, &args, timeout_ms).await?;
    AsyncResult::with_result(Ok(result))
}
```

### Parallel Execution

```rust
pub async fn parallel_operations(ctx: &mut Context, targets: Vec<TargetInfo>) -> AsyncResult<Vec<Result>> {
    let mut futures = Vec::new();
    let mut results = Vec::new();
    
    // Start multiple operations in parallel
    for target in targets {
        futures.push(ctx.call_contract(&target.address, &target.method, &target.args, target.timeout));
    }
    
    // Await each result
    for future in futures {
        let result = match future.await {
            Ok(data) => Result { success: true, data },
            Err(e) => Result { success: false, data: vec![] },
        };
        results.push(result);
    }
    
    AsyncResult::with_result(Ok(results))
}
```

### Error Handling

```rust
pub async fn safe_operation(ctx: &mut Context) -> AsyncResult<Vec<u8>> {
    // Using the ? operator with await
    let result = ctx.call_contract(&target, "method", &args, None).await?;
    
    // Or using match for more control
    let result = match ctx.call_contract(&target, "method", &args, None).await {
        Ok(data) => data,
        Err(e) => {
            ctx.log(&format!("Error: {}", e));
            return AsyncResult::with_result(Err(e));
        }
    };
    
    AsyncResult::with_result(Ok(result))
}
```

## Examples

Browse the `/examples` directory for complete contract examples:

- `cross_contract_caller.rs`: Basic cross-contract communication
- `advanced_cross_contract_caller.rs`: Sophisticated error handling and timeouts
- `advanced_cross_contract_callee.rs`: State management and parallel execution

## API Reference

For detailed API documentation, run:
```bash
cargo doc --open
