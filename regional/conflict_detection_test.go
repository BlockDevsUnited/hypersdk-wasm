package regional

import (
	"testing"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap/zaptest"
)

func TestConflictDetection(t *testing.T) {
	logger := zaptest.NewLogger(t)
	
	// Create a conflict detector with default config
	cd := NewConflictDetector(nil, logger)
	
	// Create test data
	now := time.Now()
	localTime := now
	remoteTime := now.Add(-50 * time.Millisecond) // Remote time is 50ms earlier
	
	localTxID := ids.GenerateTestID()
	remoteTxID := ids.GenerateTestID()
	
	localUpdates := map[string][]byte{
		"key1": []byte("local-value1"),
		"key2": []byte("local-value2"),
		"key3": []byte("local-value3"),
	}
	
	remoteUpdates := map[string][]byte{
		"key2": []byte("remote-value2"),
		"key3": []byte("remote-value3"),
		"key4": []byte("remote-value4"),
	}
	
	// Test conflict detection
	conflicts := cd.DetectConflicts(
		localUpdates, localTime, localTxID,
		remoteUpdates, remoteTime, remoteTxID,
	)
	
	// Should detect conflicts on key2 and key3
	assert.Equal(t, 2, len(conflicts))
	assert.Contains(t, conflicts, "key2")
	assert.Contains(t, conflicts, "key3")
	
	// Verify conflict type
	assert.Equal(t, KeyConflict, conflicts["key2"].Type)
	assert.Equal(t, KeyConflict, conflicts["key3"].Type)
	
	// Test conflict resolution
	resolutions := cd.ResolveConflicts(conflicts)
	
	// Should have resolutions for key2 and key3
	assert.Equal(t, 2, len(resolutions))
	
	// Default config gives priority to cross-region operations
	assert.Equal(t, CrossRegionWins, resolutions["key2"])
	assert.Equal(t, CrossRegionWins, resolutions["key3"])
	
	// Test with local priority
	cd = NewConflictDetector(&ConflictDetectionConfig{
		MaxTimestampDiffMs: 100,
		LocalPriority:      true,
		EnableAuditTrail:   true,
	}, logger)
	
	conflicts = cd.DetectConflicts(
		localUpdates, localTime, localTxID,
		remoteUpdates, remoteTime, remoteTxID,
	)
	
	resolutions = cd.ResolveConflicts(conflicts)
	
	// With local priority, local transactions should win
	assert.Equal(t, LocalWins, resolutions["key2"])
	assert.Equal(t, LocalWins, resolutions["key3"])
	
	// Test timestamp conflicts
	remoteTime = now.Add(-150 * time.Millisecond) // Now remote time is 150ms earlier (> 100ms diff)
	
	cd = NewConflictDetector(nil, logger) // Back to default config
	
	conflicts = cd.DetectConflicts(
		localUpdates, localTime, localTxID,
		remoteUpdates, remoteTime, remoteTxID,
	)
	
	// Should still detect conflicts on key2 and key3, but now they're timestamp conflicts
	assert.Equal(t, 2, len(conflicts))
	assert.Contains(t, conflicts, "key2")
	assert.Contains(t, conflicts, "key3")
	
	// Verify conflict type is now timestamp conflict
	assert.Equal(t, TimestampConflict, conflicts["key2"].Type)
	assert.Equal(t, TimestampConflict, conflicts["key3"].Type)
	
	resolutions = cd.ResolveConflicts(conflicts)
	
	// For timestamp conflicts, newer timestamp wins (which is local in this case)
	assert.Equal(t, LocalWins, resolutions["key2"])
	assert.Equal(t, LocalWins, resolutions["key3"])
	
	// Test audit trail
	auditTrail := cd.GetAuditTrail()
	assert.Equal(t, 2, len(auditTrail))
	
	// Clear audit trail
	cd.ClearAuditTrail()
	assert.Equal(t, 0, len(cd.GetAuditTrail()))
}

func TestNoConflicts(t *testing.T) {
	logger := zaptest.NewLogger(t)
	cd := NewConflictDetector(nil, logger)
	
	now := time.Now()
	
	localUpdates := map[string][]byte{
		"key1": []byte("local-value1"),
		"key2": []byte("local-value2"),
	}
	
	remoteUpdates := map[string][]byte{
		"key3": []byte("remote-value3"),
		"key4": []byte("remote-value4"),
	}
	
	// No key overlaps, so no conflicts
	conflicts := cd.DetectConflicts(
		localUpdates, now, ids.GenerateTestID(),
		remoteUpdates, now, ids.GenerateTestID(),
	)
	
	assert.Equal(t, 0, len(conflicts))
}

func TestRegulatoryConflicts(t *testing.T) {
	logger := zaptest.NewLogger(t)
	cd := NewConflictDetector(nil, logger)
	
	now := time.Now()
	txID1 := ids.GenerateTestID()
	txID2 := ids.GenerateTestID()
	
	// Create a manual regulatory conflict
	conflicts := map[string]ConflictMetadata{
		"regulatory_key": {
			Type:            RegulatoryConflict,
			Key:             "regulatory_key",
			LocalTimestamp:  now,
			RemoteTimestamp: now,
			LocalTxID:       txID1,
			RemoteTxID:      txID2,
		},
	}
	
	resolutions := cd.ResolveConflicts(conflicts)
	assert.Equal(t, 1, len(resolutions))
	assert.Equal(t, Rejected, resolutions["regulatory_key"])
}
