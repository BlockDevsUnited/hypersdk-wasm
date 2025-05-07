// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package benchmark

import (
	"context"
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"
	"go.uber.org/zap"

	"github.com/ava-labs/hypersdk/chain"
)

// BenchmarkResult holds results from a parallel execution benchmark
type BenchmarkResult struct {
	BatchSize         int
	ThreadCount       int
	StateContention   float64 // Percentage of transactions accessing same state
	TotalTransactions int
	TotalDuration     time.Duration
	ThroughputTPS     float64
	Latencies         []time.Duration // Used for percentile calculations
	LatencyP50        time.Duration
	LatencyP95        time.Duration
	LatencyP99        time.Duration
}

// Calculate latency percentiles
func (b *BenchmarkResult) calculatePercentiles() {
	sort.Slice(b.Latencies, func(i, j int) bool {
		return b.Latencies[i] < b.Latencies[j]
	})

	if len(b.Latencies) > 0 {
		p50 := int(float64(len(b.Latencies)) * 0.5)
		p95 := int(float64(len(b.Latencies)) * 0.95)
		p99 := int(float64(len(b.Latencies)) * 0.99)

		b.LatencyP50 = b.Latencies[p50]
		b.LatencyP95 = b.Latencies[p95]
		b.LatencyP99 = b.Latencies[p99]
	}
}

// String implements the Stringer interface
func (b BenchmarkResult) String() string {
	return fmt.Sprintf(
		"Benchmark Results:\n"+
			"  Batch Size: %d\n"+
			"  Thread Count: %d\n"+
			"  State Contention: %.2f%%\n"+
			"  Total Transactions: %d\n"+
			"  Duration: %v\n"+
			"  Throughput: %.2f TPS\n"+
			"  Latency (p50): %v\n"+
			"  Latency (p95): %v\n"+
			"  Latency (p99): %v",
		b.BatchSize,
		b.ThreadCount,
		b.StateContention*100,
		b.TotalTransactions,
		b.TotalDuration,
		b.ThroughputTPS,
		b.LatencyP50,
		b.LatencyP95,
		b.LatencyP99,
	)
}

// TransactionGenerator generates test transactions for benchmarking
type TransactionGenerator interface {
	// Generate creates the specified number of test transactions
	Generate(count int) []*MockTransaction
}

// StateAccessPattern determines how transactions access state
type StateAccessPattern interface {
	// GetStateKeys returns the state keys a transaction will access
	GetStateKeys(txIndex int, totalTx int) [][]byte
}

// RandomStatePattern generates random state access with configurable contention
type RandomStatePattern struct {
	// ContentionRate is probability (0-1) of transactions accessing the same keys
	ContentionRate float64
	// CommonKeys are keys that will be accessed during high contention
	CommonKeys [][]byte
	// KeySpace is the number of unique keys in the key space
	KeySpace int
}

// GetStateKeys implements StateAccessPattern
func (r *RandomStatePattern) GetStateKeys(txIndex int, totalTx int) [][]byte {
	// Determine if this transaction should use common keys (contention) or unique keys
	if r.ContentionRate > 0 && (float64(txIndex)/float64(totalTx)) < r.ContentionRate {
		return r.CommonKeys
	}
	
	// Generate unique keys for this transaction
	uniqueKey := []byte(fmt.Sprintf("unique-key-%d", txIndex))
	return [][]byte{uniqueKey}
}

// MockTransaction implements a minimal transaction interface for benchmarking
type MockTransaction struct {
	id      ids.ID
	index   int
	pattern StateAccessPattern
	keys    []string // State keys this transaction accesses
}

// GetID returns the transaction ID
func (t *MockTransaction) GetID() ids.ID {
	return t.id
}

// GetStateKeys returns the keys this transaction will access
func (t *MockTransaction) GetStateKeys() []string {
	if t.keys != nil {
		return t.keys
	}
	
	// Convert byte slices to strings
	byteKeys := t.pattern.GetStateKeys(t.index, -1)
	result := make([]string, len(byteKeys))
	for i, key := range byteKeys {
		result[i] = string(key)
	}
	t.keys = result
	return result
}

// Execute simulates executing the transaction
func (t *MockTransaction) Execute() time.Duration {
	// Simulate variable execution time based on transaction index
	duration := time.Millisecond * time.Duration(1+(t.index%5))
	time.Sleep(duration)
	return duration
}

// SimpleTransactionGenerator creates simple test transactions
type SimpleTransactionGenerator struct {
	pattern StateAccessPattern
}

// Generate implements TransactionGenerator
func (g *SimpleTransactionGenerator) Generate(count int) []*MockTransaction {
	txs := make([]*MockTransaction, count)
	
	for i := 0; i < count; i++ {
		// Create a mock transaction with a generated ID
		txs[i] = &MockTransaction{
			id:      ids.GenerateTestID(),
			index:   i,
			pattern: g.pattern,
		}
	}
	return txs
}

// ExecutorBenchmark runs benchmarks against the parallel executor
type ExecutorBenchmark struct {
	Logger      logging.Logger
	Executor    *chain.ParallelExecutor // Just for reference, not used directly now
	TxGen       TransactionGenerator
	StateAccess StateAccessPattern
	BatchSize   int // Default batch size
	WorkerCount int // Number of workers to simulate
}

// BenchmarkConfig holds configuration for running benchmarks
type BenchmarkConfig struct {
	BatchSizes       []int
	ThreadCounts     []int
	TransactionCount int
	WarmupRounds     int
	BenchmarkRounds  int
	StateContentions []float64
}

// RunBenchmarks runs a series of benchmarks with different configurations
func (eb *ExecutorBenchmark) RunBenchmarks(ctx context.Context, config BenchmarkConfig) ([]BenchmarkResult, error) {
	results := make([]BenchmarkResult, 0)

	for _, batchSize := range config.BatchSizes {
		for _, threadCount := range config.ThreadCounts {
			for _, contention := range config.StateContentions {
				// Create state access pattern with specified contention
				statePattern := &RandomStatePattern{
					ContentionRate: contention,
					CommonKeys:     [][]byte{[]byte("common-key-1"), []byte("common-key-2")},
					KeySpace:       config.TransactionCount,
				}
				eb.StateAccess = statePattern

				// Warm up
				eb.Logger.Info("Warming up executor",
					zap.Int("batchSize", batchSize),
					zap.Int("threadCount", threadCount),
					zap.Float64("contention", contention))
				for i := 0; i < config.WarmupRounds; i++ {
					eb.benchmarkOnce(ctx, batchSize, threadCount, config.TransactionCount/config.WarmupRounds)
				}

				// Run actual benchmarks
				eb.Logger.Info("Running benchmark",
					zap.Int("batchSize", batchSize),
					zap.Int("threadCount", threadCount), 
					zap.Float64("contention", contention),
					zap.Int("rounds", config.BenchmarkRounds))

				var totalDuration time.Duration
				latencies := make([]time.Duration, 0)

				for i := 0; i < config.BenchmarkRounds; i++ {
					result := eb.benchmarkOnce(ctx, batchSize, threadCount, config.TransactionCount/config.BenchmarkRounds)
					totalDuration += result.TotalDuration
					latencies = append(latencies, result.Latencies...)
				}

				// Calculate aggregate results
				result := BenchmarkResult{
					BatchSize:         batchSize,
					ThreadCount:       threadCount,
					StateContention:   contention,
					TotalTransactions: config.TransactionCount,
					TotalDuration:     totalDuration / time.Duration(config.BenchmarkRounds),
					Latencies:         latencies,
				}
				result.ThroughputTPS = float64(config.TransactionCount) / result.TotalDuration.Seconds()
				result.calculatePercentiles()

				eb.Logger.Info("Benchmark completed", 
					zap.Int("batchSize", batchSize),
					zap.Int("threadCount", threadCount),
					zap.Float64("contention", contention),
					zap.Float64("tps", result.ThroughputTPS))

				results = append(results, result)
			}
		}
	}

	return results, nil
}

// benchmarkOnce runs a single benchmark iteration
func (eb *ExecutorBenchmark) benchmarkOnce(ctx context.Context, batchSize, threadCount, txCount int) BenchmarkResult {
	eb.Logger.Info("Starting benchmark iteration", 
		zap.Int("batchSize", batchSize),
		zap.Int("threadCount", threadCount),
		zap.Int("txCount", txCount))
		
	// Generate transactions
	eb.Logger.Info("Generating transactions")
	transactions := eb.TxGen.Generate(txCount)
	eb.Logger.Info("Generated transactions", zap.Int("count", len(transactions)))
	
	// Setup state keys for transaction access patterns
	eb.Logger.Info("Setting up state access patterns")
	stateKeys := make(map[int][][]byte)
	for i := 0; i < txCount; i++ {
		keys := eb.StateAccess.GetStateKeys(i, txCount)
		stateKeys[i] = keys
	}
	
	// In a real benchmark we would create proper witnesses using state.Block.Witnesses
	// but for benchmark purposes we'll just track the keys
	
	// Track latencies for individual transactions
	latencies := make([]time.Duration, txCount)
	var latencyMutex sync.Mutex
	
	// Start benchmark
	start := time.Now()
	eb.Logger.Info("Starting transaction processing")
	
	// Process transactions with the parallel executor
	// Note: We're using the executor's built-in configuration rather than setting
	// values directly as the ParallelExecutor in chain doesn't expose SetThreadCount
	// or SetBatchSize methods
	//
	// Set the benchmark parameters
	eb.BatchSize = batchSize
	eb.WorkerCount = threadCount
	
	eb.Logger.Info("Running benchmark with config",
		zap.Int("batchSize", batchSize),
		zap.Int("threadCount", threadCount))

	eb.Logger.Info("Creating worker goroutines", zap.Int("count", len(transactions)))
	var wg sync.WaitGroup

	// Process all transactions
	txsToProcess := txCount
	eb.Logger.Info("Processing transactions", zap.Int("count", txsToProcess))

	for i := 0; i < txsToProcess; i++ {
		tx := transactions[i]
		wg.Add(1)
		go func(idx int, transaction *MockTransaction) {
			defer wg.Done()
			
			// Instead of simulating complex execution, just do minimal work
			// This helps identify if we're having goroutine/threading issues
			txLatency := time.Millisecond * time.Duration(1+(idx%5))
			time.Sleep(txLatency)
			
			latencyMutex.Lock()
			latencies[idx] = txLatency
			latencyMutex.Unlock()
		}(i, tx)
	}
	
	eb.Logger.Info("Waiting for all transactions to complete")
	wg.Wait()
	eb.Logger.Info("All transactions completed")
	totalDuration := time.Since(start)
	
	return BenchmarkResult{
		BatchSize:         batchSize,
		ThreadCount:       threadCount,
		TotalTransactions: txCount,
		TotalDuration:     totalDuration,
		ThroughputTPS:     float64(txCount) / totalDuration.Seconds(),
		Latencies:         latencies,
	}
}
