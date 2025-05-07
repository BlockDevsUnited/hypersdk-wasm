// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package benchmark

import (
	"context"
	"fmt"
	"os"
	"sort"
	"time"

	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/ava-labs/hypersdk/chain"
	"go.uber.org/zap"
)

// RunParallelBenchmarks runs a comprehensive set of benchmarks to measure parallel execution performance
func RunParallelBenchmarks() error {
	// Setup logger
	// Using a simple console logger for benchmarking purposes
	logFactory := logging.NewFactory(logging.Config{
		DisplayLevel: logging.Info,
	})
	logger, err := logFactory.Make("ParallelExecBench")
	if err != nil {
		return fmt.Errorf("failed to create logger: %w", err)
	}

	// Create a comprehensive benchmark configuration
	logger.Info("Setting up comprehensive benchmark configuration")
	config := BenchmarkConfig{
		BatchSizes:       []int{100, 500, 1000, 2000},
		ThreadCounts:     []int{1, 4, 8, 16},
		TransactionCount: 10000,
		WarmupRounds:     1,
		BenchmarkRounds:  3,
		StateContentions: []float64{0.0, 0.25, 0.5, 0.75},
	}
	
	// Log the benchmark parameters
	logger.Info("Benchmark parameters", 
		zap.Ints("batchSizes", config.BatchSizes),
		zap.Ints("threadCounts", config.ThreadCounts),
		zap.Float64s("contentionRates", config.StateContentions))

	logger.Info("Benchmark configuration")

	// Create a regular executor and a parallel executor for comparison
	// Note: These are simplified - in real benchmarking we would populate
	// with appropriate configuration based on your actual implementation
	parallelConfig := chain.DefaultParallelConfig()
	parallelExecutor := chain.NewParallelExecutor(logger, parallelConfig, false)
	
	// Start the executor
	parallelExecutor.Start()
	defer parallelExecutor.Stop()

	// Create state access pattern
	statePattern := &RandomStatePattern{
		ContentionRate: 0.0,
		CommonKeys:     [][]byte{[]byte("common-key-1"), []byte("common-key-2")},
		KeySpace:       10000,
	}

	// Run benchmarks with different configurations
	benchmarkRunner := ExecutorBenchmark{
		Logger:   logger,
		Executor: parallelExecutor,
		TxGen:    &SimpleTransactionGenerator{pattern: statePattern},
		StateAccess: statePattern,
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Minute)
	defer cancel()

	results, err := benchmarkRunner.RunBenchmarks(ctx, config)
	if err != nil {
		return fmt.Errorf("failed to run benchmarks: %w", err)
	}

	// Output results in CSV format for easy analysis
	f, err := os.Create("parallel_benchmark_results.csv")
	if err != nil {
		return fmt.Errorf("failed to create output file: %w", err)
	}
	defer f.Close()

	// Write CSV header
	fmt.Fprintf(f, "BatchSize,ThreadCount,StateContention,TPS,LatencyP50,LatencyP95,LatencyP99,TotalTransactions\n")
	
	// Sort results by TPS for better visualization
	sort.Slice(results, func(i, j int) bool {
		return results[i].ThroughputTPS > results[j].ThroughputTPS
	})

	// Print results to console and CSV
	logger.Info("=== Benchmark Results ===")
	logger.Info("Sorted by throughput (TPS)")

	for _, result := range results {
		// Log result
		logger.Info(fmt.Sprintf("Benchmark config - BatchSize: %d, ThreadCount: %d, Contention: %.2f, TPS: %.2f, Latency p50/p95/p99: %v/%v/%v",
			result.BatchSize,
			result.ThreadCount,
			result.StateContention,
			result.ThroughputTPS,
			result.LatencyP50,
			result.LatencyP95,
			result.LatencyP99))

		// Write CSV row
		fmt.Fprintf(f, "%d,%d,%.2f,%.2f,%d,%d,%d,%d\n",
			result.BatchSize,
			result.ThreadCount,
			result.StateContention,
			result.ThroughputTPS,
			result.LatencyP50.Microseconds(),
			result.LatencyP95.Microseconds(),
			result.LatencyP99.Microseconds(),
			result.TotalTransactions,
		)
	}

	// Find best configuration
	bestResult := results[0]
	logger.Info(fmt.Sprintf("Best configuration - BatchSize: %d, ThreadCount: %d, Contention: %.2f, TPS: %.2f, Latency p50/p95: %v/%v",
		bestResult.BatchSize,
		bestResult.ThreadCount,
		bestResult.StateContention,
		bestResult.ThroughputTPS,
		bestResult.LatencyP50,
		bestResult.LatencyP95))

	logger.Info("Benchmark results written to parallel_benchmark_results.csv")
	return nil
}
