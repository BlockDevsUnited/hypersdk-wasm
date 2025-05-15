package regional

import (
	"context"
	"testing"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap/zaptest"
)

func TestProcessCrossRegionTransaction(t *testing.T) {
	logger := zaptest.NewLogger(t)
	coordinator := NewCrossRegionCoordinator(logger, nil)

	// Test data
	sourceRegion := "region-A"
	targetRegions := []string{"region-B", "region-C"}
	txID := ids.GenerateTestID()
	timestamp := time.Now()

	// State updates that will be sent across regions
	stateUpdates := map[string][]byte{
		"key1": []byte("value1"),
		"key2": []byte("value2"),
	}

	// Process a cross-region transaction
	err := coordinator.ProcessCrossRegionTransaction(
		context.Background(),
		sourceRegion,
		targetRegions,
		stateUpdates,
		txID,
		timestamp,
	)

	// Should succeed without conflicts
	assert.NoError(t, err)
	
	// Check that the operation was queued
	coordinator.lock.RLock()
	pendingOpsCount := len(coordinator.pendingOperations)
	hasTxID := false
	var op *CrossRegionOperation
	if opFound, exists := coordinator.pendingOperations[txID]; exists {
		hasTxID = true
		op = opFound
	}
	coordinator.lock.RUnlock()
	
	assert.Equal(t, 1, pendingOpsCount)
	assert.True(t, hasTxID)
	
	// Verify the queued operation has correct data
	if assert.NotNil(t, op) {
		assert.Equal(t, sourceRegion, op.SourceRegion)
		assert.Equal(t, targetRegions, op.TargetRegions)
		assert.Equal(t, TransactionSync, op.OperationType)
		assert.Equal(t, stateUpdates, op.StateUpdates)
		assert.Equal(t, txID, op.TxIDs[0])
	}
}

func TestCrossRegionTransactionWithConflicts(t *testing.T) {
	logger := zaptest.NewLogger(t)
	coordinator := NewCrossRegionCoordinator(logger, nil)
	
	// Set up initial state for a target region
	targetRegion := "region-B"
	
	coordinator.lock.Lock()
	coordinator.regionStates[targetRegion] = &RegionalStateVersion{
		Height:    1,
		StateRoot: []byte("test-root"),
		Changes:   make(map[string][]byte),
		Timestamp: time.Now().Add(-1 * time.Second),
	}
	
	// Add a pending operation for the target region with conflicting keys
	pendingTxID := ids.GenerateTestID()
	pendingOp := &CrossRegionOperation{
		SourceRegion:  "region-C",
		TargetRegions: []string{targetRegion},
		OperationType: TransactionSync,
		StateUpdates: map[string][]byte{
			"key1": []byte("pending-value1"), // Will conflict
			"key3": []byte("pending-value3"),
		},
		Timestamp: time.Now().Add(-500 * time.Millisecond),
		TxIDs:     []ids.ID{pendingTxID},
	}
	coordinator.pendingOperations[pendingTxID] = pendingOp
	coordinator.lock.Unlock()
	
	// Create a new cross-region transaction with conflict
	sourceRegion := "region-A"
	txID := ids.GenerateTestID()
	timestamp := time.Now()
	stateUpdates := map[string][]byte{
		"key1": []byte("new-value1"), // Conflicts with pending operation
		"key2": []byte("new-value2"),
	}
	
	// Process transaction with conflicting key
	err := coordinator.ProcessCrossRegionTransaction(
		context.Background(),
		sourceRegion,
		[]string{targetRegion},
		stateUpdates,
		txID,
		timestamp,
	)
	
	// Should succeed despite conflicts (because cross-region wins by default)
	assert.NoError(t, err)
	
	// Verify the operation was queued with both keys intact
	coordinator.lock.RLock()
	op, exists := coordinator.pendingOperations[txID]
	coordinator.lock.RUnlock()
	
	assert.True(t, exists)
	assert.Contains(t, op.StateUpdates, "key1")
	assert.Contains(t, op.StateUpdates, "key2")
	
	// Change conflict detection config to prefer local operations
	coordinator = NewCrossRegionCoordinator(logger, nil)
	
	coordinator.lock.Lock()
	// Update conflict config to prefer local operations
	coordinator.config.ConflictDetection.LocalPriority = true
	
	// Set up state again
	coordinator.regionStates[targetRegion] = &RegionalStateVersion{
		Height:    1,
		StateRoot: []byte("test-root"),
		Changes:   make(map[string][]byte),
		Timestamp: time.Now().Add(-1 * time.Second),
	}
	coordinator.pendingOperations[pendingTxID] = pendingOp
	coordinator.lock.Unlock()
	
	// Process the transaction again
	err = coordinator.ProcessCrossRegionTransaction(
		context.Background(),
		sourceRegion,
		[]string{targetRegion},
		stateUpdates,
		txID,
		timestamp,
	)
	
	// Should succeed but with key1 removed from the operation
	assert.NoError(t, err)
	
	// Verify key1 was removed from state updates
	coordinator.lock.RLock()
	op, exists = coordinator.pendingOperations[txID] 
	coordinator.lock.RUnlock()
	
	assert.True(t, exists)
	assert.NotContains(t, op.StateUpdates, "key1") // Removed due to local priority
	assert.Contains(t, op.StateUpdates, "key2")    // Still present
}
