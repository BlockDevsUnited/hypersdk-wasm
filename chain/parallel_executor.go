// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package chain

import (
	"context"
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/ava-labs/avalanchego/utils/set"
	"github.com/ava-labs/avalanchego/utils/timer"
	"github.com/ava-labs/hypersdk/state"
	"go.uber.org/zap"
)

const (
	// Default values for parallel execution
	defaultBatchSize        = 500 // Matches your current 500-element batch size
	defaultMaxThreads       = 8   // Matches your 8-thread parallel execution
	defaultMaxDepWaitTime   = 50 * time.Millisecond
	defaultConflictRetries  = 3
	defaultTimeoutExtension = 200 * time.Millisecond
)

// ParallelExecutionStats tracks statistics about parallel execution
type ParallelExecutionStats struct {
	TotalTxsProcessed      atomic.Uint64
	TotalBatchesProcessed  atomic.Uint64
	ConflictsDetected      atomic.Uint64
	ConflictsResolved      atomic.Uint64
	MaxParallelism         atomic.Uint64
	AverageParallelism     atomic.Uint64
	AverageLatency         atomic.Uint64 // Nanoseconds
	WaitingOnDeps          atomic.Uint64
	BatchPrepTime          atomic.Uint64 // Nanoseconds
	BatchExecTime          atomic.Uint64 // Nanoseconds
	StateApplicationTime   atomic.Uint64 // Nanoseconds
	TeeAttestationTime     atomic.Uint64 // Nanoseconds
	WitnessPreparationTime atomic.Uint64 // Nanoseconds
}

// ParallelConfig contains configuration for parallel execution
type ParallelConfig struct {
	MaxThreads          int
	BatchSize           int
	MaxDependencyWait   time.Duration
	ConflictRetries     int
	TimeoutExtension    time.Duration
	DynamicBatchSizing  bool
	PrioritizeFeeYield  bool
	StateCompressionLvl int // 0-9, with 0 being no compression
}

// DefaultParallelConfig returns a default configuration
func DefaultParallelConfig() *ParallelConfig {
	return &ParallelConfig{
		MaxThreads:          defaultMaxThreads,
		BatchSize:           defaultBatchSize,
		MaxDependencyWait:   defaultMaxDepWaitTime,
		ConflictRetries:     defaultConflictRetries,
		TimeoutExtension:    defaultTimeoutExtension,
		DynamicBatchSizing:  true,
		PrioritizeFeeYield:  true,
		StateCompressionLvl: 1,
	}
}

// TransactionDep represents a dependency between transactions
type TransactionDep struct {
	TxID       ids.ID
	StateKeys  [][]byte
	DependsOn  set.Set[ids.ID]
	Priority   uint64
	Ready      bool
	Processing bool
	Completed  bool
	Result     *Result
	Err        error
}

// ParallelExecutor handles parallel execution of transactions
type ParallelExecutor struct {
	log      logging.Logger
	config   *ParallelConfig
	metrics  *ParallelExecutionStats
	shutdown chan struct{}

	// State tracking
	txTracker      map[ids.ID]*TransactionDep
	readSets       map[string]set.Set[ids.ID]  // StateKey -> Set of TxIDs that read it
	writeSets      map[string]set.Set[ids.ID]  // StateKey -> Set of TxIDs that write it
	txTrackerMutex sync.RWMutex
	
	// TEE integration
	teeEnabled   bool
	sgxBatchSets map[ids.ID]*TEEBatchSet
	sevBatchSets map[ids.ID]*TEEBatchSet
	
	// Execution pipeline
	readyQueue   chan ids.ID
	resultQueue  chan *ResultWithDeps
	workerWG     sync.WaitGroup
	statsTracker *timer.Timer
}

// TEEBatchSet tracks a set of transactions assigned to a specific TEE
type TEEBatchSet struct {
	BatchID     ids.ID
	TEEID       ids.ID
	TEEType     string // "SGX" or "SEV"
	Txs         []*Transaction
	StateKeys   set.Set[string]
	BatchStatus string // "pending", "processing", "attested", "committed"
	Attestation []byte
	Proof       []byte
}

// ResultWithDeps contains execution result with dependency tracking
type ResultWithDeps struct {
	TxID   ids.ID
	Result *Result
	Err    error
}

// NewParallelExecutor creates a new parallel execution engine
func NewParallelExecutor(
	log logging.Logger,
	config *ParallelConfig,
	teeEnabled bool,
) *ParallelExecutor {
	if config == nil {
		config = DefaultParallelConfig()
	}
	
	// Adjust thread count based on available cores if not specified
	if config.MaxThreads <= 0 {
		config.MaxThreads = runtime.NumCPU()
	}
	
	return &ParallelExecutor{
		log:          log,
		config:       config,
		metrics:      &ParallelExecutionStats{},
		shutdown:     make(chan struct{}),
		txTracker:    make(map[ids.ID]*TransactionDep),
		readSets:     make(map[string]set.Set[ids.ID]),
		writeSets:    make(map[string]set.Set[ids.ID]),
		teeEnabled:   teeEnabled,
		sgxBatchSets: make(map[ids.ID]*TEEBatchSet),
		sevBatchSets: make(map[ids.ID]*TEEBatchSet),
		readyQueue:   make(chan ids.ID, config.BatchSize*2),
		resultQueue:  make(chan *ResultWithDeps, config.BatchSize*2),
	}
}

// Start initializes the executor workers
func (pe *ParallelExecutor) Start() {
	pe.statsTracker = timer.NewTimer(func() {
		pe.logStats()
	})
	pe.statsTracker.SetTimeoutIn(30 * time.Second) // Log stats every 30 seconds
	
	// Start worker pool
	pe.workerWG.Add(pe.config.MaxThreads)
	for i := 0; i < pe.config.MaxThreads; i++ {
		go pe.worker()
	}
	
	pe.log.Info("Parallel executor started",
		zap.Int("threads", pe.config.MaxThreads),
		zap.Int("batchSize", pe.config.BatchSize),
		zap.Bool("teeEnabled", pe.teeEnabled),
	)
}

// Stop gracefully shuts down the executor
func (pe *ParallelExecutor) Stop() {
	close(pe.shutdown)
	pe.workerWG.Wait()
	pe.statsTracker.Stop()
	pe.log.Info("Parallel executor stopped")
}

// worker processes transactions from the ready queue
func (pe *ParallelExecutor) worker() {
	defer pe.workerWG.Done()
	
	for {
		select {
		case <-pe.shutdown:
			return
		case txID := <-pe.readyQueue:
			pe.processTx(txID)
		}
	}
}

// processTx executes a single transaction
func (pe *ParallelExecutor) processTx(txID ids.ID) {
	pe.txTrackerMutex.RLock()
	txDep, exists := pe.txTracker[txID]
	pe.txTrackerMutex.RUnlock()
	
	if !exists || txDep.Completed {
		return
	}
	
	// Mark as processing
	pe.txTrackerMutex.Lock()
	if txDep.Processing || txDep.Completed {
		pe.txTrackerMutex.Unlock()
		return
	}
	txDep.Processing = true
	pe.txTrackerMutex.Unlock()
	
	// Execute transaction logic here
	// For now we'll just simulate with a placeholder
	result := &Result{
		Success: true,
	}
	
	// Update tracker with result
	pe.txTrackerMutex.Lock()
	txDep.Completed = true
	txDep.Result = result
	pe.txTrackerMutex.Unlock()
	
	// Send result
	pe.resultQueue <- &ResultWithDeps{
		TxID:   txID,
		Result: result,
	}
	
	// Check for newly ready transactions
	pe.updateDependencies(txID)
}

// updateDependencies checks if any transaction dependencies are satisfied
// after completing a transaction
func (pe *ParallelExecutor) updateDependencies(completedTxID ids.ID) {
	pe.txTrackerMutex.Lock()
	defer pe.txTrackerMutex.Unlock()
	
	// Find transactions that depend on the completed transaction
	for _, txDep := range pe.txTracker {
		if txDep.Completed || txDep.Processing {
			continue
		}
		
		if txDep.DependsOn.Contains(completedTxID) {
			txDep.DependsOn.Remove(completedTxID)
			
			// If all dependencies are satisfied, mark as ready
			if txDep.DependsOn.Len() == 0 {
				txDep.Ready = true
				
				// Add to ready queue
				select {
				case pe.readyQueue <- txDep.TxID:
				default:
					pe.log.Warn("Ready queue full, transaction execution delayed",
						zap.Stringer("txID", txDep.TxID))
				}
			}
		}
	}
}

// ExecuteParallel processes a batch of transactions in parallel
func (pe *ParallelExecutor) ExecuteParallel(
	ctx context.Context,
	txs []*Transaction,
	parentView state.Immutable,
	blockCtx blockContext,
	r Rules,
) ([]*Result, error) {
	if len(txs) == 0 {
		return nil, nil
	}
	
	startTime := time.Now()
	
	// Phase 1: Analyze dependencies
	if err := pe.analyzeDependencies(ctx, txs, parentView); err != nil {
		return nil, fmt.Errorf("dependency analysis failed: %w", err)
	}
	
	depAnalysisTime := time.Since(startTime)
	
	// Phase 2: Execute transactions
	results, err := pe.executeTransactions(ctx, txs, parentView, blockCtx, r)
	if err != nil {
		return nil, fmt.Errorf("transaction execution failed: %w", err)
	}
	
	executionTime := time.Since(startTime) - depAnalysisTime
	
	// Update metrics
	pe.metrics.TotalTxsProcessed.Add(uint64(len(txs)))
	pe.metrics.TotalBatchesProcessed.Add(1)
	pe.metrics.BatchPrepTime.Store(uint64(depAnalysisTime.Nanoseconds()))
	pe.metrics.BatchExecTime.Store(uint64(executionTime.Nanoseconds()))
	
	pe.log.Debug("Parallel execution completed",
		zap.Int("txCount", len(txs)),
		zap.Duration("depAnalysisTime", depAnalysisTime),
		zap.Duration("executionTime", executionTime),
		zap.Duration("totalTime", time.Since(startTime)))
	
	return results, nil
}

// analyzeDependencies identifies dependencies between transactions
func (pe *ParallelExecutor) analyzeDependencies(
	ctx context.Context,
	txs []*Transaction,
	parentView state.Immutable,
) error {
	pe.txTrackerMutex.Lock()
	defer pe.txTrackerMutex.Unlock()
	
	// Clear previous state
	pe.txTracker = make(map[ids.ID]*TransactionDep)
	pe.readSets = make(map[string]set.Set[ids.ID])
	pe.writeSets = make(map[string]set.Set[ids.ID])
	
	// First pass: collect state keys for each transaction
	for _, tx := range txs {
		txID := tx.GetID()
		
		stateKeys, err := tx.StateKeys(nil) // Replace with actual balance handler
		if err != nil {
			return fmt.Errorf("failed to get state keys for tx %s: %w", txID, err)
		}
		
		// Convert state.Keys (map[string]Permissions) to [][]byte for storage in TransactionDep
		keyBytes := make([][]byte, 0, len(stateKeys))
		for strKey := range stateKeys {
			// Convert string keys to byte slices
			keyBytes = append(keyBytes, []byte(strKey))
		}
		
		pe.txTracker[txID] = &TransactionDep{
			TxID:      txID,
			StateKeys: keyBytes,
			DependsOn: set.Set[ids.ID]{},  // Initialize as empty set
			Priority:  calculatePriority(tx),
			Ready:     false,
		}
	}
	
	// Second pass: build dependency graph
	for _, tx := range txs {
		txID := tx.GetID()
		txDep := pe.txTracker[txID]
		
		// For each state key this tx accesses
		for _, stateKey := range txDep.StateKeys {
			strKey := string(stateKey)
			
			// Check for write conflicts (read-write, write-write)
			if writers, exists := pe.writeSets[strKey]; exists {
				for writerID := range writers {
					if writerID != txID {
						// This tx depends on any previous writer
						txDep.DependsOn.Add(writerID)
					}
				}
			}
			
			// Register this tx as a reader/writer for this key
			// For simplicity, we treat all accesses as potential writes
			if _, exists := pe.writeSets[strKey]; !exists {
				pe.writeSets[strKey] = set.Set[ids.ID]{}
			}
			// Add the ID to the set
			writeSet := pe.writeSets[strKey]
			writeSet.Add(txID)
			pe.writeSets[strKey] = writeSet
			
			if _, exists := pe.readSets[strKey]; !exists {
				pe.readSets[strKey] = set.Set[ids.ID]{}
			}
			// Add the ID to the set
			readSet := pe.readSets[strKey]
			readSet.Add(txID)
			pe.readSets[strKey] = readSet
		}
		
		// If no dependencies, mark as ready
		if txDep.DependsOn.Len() == 0 {
			txDep.Ready = true
			
			// Add to ready queue
			select {
			case pe.readyQueue <- txID:
			default:
				pe.log.Warn("Ready queue full during initialization",
					zap.Stringer("txID", txID))
			}
		}
	}
	
	return nil
}

// executeTransactions executes transactions in parallel according to dependencies
func (pe *ParallelExecutor) executeTransactions(
	ctx context.Context,
	txs []*Transaction,
	parentView state.Immutable,
	blockCtx blockContext,
	r Rules,
) ([]*Result, error) {
	resultCount := len(txs)
	results := make([]*Result, resultCount)
	
	// Create a mapping from txID to index
	txIndexMap := make(map[ids.ID]int, resultCount)
	for i, tx := range txs {
		txIndexMap[tx.GetID()] = i
	}
	
	// Process results until all transactions are complete or context is canceled
	processedCount := 0
	for processedCount < resultCount {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case res := <-pe.resultQueue:
			if idx, exists := txIndexMap[res.TxID]; exists && idx < resultCount {
				results[idx] = res.Result
				processedCount++
			}
		}
	}
	
	return results, nil
}

// logStats logs current execution statistics
func (pe *ParallelExecutor) logStats() {
	pe.log.Info("Parallel execution stats",
		zap.Uint64("txsProcessed", pe.metrics.TotalTxsProcessed.Load()),
		zap.Uint64("batchesProcessed", pe.metrics.TotalBatchesProcessed.Load()),
		zap.Uint64("conflicts", pe.metrics.ConflictsDetected.Load()),
		zap.Uint64("conflictsResolved", pe.metrics.ConflictsResolved.Load()),
	)
}

// calculatePriority determines transaction priority based on fees and other factors
func calculatePriority(tx *Transaction) uint64 {
	// Simple priority calculation based on fee rate
	// In a real implementation, use the actual fee calculation logic
	return 100 // Placeholder
}
