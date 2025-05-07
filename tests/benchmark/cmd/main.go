// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/ava-labs/hypersdk/tests/benchmark"
)

func main() {
	// Command line flags
	benchmarkType := flag.String("type", "parallel", "Benchmark type: 'parallel' or 'stateless'")
	timeoutMinutes := flag.Int("timeout", 30, "Timeout in minutes for the benchmark")
	flag.Parse()

	// Validate benchmark type
	if *benchmarkType != "parallel" && *benchmarkType != "stateless" {
		fmt.Printf("Invalid benchmark type: %s. Must be 'parallel' or 'stateless'\n", *benchmarkType)
		os.Exit(1)
	}

	// Print header based on benchmark type
	if *benchmarkType == "parallel" {
		fmt.Println("=== HyperSDK Parallel Execution Benchmark ===")
		fmt.Printf("Starting parallel execution benchmark at %s\n", time.Now().Format(time.RFC3339))
		fmt.Println("Testing various batch sizes, thread counts, and contention rates...")
	} else {
		fmt.Println("=== HyperSDK Stateless System Benchmark ===")
		fmt.Printf("Starting stateless system benchmark at %s\n", time.Now().Format(time.RFC3339))
		fmt.Println("Testing full stateless architecture with witness validation, TEE simulation, and crypto verification...")
	}

	// Create a context with timeout
	ctx, cancel := context.WithTimeout(context.Background(), time.Duration(*timeoutMinutes)*time.Minute)
	defer cancel()

	// Run the selected benchmark with timeout
	done := make(chan error)
	go func() {
		var err error
		if *benchmarkType == "parallel" {
			err = benchmark.RunParallelBenchmarks()
		} else {
			err = benchmark.RunStatelessBenchmark()
		}
		done <- err
	}()

	// Wait for completion or timeout
	select {
	case err := <-done:
		if err != nil {
			fmt.Printf("Error running benchmarks: %v\n", err)
			os.Exit(1)
		}
		var resultsFile string
		if *benchmarkType == "parallel" {
			resultsFile = "parallel_benchmark_results.csv"
		} else {
			resultsFile = "stateless_system_benchmark_results.csv"
		}
		fmt.Println("Benchmarks completed successfully!")
		fmt.Printf("Results are available in %s\n", resultsFile)
	case <-ctx.Done():
		fmt.Printf("\nBenchmark timed out after %d minutes!\n", *timeoutMinutes)
		fmt.Println("This indicates a potential deadlock or infinite loop in the benchmark code.")
		os.Exit(1)
	}
}
