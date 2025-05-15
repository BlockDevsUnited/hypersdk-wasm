package regional

import (
	"testing"
	"time"
	
	"github.com/ava-labs/avalanchego/ids"
	"github.com/stretchr/testify/assert"
)

func TestBatchOperation(t *testing.T) {
	// Create a test BatchOperation
	txID := ids.GenerateTestID()
	payload := []byte("transaction data")
	timestamp := time.Now().UnixNano() / 1000000 // Milliseconds
	primaryRegion := "us-east"
	otherRegions := []string{"us-west", "eu-central"}
	
	op := BatchOperation{
		TxID:          txID,
		PrimaryRegion: primaryRegion,
		OtherRegions:  otherRegions,
		Timestamp:     timestamp,
		Payload:       payload,
	}
	
	// Verify fields
	assert.Equal(t, txID, op.TxID)
	assert.Equal(t, primaryRegion, op.PrimaryRegion)
	assert.Equal(t, otherRegions, op.OtherRegions)
	assert.Equal(t, timestamp, op.Timestamp)
	assert.Equal(t, payload, op.Payload)
	
	// Test validation when fields are missing
	invalidOps := []BatchOperation{
		{
			// Missing TxID
			PrimaryRegion: primaryRegion,
			OtherRegions:  otherRegions,
			Timestamp:     timestamp,
			Payload:       payload,
		},
		{
			TxID: txID,
			// Missing PrimaryRegion
			OtherRegions:  otherRegions,
			Timestamp:     timestamp,
			Payload:       payload,
		},
		{
			TxID:          txID,
			PrimaryRegion: primaryRegion,
			OtherRegions:  otherRegions,
			// Missing Timestamp
			Payload:       payload,
		},
		{
			TxID:          txID,
			PrimaryRegion: primaryRegion,
			OtherRegions:  otherRegions,
			Timestamp:     timestamp,
			// Missing Payload
		},
	}
	
	// Check validation for each invalid operation
	for i, invalidOp := range invalidOps {
		valid := isValidBatchOperation(invalidOp)
		assert.False(t, valid, "Operation %d missing required field should be invalid", i)
	}
	
	// Test valid operation
	valid := isValidBatchOperation(op)
	assert.True(t, valid, "Complete operation should be valid")
}

func TestBatchResult(t *testing.T) {
	// Create a test BatchResult
	result := BatchResult{
		Success: true,
		Error:   "",
		Region:  "us-east",
		Metrics: map[string]interface{}{"latency_ms": 42},
	}
	
	// Verify fields
	assert.True(t, result.Success)
	assert.Empty(t, result.Error)
	assert.Equal(t, "us-east", result.Region)
	assert.Equal(t, 42, result.Metrics["latency_ms"])
	
	// Test failure result
	failureResult := BatchResult{
		Success: false,
		Error:   "transaction failed due to invalid attestation",
		Region:  "us-west",
		Metrics: map[string]interface{}{"error_count": 1},
	}
	
	assert.False(t, failureResult.Success)
	assert.Equal(t, "transaction failed due to invalid attestation", failureResult.Error)
	assert.Equal(t, "us-west", failureResult.Region)
	assert.Equal(t, 1, failureResult.Metrics["error_count"])
}

// Helper function to validate a BatchOperation
// This function would typically be part of the actual implementation,
// but we're adding it here for testing purposes
func isValidBatchOperation(op BatchOperation) bool {
	if op.TxID == ids.Empty || len(op.PrimaryRegion) == 0 || op.Timestamp == 0 || len(op.Payload) == 0 {
		return false
	}
	
	return true
}
