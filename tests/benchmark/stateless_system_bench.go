// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package benchmark

import (
	"context"
	"crypto/sha256"
	"fmt"
	"math/rand"
	"os"
	"sort"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"
	"go.uber.org/zap"

	"github.com/ava-labs/hypersdk/chain"
)

// StatelessSystemBenchmark provides a more realistic benchmark of the full stateless system
// including witness validation, cryptographic operations, and network simulation.
type StatelessSystemBenchmark struct {
	Logger          logging.Logger
	Executor        *chain.ParallelExecutor
	TxGen           TransactionGenerator
	StateAccess     StateAccessPattern
	BatchSize       int
	WorkerCount     int
	NetworkLatencyMs int // Simulated network latency in milliseconds
	
	// Configuration for TEE and witness simulations
	WitnessValidationTimeMs int // Time to simulate witness validation in ms
	CryptoVerificationTimeMs int // Time to simulate crypto verification in ms
	TEEProcessingTimeMs     int // Time to simulate TEE processing in ms
}

// WitnessEntry simulates a state witness entry
type MockWitnessEntry struct {
	Key       []byte
	Value     []byte
	Height    uint64
	Hash      []byte
	Timestamp time.Time
}

// TEEBatch simulates a batch processed by a TEE
type MockTEEBatch struct {
	ID        ids.ID
	TxIDs     []ids.ID
	BatchHash []byte
	Signature []byte
}

// RunStatelessBenchmark runs a benchmark with the full stateless system components
func RunStatelessBenchmark() error {
	// Setup logger
	logFactory := logging.NewFactory(logging.Config{
		DisplayLevel: logging.Info,
	})
	logger, err := logFactory.Make("StatelessBench")
	if err != nil {
		return fmt.Errorf("failed to create logger: %w", err)
	}

	logger.Info("Starting comprehensive stateless system benchmark")
	
	// Create parallel executor as in previous benchmarks
	parallelConfig := chain.ParallelConfig{
		MaxThreads: 8,  // Based on our previous best findings
		BatchSize:  100, // Based on our previous best findings
	}
	parallelExecutor := chain.NewParallelExecutor(logger, &parallelConfig, false)
	parallelExecutor.Start()
	defer parallelExecutor.Stop()
	
	// Create a more realistic benchmark setup with witness/TEE/crypto components
	benchConfig := BenchmarkConfig{
		BatchSizes:       []int{100}, // Using optimal from previous test
		ThreadCounts:     []int{8},   // Using optimal from previous test
		TransactionCount: 1000,       // Smaller count for more complex benchmark
		WarmupRounds:     1,
		BenchmarkRounds:  3,
		StateContentions: []float64{0.0, 0.25, 0.50},
	}
	
	// Create state pattern
	statePattern := &RandomStatePattern{
		ContentionRate: 0.25, // Use moderate contention rate
		CommonKeys:     [][]byte{[]byte("common-key-1"), []byte("common-key-2")},
		KeySpace:       10000,
	}
	
	benchmark := &StatelessSystemBenchmark{
		Logger:                 logger,
		Executor:               parallelExecutor,
		TxGen:                  &SimpleTransactionGenerator{pattern: statePattern},
		StateAccess:            statePattern,
		NetworkLatencyMs:       5,     // 5ms simulated network latency
		WitnessValidationTimeMs: 2,    // 2ms simulated witness validation
		CryptoVerificationTimeMs: 1,    // 1ms simulated crypto verification
		TEEProcessingTimeMs:     10,   // 10ms simulated TEE processing
	}
	
	// Run the benchmark
	results, err := benchmark.RunFullSystemBenchmark(context.Background(), benchConfig)
	if err != nil {
		return fmt.Errorf("failed to run stateless system benchmark: %w", err)
	}
	
	// Output results
	logger.Info("=== Stateless System Benchmark Results ===")
	logger.Info("Sorted by throughput (TPS)")
	
	sort.Slice(results, func(i, j int) bool {
		return results[i].ThroughputTPS > results[j].ThroughputTPS
	})
	
	for _, result := range results {
		logger.Info(fmt.Sprintf("Benchmark config - BatchSize: %d, ThreadCount: %d, Contention: %.2f, TPS: %.2f, Latency p50/p95/p99: %v/%v/%v",
			result.BatchSize,
			result.ThreadCount,
			result.StateContention,
			result.ThroughputTPS,
			result.LatencyP50,
			result.LatencyP95,
			result.LatencyP99))
	}
	
	// Write CSV output
	f, err := createCSVOutput("stateless_system_benchmark_results.csv", results)
	if err != nil {
		return err
	}
	defer f.Close()
	
	logger.Info("Benchmark results written to stateless_system_benchmark_results.csv")
	return nil
}

// RunFullSystemBenchmark runs the benchmark with all components
func (sb *StatelessSystemBenchmark) RunFullSystemBenchmark(ctx context.Context, config BenchmarkConfig) ([]BenchmarkResult, error) {
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
				sb.StateAccess = statePattern
				sb.TxGen = &SimpleTransactionGenerator{pattern: statePattern}
				
				// Configuration
				sb.BatchSize = batchSize
				sb.WorkerCount = threadCount
				
				// Warm up
				sb.Logger.Info("Warming up system",
					zap.Int("batchSize", batchSize),
					zap.Int("threadCount", threadCount),
					zap.Float64("contention", contention))
				
				for i := 0; i < config.WarmupRounds; i++ {
					sb.benchmarkFullSystemOnce(ctx, config.TransactionCount/config.WarmupRounds)
				}
				
				// Run actual benchmarks
				sb.Logger.Info("Running system benchmark",
					zap.Int("batchSize", batchSize),
					zap.Int("threadCount", threadCount),
					zap.Float64("contention", contention))
				
				var totalDuration time.Duration
				latencies := make([]time.Duration, 0)
				
				for i := 0; i < config.BenchmarkRounds; i++ {
					result := sb.benchmarkFullSystemOnce(ctx, config.TransactionCount/config.BenchmarkRounds)
					totalDuration += result.TotalDuration
					latencies = append(latencies, result.Latencies...)
				}
				
				// Calculate results
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
				
				sb.Logger.Info("System benchmark completed",
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

// benchmarkFullSystemOnce runs a single iteration of the full system benchmark
func (sb *StatelessSystemBenchmark) benchmarkFullSystemOnce(ctx context.Context, txCount int) BenchmarkResult {
	sb.Logger.Info("Starting full system benchmark iteration", zap.Int("txCount", txCount))
	
	// Generate transactions
	transactions := sb.TxGen.Generate(txCount)
	
	// Create a mock witness store for this benchmark
	witnessStore := createMockWitnessStore(txCount)
	
	// Track latencies
	latencies := make([]time.Duration, txCount)
	var latencyMutex sync.Mutex
	
	// Start benchmark timing
	start := time.Now()
	
	// Process in reasonable sized batches
	batchSize := sb.BatchSize
	if batchSize <= 0 {
		batchSize = 100
	}
	
	// Use a worker pool to process transactions
	var wg sync.WaitGroup
	workerCount := sb.WorkerCount
	if workerCount <= 0 {
		workerCount = 8
	}
	
	// Create channel for workers
	txChan := make(chan *MockTransaction, txCount)
	for _, tx := range transactions {
		txChan <- tx
	}
	close(txChan)
	
	// Start workers
	for w := 0; w < workerCount; w++ {
		wg.Add(1)
		go func(workerID int) {
			defer wg.Done()
			
			for tx := range txChan {
				txStart := time.Now()
				
				// STEP 1: Simulate network latency for transaction receipt
				simulateNetworkLatency(sb.NetworkLatencyMs)
				
				// STEP 2: Get transaction dependencies (keys it will access)
				keys := tx.GetStateKeys()
				
				// STEP 3: Fetch and validate witnesses for each key
				for _, key := range keys {
					// Find witness in store (simulated)
					witness, found := witnessStore[key]
					if found {
						// Simulate witness validation time
						simulateWitnessValidation(sb.WitnessValidationTimeMs, witness)
					} else {
						// If no witness, simulate creating one (more expensive)
						simulateWitnessCreation(key)
					}
				}
				
				// STEP 4: Simulate TEE execution (SGX or SEV)
				simulateTEEProcessing(sb.TEEProcessingTimeMs)
				
				// STEP 5: Simulate cryptographic verification
				simulateCryptoVerification(sb.CryptoVerificationTimeMs, tx.GetID().String())
				
				// Calculate total latency
				txLatency := time.Since(txStart)
				
				// Record latency
				latencyMutex.Lock()
				latencies[tx.index] = txLatency
				latencyMutex.Unlock()
			}
		}(w)
	}
	
	// Wait for all transactions to be processed
	wg.Wait()
	
	// Calculate total duration
	totalDuration := time.Since(start)
	
	return BenchmarkResult{
		BatchSize:         sb.BatchSize,
		ThreadCount:       sb.WorkerCount,
		TotalTransactions: txCount,
		TotalDuration:     totalDuration,
		ThroughputTPS:     float64(txCount) / totalDuration.Seconds(),
		Latencies:         latencies,
	}
}

// Helper functions for simulation

// createMockWitnessStore creates a simulated witness store
func createMockWitnessStore(capacity int) map[string]*MockWitnessEntry {
	store := make(map[string]*MockWitnessEntry, capacity)
	
	// Pre-populate with some witnesses
	for i := 0; i < capacity/5; i++ {
		key := fmt.Sprintf("key-%d", i)
		value := fmt.Sprintf("value-%d", i)
		
		// Create hash
		hasher := sha256.New()
		hasher.Write([]byte(key))
		hasher.Write([]byte(value))
		hash := hasher.Sum(nil)
		
		store[key] = &MockWitnessEntry{
			Key:       []byte(key),
			Value:     []byte(value),
			Height:    uint64(100 + i),
			Hash:      hash,
			Timestamp: time.Now().Add(-time.Duration(rand.Intn(3600)) * time.Second),
		}
	}
	
	return store
}

// simulateNetworkLatency simulates network communication
func simulateNetworkLatency(latencyMs int) {
	// Add some randomness to the latency to be more realistic
	jitter := rand.Intn(latencyMs/2 + 1)
	time.Sleep(time.Duration(latencyMs+jitter) * time.Millisecond)
}

// simulateWitnessValidation simulates validating a witness
func simulateWitnessValidation(timeMs int, witness *MockWitnessEntry) {
	// Do some actual work to simulate validation
	hasher := sha256.New()
	hasher.Write(witness.Key)
	hasher.Write(witness.Value)
	hasher.Write([]byte(fmt.Sprintf("%d", witness.Height)))
	_ = hasher.Sum(nil)
	
	// Add simulation time
	time.Sleep(time.Duration(timeMs) * time.Millisecond)
}

// simulateWitnessCreation simulates creating a new witness
func simulateWitnessCreation(key string) {
	// Creating a witness is more expensive than validating one
	time.Sleep(5 * time.Millisecond)
}

// simulateTEEProcessing simulates processing in a Trusted Execution Environment
func simulateTEEProcessing(timeMs int) {
	// Simulate some variability in TEE processing time
	variability := rand.Intn(timeMs/4 + 1)
	processingTime := timeMs + variability
	
	// Do some actual computation to simulate TEE work
	data := make([]byte, 1024)
	rand.Read(data)
	hash := sha256.Sum256(data)
	_ = hash
	
	time.Sleep(time.Duration(processingTime) * time.Millisecond)
}

// simulateCryptoVerification simulates cryptographic verification
func simulateCryptoVerification(timeMs int, data string) {
	// Do some actual crypto work
	hash1 := sha256.Sum256([]byte(data))
	hash2 := sha256.Sum256(hash1[:])
	_ = hash2
	
	time.Sleep(time.Duration(timeMs) * time.Millisecond)
}

// Helper to create CSV output
func createCSVOutput(filename string, results []BenchmarkResult) (*os.File, error) {
	f, err := os.Create(filename)
	if err != nil {
		return nil, fmt.Errorf("failed to create output file: %w", err)
	}
	
	// Write CSV header
	fmt.Fprintf(f, "BatchSize,ThreadCount,StateContention,TPS,LatencyP50,LatencyP95,LatencyP99,TotalTransactions,NetworkLatencyMs,WitnessValidationMs,CryptoVerificationMs,TEEProcessingMs\n")
	
	// Write results
	for _, result := range results {
		fmt.Fprintf(f, "%d,%d,%.2f,%.2f,%v,%v,%v,%d,%d,%d,%d,%d\n",
			result.BatchSize,
			result.ThreadCount,
			result.StateContention,
			result.ThroughputTPS,
			result.LatencyP50,
			result.LatencyP95,
			result.LatencyP99,
			result.TotalTransactions,
			5, // NetworkLatencyMs - hardcoded for now
			2, // WitnessValidationMs - hardcoded for now
			1, // CryptoVerificationMs - hardcoded for now
			10, // TEEProcessingMs - hardcoded for now
		)
	}
	
	return f, nil
}
