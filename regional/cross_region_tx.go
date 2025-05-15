package regional

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"
	
	"github.com/ava-labs/avalanchego/ids"
	"go.uber.org/zap"
)

// txCircuitBreakers maintains a circuit breaker for each target region
var (
	txCircuitBreakers     = make(map[string]*CircuitBreaker)
	txCircuitBreakersMutex sync.RWMutex
)

// getOrCreateCircuitBreaker gets an existing circuit breaker or creates a new one
func (c *CrossRegionCoordinator) getOrCreateCircuitBreaker(region string) *CircuitBreaker {
	txCircuitBreakersMutex.RLock()
	cb, exists := txCircuitBreakers[region]
	txCircuitBreakersMutex.RUnlock()

	if exists {
		return cb
	}

	// Create new circuit breaker
	txCircuitBreakersMutex.Lock()
	defer txCircuitBreakersMutex.Unlock()
	
	// Double-check to avoid race condition
	cb, exists = txCircuitBreakers[region]
	if exists {
		return cb
	}
	
	// Create predicate to determine which errors should trip the breaker
	shouldTrip := func(err error) bool {
		var crErr CrossRegionError
		if errors.As(err, &crErr) {
			// Only network errors and timeouts should trip the circuit breaker
			return crErr.Category == NetworkError || crErr.Category == TimeoutError
		}
		return false
	}
	
	// Create a new circuit breaker with a threshold of 5 failures and 5s reset timeout
	cb = NewCircuitBreaker(5, 5*time.Second, shouldTrip, c.logger)
	txCircuitBreakers[region] = cb
	return cb
}

// ProcessCrossRegionTransaction processes a transaction that spans multiple regions
// It handles state conflict detection and resolution between regions with improved
// error handling and resilience mechanisms
func (c *CrossRegionCoordinator) ProcessCrossRegionTransaction(ctx context.Context, sourceRegion string, targetRegions []string, 
	stateUpdates map[string][]byte, txID ids.ID, timestamp time.Time) error {

	c.logger.Info("Processing cross-region transaction", 
		zap.String("sourceRegion", sourceRegion),
		zap.Strings("targetRegions", targetRegions),
		zap.Stringer("txID", txID),
		zap.Time("timestamp", timestamp),
	)
	
	// Create timeout context to ensure operations don't hang indefinitely
	txCtx, cancel := context.WithTimeout(ctx, 500*time.Millisecond) // 500ms aligns with our "100ms and regulated" value proposition
	defer cancel()

	// Create operation proof (in production, this would be a real attestation proof)
	proof := []byte("mock-attestation-proof")
	
	// Create cross-region operation
	op := &CrossRegionOperation{
		SourceRegion:  sourceRegion,
		TargetRegions: targetRegions,
		OperationType: TransactionSync,
		StateUpdates:  stateUpdates,
		Timestamp:     timestamp,
		TxIDs:         []ids.ID{txID},
		Proof:         proof,
	}
	
	// Ensure the operation is valid before proceeding
	if len(targetRegions) == 0 {
		return NewValidationError("", txID.String(), "no target regions specified", nil)
	}
	
	// Check if source region is valid
	if sourceRegion == "" {
		return NewValidationError("", txID.String(), "source region not specified", nil)
	}
	
	c.lock.Lock()
	defer c.lock.Unlock()
	
	// For each target region, check for potential conflicts
	for _, targetRegion := range targetRegions {
		// Get the local state for the target region
		localState, exists := c.regionStates[targetRegion]
		if !exists {
			c.logger.Info("Target region state not found, creating initial state",
				zap.String("region", targetRegion))
			localState = &RegionalStateVersion{
				Height:    0,
				StateRoot: nil,
				Changes:   make(map[string][]byte),
				Timestamp: time.Now(),
			}
			c.regionStates[targetRegion] = localState
			continue // No conflicts if region state doesn't exist yet
		}
		
		// Check for pending operations that might conflict
		pendingUpdates := make(map[string][]byte)
		for _, pendingOp := range c.pendingOperations {
			if containsRegion(pendingOp.TargetRegions, targetRegion) {
				for k, v := range pendingOp.StateUpdates {
					pendingUpdates[k] = v
				}
			}
		}
		
		// Create a conflict detector
		conflictDetector := NewConflictDetector(c.config.ConflictDetection, c.logger)
		
		// Detect conflicts between the pending local updates and incoming cross-region updates
		localTxID, err := ids.FromString("local-pending-tx-" + targetRegion) // Mock ID for local updates
		if err != nil {
			localTxID = ids.Empty
		}
		
		conflicts := conflictDetector.DetectConflicts(
			pendingUpdates, localState.Timestamp, localTxID,
			stateUpdates, timestamp, txID,
		)
		
		if len(conflicts) > 0 {
			c.logger.Info("Detected conflicts in cross-region transaction", 
				zap.Int("conflictCount", len(conflicts)),
				zap.String("targetRegion", targetRegion),
			)
			
			// Resolve the conflicts
			resolutions := conflictDetector.ResolveConflicts(conflicts)
			
			// Apply resolutions
			for key, resolution := range resolutions {
				switch resolution {
				case LocalWins:
					// Local state takes priority, remove from cross-region update
					delete(op.StateUpdates, key)
					c.logger.Debug("Local update takes precedence", 
						zap.String("key", key),
						zap.String("region", targetRegion),
					)
					// If local priority is enabled, we need to make sure the key is removed from all state updates
					if c.config.ConflictDetection.LocalPriority {
						delete(stateUpdates, key) // This ensures the key is actually removed from the source map too
					}
				
				case Rejected:
					// Regulatory conflict, operation must be rejected entirely
					c.logger.Warn("Cross-region operation rejected due to regulatory conflict", 
						zap.String("key", key),
						zap.String("region", targetRegion),
					)
					return fmt.Errorf("operation rejected due to regulatory conflict on key %s", key)
				
				// CrossRegionWins and NoResolutionNeeded: keep the cross-region update as is
				}
			}
		}
	}
	
	// If we've made it here, all conflicts have been resolved or accepted
	// Use circuit breakers and retry logic for queuing operations
	
	// Keep track of failed regions
	failedRegions := make([]string, 0)
	
	// Process each target region individually with circuit breaker protection
	for _, region := range targetRegions {
		cb := c.getOrCreateCircuitBreaker(region)
		
		// Define operation function that will be executed with circuit breaker protection
		operation := func() error {
			// Queue the operation for the specific region
			return RetryWithBackoff(txCtx, func() error {
				select {
				case <-txCtx.Done():
					return NewTimeoutError(region, txID.String(), "operation timed out while queuing")
				default:
					err := c.QueueOperation(op)
					if err != nil {
						return NewNetworkError(region, txID.String(), "failed to queue cross-region operation", err)
					}
					return nil
				}
			}, DefaultRetryPolicy(), c.logger)
		}
		
		// Execute with circuit breaker protection
		err := cb.Execute(operation)
		if err != nil {
			c.logger.Warn("Failed to process cross-region transaction for region",
				zap.String("region", region),
				zap.Stringer("txID", txID),
				zap.Error(err),
			)
			failedRegions = append(failedRegions, region)
			
			// Check if it's a critical error that requires immediate attention
			var crErr CrossRegionError
			if errors.As(err, &crErr) && crErr.Critical {
				c.logger.Error("Critical error encountered in cross-region transaction",
					zap.String("category", fmt.Sprintf("%d", crErr.Category)),
					zap.String("region", region),
					zap.Stringer("txID", txID),
					zap.Error(err),
				)
				return err
			}
			
			// Continue with other regions even if one fails
			continue
		}
	}
	
	// If all regions failed, return an error
	if len(failedRegions) == len(targetRegions) {
		return NewNetworkError("", txID.String(), 
			fmt.Sprintf("all target regions failed to process transaction: %v", failedRegions), nil)
	}
	
	// If some regions succeeded, add the operation to pending operations
	if len(failedRegions) < len(targetRegions) {
		// Store the operation in pending operations for tracking
		c.pendingOperations[txID] = op
		
		// If some regions failed, log a warning
		if len(failedRegions) > 0 {
			c.logger.Warn("Cross-region transaction partially processed",
				zap.Stringer("txID", txID),
				zap.Strings("failedRegions", failedRegions),
			)
		}
	}
	
	return nil
}

// containsRegion checks if a slice of regions contains a specific region
func containsRegion(regions []string, region string) bool {
	for _, r := range regions {
		if r == region {
			return true
		}
	}
	return false
}
