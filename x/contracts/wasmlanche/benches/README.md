# Wasmlanche Benchmarks

This directory contains benchmarks for measuring the performance characteristics of Wasmlanche contract operations.

## Benchmark Types

### Time Benchmarks (`call_contract_time.rs`)
Time benchmarks measure the execution time of various contract operations using the `criterion` framework. These benchmarks run on all platforms and measure:

- Basic function calls
- State operations (read/write/delete)
- Balance operations
- Event operations
- Schema operations
- Parallel operations with different scales (10/100/1000 operations)

To run time benchmarks:
```bash
cargo bench --bench call_contract_time
```

### Memory Benchmarks (`call_contract_mem.rs`)
Memory benchmarks measure memory usage and allocation patterns using the `iai` framework, which requires `valgrind`. These benchmarks **only run on Linux systems**.

#### Prerequisites for Memory Benchmarks
1. Linux operating system
2. Valgrind installed:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install valgrind

   # Fedora
   sudo dnf install valgrind

   # Arch Linux
   sudo pacman -S valgrind
   ```

To run memory benchmarks:
```bash
cargo bench --bench call_contract_mem
```

## Benchmark Coverage

Our benchmarks cover the following aspects of Wasmlanche:

1. Core Operations:
   - Function calls
   - NFT operations
   - Context creation and management

2. State Operations:
   - Reading state
   - Writing state
   - Deleting state
   - Schema operations

3. Balance Operations:
   - Getting balance
   - Setting balance

4. Event Operations:
   - Emitting events
   - Retrieving events

5. Parallel Operations:
   - Parallel reads
   - Parallel event emissions
   - Memory impact of parallel operations

## Understanding Results

### Time Benchmark Results
Time benchmarks output results in nanoseconds, showing:
- Mean execution time
- Standard deviation
- Sample size
- R² value (indicating reliability of measurements)

Example output:
```
call_contract/always_true    time:   [1.2345 µs 1.2456 µs 1.2567 µs]
```

### Memory Benchmark Results
Memory benchmarks (Linux only) show:
- Instructions executed
- L1/L2 cache accesses
- Memory allocations
- Peak memory usage

Example output:
```
call_contract_nft            instructions:   123,456
                            l1 accesses:     12,345
                            l2 accesses:     1,234
                            ram accesses:    123
                            estimated cycles: 234,567
```

## Best Practices

1. Run benchmarks on a quiet system with minimal background processes
2. Run multiple times to ensure consistent results
3. For memory benchmarks:
   - Use a Linux system with valgrind support
   - Run with sufficient memory available
   - Consider running with different heap configurations

## Troubleshooting

### Common Issues

1. Memory benchmarks failing on macOS:
   ```
   Unexpected error while launching valgrind
   ```
   **Solution**: Memory benchmarks require Linux with valgrind installed.

2. High variance in benchmark results:
   - Check for background processes
   - Try increasing the measurement time
   - Run benchmarks multiple times

## Contributing

When adding new benchmarks:
1. Add both time and memory benchmark variants
2. Follow the existing naming conventions
3. Document any special requirements or considerations
4. Update this README if adding new benchmark categories
