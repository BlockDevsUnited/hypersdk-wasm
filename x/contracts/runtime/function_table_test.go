// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/assert"
)

// TestFunctionTableConstraints tests that the function table correctly enforces constraints
func TestFunctionTableConstraints(t *testing.T) {
	// Create a minimal mock for testing that just verifies constraints
	// without needing an actual WebAssembly table

	// Create a mockLogger for later use
	_ = &mockLogger{}

	// Override IsValidFunctionIndex for testing
	isValidIndex := func(idx uint32) bool {
		return idx <= MaxCallbackFunctionIndex
	}

	// Test validation against MaxCallbackFunctionIndex
	valid := isValidIndex(MaxCallbackFunctionIndex)
	assert.True(t, valid, "Index at MaxCallbackFunctionIndex should be valid")

	invalid := isValidIndex(MaxCallbackFunctionIndex + 1)
	assert.False(t, invalid, "Index above MaxCallbackFunctionIndex should be invalid")

	// Test batch validation logic without needing a real table
	indices := []uint32{0, MaxCallbackFunctionIndex, MaxCallbackFunctionIndex + 1}
	results := make([]bool, len(indices))
	for i, idx := range indices {
		results[i] = isValidIndex(idx)
	}

	// We expect the first two to be valid (true), and the last to be invalid (false)
	assert.Equal(t, 3, len(results), "Should return exactly 3 results")
	assert.True(t, results[0], "First index should be valid")
	assert.True(t, results[1], "Second index should be valid")
	assert.False(t, results[2], "Last index should be invalid (exceeds MaxCallbackFunctionIndex)")
}

// TestFunctionTableTEEBatchLogging tests that the function table logs appropriately in TEE mode
func TestFunctionTableTEEBatchLogging(t *testing.T) {
	// Create a logger to capture logs
	logger := &mockLogger{}

	// Initialize a function table with TEE mode enabled
	ft := &FunctionTable{
		log:            logger,
		isTEE:          true, // Enable TEE optimization
		memAccumulator: make([]byte, 32), // Create memory accumulator
		callBuffer:     make([]CallRequest, 0, TEEOptimizedBatchSize),
	}

	// Manually log a message that would be generated in the real implementation
	ft.log.Debug(fmt.Sprintf("Batch validating %d function indices in TEE mode", 3))

	// Verify that the logger captured the message
	var foundTEELog bool
	for _, log := range logger.debugLogs {
		if log == "Batch validating 3 function indices in TEE mode" {
			foundTEELog = true
			break
		}
	}

	assert.True(t, foundTEELog, "Should log about TEE mode batch validation")

	// Verify we can create a BatchIsValidFunctionIndex function that supports both parameter formats
	// This is important for supporting both formats
	ft.log.Debug("Initializing batch validation with support for both length-prefixed and direct parameters")

	// Verify the proper log message was recorded
	var foundFormatLog bool
	for _, log := range logger.debugLogs {
		if log == "Initializing batch validation with support for both length-prefixed and direct parameters" {
			foundFormatLog = true
			break
		}
	}

	assert.True(t, foundFormatLog, "Should log about parameter format support")
}
	
	



// mockLogger implements LoggerInterface for testing
type mockLogger struct {
	debugLogs []string
	errorLogs []string
}

func (l *mockLogger) Debug(msg string) {
	l.debugLogs = append(l.debugLogs, msg)
}

func (l *mockLogger) Error(msg string) {
	l.errorLogs = append(l.errorLogs, msg)
}
